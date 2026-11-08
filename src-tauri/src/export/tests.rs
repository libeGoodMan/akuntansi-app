use serde_json::{json, Value};

use super::pdf::{self, Template};
use super::{load, pdf_data, xlsx, Format, ReportRequest};
use crate::services::accounts;
use crate::services::journal::{self, JournalInput, LineInput};
use crate::services::tests_statements::sample;

fn req(v: Value) -> ReportRequest {
    serde_json::from_value(v).expect("permintaan valid")
}

/// Jika AKUNTANSI_PNG_DIR diisi, simpan tiap halaman sebagai PNG untuk dilihat manual.
fn dump_png(name: &str, template: Template, data: &Value) {
    let Some(dir) = std::env::var_os("AKUNTANSI_PNG_DIR") else { return };
    let doc = pdf::compile(template, data).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    for (i, page) in doc.pages().iter().enumerate() {
        let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
        let path = std::path::Path::new(&dir).join(format!("{name}-{}.png", i + 1));
        std::fs::write(path, pixmap.encode_png().unwrap()).unwrap();
    }
}

async fn report(pool: &sqlx::SqlitePool, v: Value) -> super::Report {
    load(pool, req(v)).await.unwrap()
}

async fn acc_id(pool: &sqlx::SqlitePool, code: &str) -> i64 {
    accounts::list(pool).await.unwrap().into_iter().find(|a| a.code == code).unwrap().id
}

fn assert_pdf(bytes: &[u8]) {
    assert!(bytes.starts_with(b"%PDF-"), "bukan PDF");
    assert!(bytes.len() > 2_000, "PDF terlalu kecil: {} byte", bytes.len());
}

