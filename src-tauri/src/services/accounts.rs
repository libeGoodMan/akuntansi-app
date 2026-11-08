use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

use super::audit;
use crate::error::{AppError, AppResult};

const TYPES: [&str; 5] = ["ASSET", "LIABILITY", "EQUITY", "REVENUE", "EXPENSE"];

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i64,
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub account_type: String,
    pub normal_balance: String,
    pub parent_id: Option<i64>,
    pub is_header: bool,
    pub is_active: bool,
    pub is_system: bool,
    pub system_role: Option<String>,
    /// Akun Kas/Bank: dihitung pada kartu "Kas & Bank" di Dashboard.
    pub is_cash: bool,
    /// Sudah dipakai di baris jurnal (tidak bisa dihapus, hanya dinonaktifkan).
    pub in_use: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    /// Kosong = ikuti jenis akun. Isi manual untuk akun kontra (mis. Akumulasi Penyusutan).
    pub normal_balance: Option<String>,
    pub parent_id: Option<i64>,
    pub is_header: bool,
    /// Tandai sebagai akun kas/bank (hanya akun aset yang bukan header).
    #[serde(default)]
    pub is_cash: bool,
}

fn invalid<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Validation(msg.into()))
}

pub fn default_normal_balance(account_type: &str) -> &'static str {
    match account_type {
        "ASSET" | "EXPENSE" => "DEBIT",
        _ => "CREDIT",
    }
}

/// SELECT akun + penanda `in_use`; `$tail` ditambahkan saat compile (tetap `&'static str`).
macro_rules! select_account {
    ($tail:literal) => {
        concat!(
            "SELECT a.id, a.code, a.name, a.type, a.normal_balance, a.parent_id,
             a.is_header, a.is_active, a.is_system, a.system_role, a.is_cash,
             EXISTS(SELECT 1 FROM journal_lines l WHERE l.account_id = a.id) AS in_use
             FROM accounts a ",
            $tail
        )
    };
}

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Account>> {
    Ok(sqlx::query_as(select_account!("ORDER BY a.code"))
        .fetch_all(pool)
        .await?)
}

async fn fetch(conn: &mut SqliteConnection, id: i64) -> AppResult<Account> {
    sqlx::query_as(select_account!("WHERE a.id = ?"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| AppError::Validation("Akun tidak ditemukan".into()))
}

async fn child_count(conn: &mut SqliteConnection, id: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE parent_id = ?")
        .bind(id)
        .fetch_one(conn)
        .await?)
}

struct Clean {
    code: String,
    name: String,
    account_type: String,
    normal_balance: String,
    parent_id: Option<i64>,
    is_header: bool,
    is_cash: bool,
}

/// Validasi bentuk input dan hubungan dengan akun induk. `self_id` diisi saat edit.
async fn validate(
    conn: &mut SqliteConnection,
    input: &AccountInput,
    self_id: Option<i64>,
) -> AppResult<Clean> {
    let code = input.code.trim().to_string();
    let name = input.name.trim().to_string();
    if code.is_empty() || code.chars().count() > 20 {
        return invalid("Kode akun wajib diisi (maksimal 20 karakter)");
    }
    if name.is_empty() || name.chars().count() > 100 {
        return invalid("Nama akun wajib diisi (maksimal 100 karakter)");
    }
    if !TYPES.contains(&input.account_type.as_str()) {
        return invalid("Jenis akun tidak dikenal");
    }
    let normal_balance = match input.normal_balance.as_deref() {
        None | Some("") => default_normal_balance(&input.account_type).to_string(),
        Some(nb @ ("DEBIT" | "CREDIT")) => nb.to_string(),
        Some(_) => return invalid("Saldo normal harus DEBIT atau CREDIT"),
    };

    let dup: Option<i64> = sqlx::query_scalar("SELECT id FROM accounts WHERE code = ?")
        .bind(&code)
        .fetch_optional(&mut *conn)
        .await?;
    if dup.is_some_and(|d| Some(d) != self_id) {
        return invalid(format!("Kode akun {code} sudah dipakai"));
    }

    if let Some(pid) = input.parent_id {
        if Some(pid) == self_id {
            return invalid("Akun tidak bisa menjadi induk dirinya sendiri");
        }
        let parent = fetch(&mut *conn, pid).await?;
        if !parent.is_header {
            return invalid("Akun induk harus berupa akun header");
        }
        if parent.account_type != input.account_type {
            return invalid("Jenis akun harus sama dengan jenis akun induknya");
        }
        // Cegah siklus: induk baru tidak boleh keturunan dari akun ini.
        if let Some(me) = self_id {
            let mut cursor = parent.parent_id;
            while let Some(c) = cursor {
                if c == me {
                    return invalid("Induk yang dipilih adalah turunan dari akun ini");
                }
                cursor = sqlx::query_scalar("SELECT parent_id FROM accounts WHERE id = ?")
                    .bind(c)
                    .fetch_one(&mut *conn)
                    .await?;
            }
        }
    }

    if input.is_cash && (input.is_header || input.account_type != "ASSET") {
        return invalid("Akun kas/bank harus berupa akun aset yang bukan header");
    }

    Ok(Clean {
        code,
        name,
        account_type: input.account_type.clone(),
        normal_balance,
        parent_id: input.parent_id,
        is_header: input.is_header,
        is_cash: input.is_cash,
    })
}

