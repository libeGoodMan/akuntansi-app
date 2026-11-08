/// Satu baris template akun. `parent` = kode akun induk (harus muncul lebih dulu di daftar).
pub struct TemplateAccount {
    pub code: &'static str,
    pub name: &'static str,
    pub account_type: &'static str,
    pub parent: Option<&'static str>,
    pub is_header: bool,
    /// Isi hanya untuk akun kontra (saldo normal berlawanan dengan jenisnya).
    pub normal_balance: Option<&'static str>,
    pub system_role: Option<&'static str>,
    /// Akun kas/bank untuk Dashboard.
    pub is_cash: bool,
}

const fn h(
    code: &'static str,
    name: &'static str,
    t: &'static str,
    parent: Option<&'static str>,
) -> TemplateAccount {
    TemplateAccount { code, name, account_type: t, parent, is_header: true, normal_balance: None, system_role: None, is_cash: false }
}

const fn a(
    code: &'static str,
    name: &'static str,
    t: &'static str,
    parent: &'static str,
) -> TemplateAccount {
    TemplateAccount { code, name, account_type: t, parent: Some(parent), is_header: false, normal_balance: None, system_role: None, is_cash: false }
}

/// Akun kontra: saldo normal dibalik dari jenisnya.
const fn contra(
    code: &'static str,
    name: &'static str,
    t: &'static str,
    parent: &'static str,
    normal: &'static str,
) -> TemplateAccount {
    TemplateAccount { code, name, account_type: t, parent: Some(parent), is_header: false, normal_balance: Some(normal), system_role: None, is_cash: false }
}

/// Akun kas/bank (ditandai agar muncul di kartu Kas & Bank).
const fn cash(code: &'static str, name: &'static str, parent: &'static str) -> TemplateAccount {
    TemplateAccount { code, name, account_type: "ASSET", parent: Some(parent), is_header: false, normal_balance: None, system_role: None, is_cash: true }
}

const fn sys(
    code: &'static str,
    name: &'static str,
    parent: &'static str,
    role: &'static str,
) -> TemplateAccount {
    TemplateAccount { code, name, account_type: "EQUITY", parent: Some(parent), is_header: false, normal_balance: None, system_role: Some(role), is_cash: false }
}

pub fn get(template: &str) -> Option<Vec<TemplateAccount>> {
    match template {
        "standard" => Some(standard()),
        "empty" => Some(minimal()),
        _ => None,
    }
}

/// Hanya akun sistem yang wajib ada agar saldo awal dan tutup buku bisa berjalan.
fn minimal() -> Vec<TemplateAccount> {
    vec![
        h("3-0000", "EKUITAS", "EQUITY", None),
        sys("3-1000", "Modal Pemilik", "3-0000", "OPENING_EQUITY"),
        sys("3-3000", "Laba Ditahan", "3-0000", "RETAINED_EARNINGS"),
    ]
}

/// COA standar perusahaan dagang/jasa kecil-menengah di Indonesia.
fn standard() -> Vec<TemplateAccount> {
    vec![
        h("1-0000", "ASET", "ASSET", None),
        h("1-1000", "Aset Lancar", "ASSET", Some("1-0000")),
        cash("1-1100", "Kas", "1-1000"),
        cash("1-1200", "Bank", "1-1000"),
        a("1-1300", "Piutang Usaha", "ASSET", "1-1000"),
        a("1-1400", "Persediaan Barang Dagang", "ASSET", "1-1000"),
        a("1-1500", "Perlengkapan", "ASSET", "1-1000"),
        a("1-1600", "Biaya Dibayar Dimuka", "ASSET", "1-1000"),
        a("1-1700", "PPN Masukan", "ASSET", "1-1000"),
        h("1-2000", "Aset Tetap", "ASSET", Some("1-0000")),
        a("1-2100", "Tanah", "ASSET", "1-2000"),
        a("1-2200", "Bangunan", "ASSET", "1-2000"),
        contra("1-2210", "Akumulasi Penyusutan Bangunan", "ASSET", "1-2000", "CREDIT"),
        a("1-2300", "Kendaraan", "ASSET", "1-2000"),
        contra("1-2310", "Akumulasi Penyusutan Kendaraan", "ASSET", "1-2000", "CREDIT"),
        a("1-2400", "Peralatan", "ASSET", "1-2000"),
        contra("1-2410", "Akumulasi Penyusutan Peralatan", "ASSET", "1-2000", "CREDIT"),
        h("2-0000", "KEWAJIBAN", "LIABILITY", None),
        h("2-1000", "Kewajiban Lancar", "LIABILITY", Some("2-0000")),
        a("2-1100", "Utang Usaha", "LIABILITY", "2-1000"),
        a("2-1200", "Utang Gaji", "LIABILITY", "2-1000"),
        a("2-1300", "Utang Pajak", "LIABILITY", "2-1000"),
        a("2-1400", "PPN Keluaran", "LIABILITY", "2-1000"),
        a("2-1500", "Pendapatan Diterima Dimuka", "LIABILITY", "2-1000"),
        h("2-2000", "Kewajiban Jangka Panjang", "LIABILITY", Some("2-0000")),
        a("2-2100", "Utang Bank", "LIABILITY", "2-2000"),
        h("3-0000", "EKUITAS", "EQUITY", None),
        sys("3-1000", "Modal Pemilik", "3-0000", "OPENING_EQUITY"),
        contra("3-2000", "Prive", "EQUITY", "3-0000", "DEBIT"),
        sys("3-3000", "Laba Ditahan", "3-0000", "RETAINED_EARNINGS"),
        h("4-0000", "PENDAPATAN", "REVENUE", None),
        a("4-1000", "Penjualan", "REVENUE", "4-0000"),
        contra("4-1100", "Retur dan Potongan Penjualan", "REVENUE", "4-0000", "DEBIT"),
        a("4-2000", "Pendapatan Jasa", "REVENUE", "4-0000"),
        a("4-9000", "Pendapatan Lain-lain", "REVENUE", "4-0000"),
        h("5-0000", "HARGA POKOK PENJUALAN", "EXPENSE", None),
        a("5-1000", "Harga Pokok Penjualan", "EXPENSE", "5-0000"),
        a("5-1100", "Pembelian", "EXPENSE", "5-0000"),
        a("5-1200", "Beban Angkut Pembelian", "EXPENSE", "5-0000"),
        h("6-0000", "BEBAN OPERASIONAL", "EXPENSE", None),
        a("6-1000", "Beban Gaji", "EXPENSE", "6-0000"),
        a("6-1100", "Beban Sewa", "EXPENSE", "6-0000"),
        a("6-1200", "Beban Listrik, Air, dan Telepon", "EXPENSE", "6-0000"),
        a("6-1300", "Beban Perlengkapan", "EXPENSE", "6-0000"),
        a("6-1400", "Beban Penyusutan", "EXPENSE", "6-0000"),
        a("6-1500", "Beban Pemasaran", "EXPENSE", "6-0000"),
        a("6-1600", "Beban Administrasi dan Umum", "EXPENSE", "6-0000"),
        a("6-9100", "Beban Bunga", "EXPENSE", "6-0000"),
        a("6-9200", "Beban Pajak", "EXPENSE", "6-0000"),
        a("6-9900", "Beban Lain-lain", "EXPENSE", "6-0000"),
    ]
}