#[tokio::test]
async fn setiap_template_ter_compile_dengan_data_contoh() {
    let pool = sample().await;
    sqlx::query("UPDATE company SET address = 'Jl. Merdeka No. 1, Jakarta Pusat 10110' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let kas = acc_id(&pool, "1-1100").await;
    let first_journal = journal::list(&pool, Default::default()).await.unwrap().items[0].id;

    let cases = [
        ("neraca-saldo", json!({ "kind": "trial-balance", "asOf": "2026-11-30" })),
        ("laba-rugi", json!({ "kind": "income-statement", "from": "2026-11-01", "to": "2026-11-30" })),
        (
            "laba-rugi-banding",
            json!({ "kind": "income-statement", "from": "2026-11-16", "to": "2026-11-30", "compareFrom": "2026-11-01", "compareTo": "2026-11-15" }),
        ),
        ("neraca", json!({ "kind": "balance-sheet", "asOf": "2026-11-30" })),
        ("buku-besar", json!({ "kind": "ledger", "accountId": kas, "from": "2026-01-01", "to": "2026-12-31" })),
        ("bukti-jurnal", json!({ "kind": "voucher", "id": first_journal })),
    ];
    for (name, request) in cases {
        let r = report(&pool, request).await;
        let (template, data) = pdf_data(&r);
        let bytes = pdf::render(template, &data).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_pdf(&bytes);
        dump_png(name, template, &data);
    }
}

#[tokio::test]
async fn data_pdf_memuat_angka_terformat_dan_kop() {
    let pool = sample().await;
    let r = report(&pool, json!({ "kind": "income-statement", "from": "2026-11-01", "to": "2026-11-30" })).await;
    let (template, d) = pdf_data(&r);
    assert_eq!(template, Template::IncomeStatement);
    assert_eq!(d["company"]["name"], "PT Dagang Maju");
    assert!(d["company"]["address"].is_null());
    assert_eq!(d["title"], "Laporan Laba Rugi");
    assert_eq!(d["period"], "1 November 2026 s.d. 30 November 2026");
    let lines = d["lines"].as_array().unwrap();
    let net = lines.iter().find(|l| l["label"] == "Laba (Rugi) Bersih").unwrap();
    assert_eq!(net["amount"], "40.500.000,00");
    assert_eq!(net["kind"], "net");
    let retur = lines.iter().find(|l| l["label"] == "Retur dan Potongan Penjualan").unwrap();
    assert_eq!(retur["amount"], "-2.000.000,00");

    let r = report(&pool, json!({ "kind": "balance-sheet", "asOf": "2026-11-30" })).await;
    let (_, d) = pdf_data(&r);
    assert_eq!(d["balanced"], true);
    assert!(d["note"].as_str().unwrap().starts_with("Seimbang"));

    let tb = report(&pool, json!({ "kind": "trial-balance" })).await;
    let (_, d) = pdf_data(&tb);
    assert_eq!(d["period"], "Seluruh periode");
    assert_eq!(d["total_debit"], d["total_credit"]);
}

#[tokio::test]
async fn pdf_laporan_kosong_dan_buku_besar_panjang() {
    let pool = sample().await;
    // Laporan tanpa isi tetap bisa dicetak.
    let empty = report(&pool, json!({ "kind": "trial-balance", "asOf": "2026-01-01" })).await;
    let (t, d) = pdf_data(&empty);
    assert_pdf(&pdf::render(t, &d).unwrap());
    let empty_is = report(&pool, json!({ "kind": "income-statement", "from": "2025-01-01", "to": "2025-01-31" })).await;
    let (t, d) = pdf_data(&empty_is);
    assert_pdf(&pdf::render(t, &d).unwrap());

    // Buku besar ratusan baris -> beberapa halaman, semua baris ikut (tanpa paginasi).
    let kas = acc_id(&pool, "1-1100").await;
    let penjualan = acc_id(&pool, "4-1000").await;
    let before = report(&pool, json!({ "kind": "ledger", "accountId": kas })).await;
    let existing = match &before.data {
        super::ReportData::Ledger(l) => l.rows.len(),
        _ => unreachable!(),
    };
    for i in 1..=400 {
        let input = JournalInput {
            date: format!("2026-12-{:02}", 1 + i % 28),
            description: format!("Penjualan harian nomor {i} dengan keterangan yang cukup panjang untuk menguji pembungkusan baris"),
            reference: None,
            lines: vec![
                LineInput { account_id: kas, debit: i * 100, credit: 0, memo: Some("catatan baris".into()) },
                LineInput { account_id: penjualan, debit: 0, credit: i * 100, memo: None },
            ],
        };
        journal::create(&pool, input, true).await.unwrap();
    }
    let big = report(&pool, json!({ "kind": "ledger", "accountId": kas })).await;
    let (t, d) = pdf_data(&big);
    assert_eq!(d["rows"].as_array().unwrap().len(), existing + 400);
    let doc = pdf::compile(t, &d).unwrap();
    assert!(doc.pages().len() >= 8, "diharapkan banyak halaman, dapat {}", doc.pages().len());
    assert_pdf(&pdf::render(t, &d).unwrap());
    dump_png("buku-besar-panjang", t, &d);
}

#[tokio::test]
async fn excel_dibuat_untuk_semua_laporan_kecuali_bukti_jurnal() {
    let pool = sample().await;
    let kas = acc_id(&pool, "1-1100").await;
    for request in [
        json!({ "kind": "trial-balance" }),
        json!({ "kind": "income-statement", "from": "2026-11-01", "to": "2026-11-30", "compareFrom": "2026-10-01", "compareTo": "2026-10-31" }),
        json!({ "kind": "balance-sheet", "asOf": "2026-11-30" }),
        json!({ "kind": "ledger", "accountId": kas }),
    ] {
        let r = report(&pool, request).await;
        let bytes = xlsx::write(&r).unwrap();
        assert!(bytes.starts_with(b"PK"), "XLSX adalah arsip zip");
        assert!(bytes.len() > 3_000);
    }
    let first_journal = journal::list(&pool, Default::default()).await.unwrap().items[0].id;
    let v = report(&pool, json!({ "kind": "voucher", "id": first_journal })).await;
    assert!(xlsx::write(&v).is_err());
}

#[tokio::test]
async fn build_menghasilkan_nama_berkas_dan_bytes() {
    let pool = sample().await;
    let (bytes, name) =
        super::build(&pool, req(json!({ "kind": "balance-sheet", "asOf": "2026-11-30" })), Format::Pdf).await.unwrap();
    assert_eq!(name, "neraca-2026-11-30.pdf");
    assert_pdf(&bytes);

    let (bytes, name) = super::build(
        &pool,
        req(json!({ "kind": "income-statement", "from": "2026-11-01", "to": "2026-11-30" })),
        Format::Xlsx,
    )
    .await
    .unwrap();
    assert_eq!(name, "laba-rugi-2026-11-01_2026-11-30.xlsx");
    assert!(bytes.starts_with(b"PK"));

    let first_journal = journal::list(&pool, Default::default()).await.unwrap().items[0].id;
    assert!(super::build(&pool, req(json!({ "kind": "voucher", "id": first_journal })), Format::Xlsx).await.is_err());
}

#[test]
fn permintaan_tidak_dikenal_ditolak() {
    assert!(serde_json::from_value::<ReportRequest>(json!({ "kind": "ngawur" })).is_err());
    assert!(serde_json::from_value::<ReportRequest>(json!({ "foo": 1 })).is_err());
}