pub async fn create(pool: &SqlitePool, input: AccountInput) -> AppResult<Account> {
    let mut tx = pool.begin().await?;
    let c = validate(&mut tx, &input, None).await?;
    let id = sqlx::query(
        "INSERT INTO accounts (code, name, type, normal_balance, parent_id, is_header, is_cash)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&c.code)
    .bind(&c.name)
    .bind(&c.account_type)
    .bind(&c.normal_balance)
    .bind(c.parent_id)
    .bind(c.is_header)
    .bind(c.is_cash)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    let account = fetch(&mut tx, id).await?;
    audit::log(&mut tx, "CREATE", "account", Some(id), json!({ "after": account })).await?;
    tx.commit().await?;
    Ok(account)
}

pub async fn update(pool: &SqlitePool, id: i64, input: AccountInput) -> AppResult<Account> {
    let mut tx = pool.begin().await?;
    let before = fetch(&mut tx, id).await?;
    let c = validate(&mut tx, &input, Some(id)).await?;

    let type_changed = c.account_type != before.account_type;
    let header_changed = c.is_header != before.is_header;
    let parent_changed = c.parent_id != before.parent_id;

    if before.is_system && (type_changed || header_changed || parent_changed) {
        return invalid("Akun sistem hanya boleh diubah kode dan namanya");
    }
    if before.in_use && (type_changed || (header_changed && c.is_header)) {
        return invalid("Akun sudah dipakai di jurnal: jenisnya tidak bisa diubah dan tidak bisa dijadikan header");
    }
    if child_count(&mut tx, id).await? > 0 && (type_changed || (header_changed && !c.is_header)) {
        return invalid("Akun masih punya akun anak: jenisnya tidak bisa diubah dan tidak bisa dijadikan akun biasa");
    }

    sqlx::query(
        "UPDATE accounts SET code = ?, name = ?, type = ?, normal_balance = ?, parent_id = ?, is_header = ?, is_cash = ?
         WHERE id = ?",
    )
    .bind(&c.code)
    .bind(&c.name)
    .bind(&c.account_type)
    .bind(&c.normal_balance)
    .bind(c.parent_id)
    .bind(c.is_header)
    .bind(c.is_cash)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    let after = fetch(&mut tx, id).await?;
    audit::log(&mut tx, "UPDATE", "account", Some(id), json!({ "before": before, "after": after }))
        .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn set_active(pool: &SqlitePool, id: i64, active: bool) -> AppResult<Account> {
    let mut tx = pool.begin().await?;
    let before = fetch(&mut tx, id).await?;

    if !active {
        if before.is_system {
            return invalid("Akun sistem tidak bisa dinonaktifkan");
        }
        let active_children: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE parent_id = ? AND is_active = 1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if active_children > 0 {
            return invalid("Nonaktifkan dulu semua akun anaknya");
        }
    } else if let Some(pid) = before.parent_id {
        let parent = fetch(&mut tx, pid).await?;
        if !parent.is_active {
            return invalid("Aktifkan dulu akun induknya");
        }
    }

    sqlx::query("UPDATE accounts SET is_active = ? WHERE id = ?")
        .bind(active)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let after = fetch(&mut tx, id).await?;
    audit::log(
        &mut tx,
        if active { "ACTIVATE" } else { "DEACTIVATE" },
        "account",
        Some(id),
        json!({ "code": after.code, "name": after.name }),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let before = fetch(&mut tx, id).await?;
    if before.is_system {
        return invalid("Akun sistem tidak bisa dihapus");
    }
    if before.in_use {
        return invalid("Akun sudah dipakai di jurnal dan tidak bisa dihapus. Nonaktifkan saja");
    }
    if child_count(&mut tx, id).await? > 0 {
        return invalid("Akun masih punya akun anak");
    }
    sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit::log(&mut tx, "DELETE", "account", Some(id), json!({ "before": before })).await?;
    tx.commit().await?;
    Ok(())
}
