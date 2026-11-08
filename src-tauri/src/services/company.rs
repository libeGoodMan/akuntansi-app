use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

use super::{audit, coa_templates};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    pub name: String,
    pub currency: String,
    pub fiscal_year_start_month: i64,
    /// Alamat untuk kop laporan.
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInput {
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInput {
    pub name: String,
    pub fiscal_year_start_month: i64,
    /// "standard" (COA lengkap) atau "empty" (hanya akun sistem).
    pub template: String,
}

pub async fn get(pool: &SqlitePool) -> AppResult<Option<Company>> {
    Ok(
        sqlx::query_as("SELECT name, currency, fiscal_year_start_month, address FROM company WHERE id = 1")
            .fetch_optional(pool)
            .await?,
    )
}

/// Setup awal: simpan profil perusahaan dan isi COA dari template, dalam satu transaksi.
pub async fn setup(pool: &SqlitePool, input: SetupInput) -> AppResult<Company> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::Validation("Nama perusahaan wajib diisi (maksimal 120 karakter)".into()));
    }
    if !(1..=12).contains(&input.fiscal_year_start_month) {
        return Err(AppError::Validation("Bulan awal tahun fiskal harus 1 sampai 12".into()));
    }
    let template = coa_templates::get(&input.template)
        .ok_or_else(|| AppError::Validation("Template COA tidak dikenal".into()))?;

    let mut tx = pool.begin().await?;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM company")
        .fetch_one(&mut *tx)
        .await?;
    if exists > 0 {
        return Err(AppError::Validation("Perusahaan sudah disetup".into()));
    }

    sqlx::query("INSERT INTO company (id, name, fiscal_year_start_month) VALUES (1, ?, ?)")
        .bind(&name)
        .bind(input.fiscal_year_start_month)
        .execute(&mut *tx)
        .await?;

    let mut ids: HashMap<&str, i64> = HashMap::new();
    for acc in &template {
        let parent_id = match acc.parent {
            Some(code) => Some(*ids.get(code).ok_or_else(|| {
                AppError::Other(format!("Template rusak: induk {code} tidak ditemukan"))
            })?),
            None => None,
        };
        let normal = acc
            .normal_balance
            .unwrap_or_else(|| super::accounts::default_normal_balance(acc.account_type));
        let id = sqlx::query(
            "INSERT INTO accounts (code, name, type, normal_balance, parent_id, is_header, is_system, system_role, is_cash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(acc.code)
        .bind(acc.name)
        .bind(acc.account_type)
        .bind(normal)
        .bind(parent_id)
        .bind(acc.is_header)
        .bind(acc.system_role.is_some())
        .bind(acc.system_role)
        .bind(acc.is_cash)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
        ids.insert(acc.code, id);
    }

    audit::log(
        &mut tx,
        "SETUP",
        "company",
        Some(1),
        json!({ "name": name, "template": input.template, "accounts": template.len() }),
    )
    .await?;
    tx.commit().await?;

    Ok(Company {
        name,
        currency: "IDR".into(),
        fiscal_year_start_month: input.fiscal_year_start_month,
        address: None,
    })
}

/// Ubah nama dan alamat perusahaan (dipakai pada kop laporan).
pub async fn update(pool: &SqlitePool, input: UpdateInput) -> AppResult<Company> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::Validation("Nama perusahaan wajib diisi (maksimal 120 karakter)".into()));
    }
    let address = input.address.as_deref().map(str::trim).filter(|a| !a.is_empty()).map(String::from);
    if address.as_ref().is_some_and(|a| a.chars().count() > 300) {
        return Err(AppError::Validation("Alamat maksimal 300 karakter".into()));
    }

    let mut tx = pool.begin().await?;
    let before = get(pool).await?.ok_or_else(|| AppError::Validation("Perusahaan belum disetup".into()))?;
    sqlx::query("UPDATE company SET name = ?, address = ? WHERE id = 1")
        .bind(&name)
        .bind(&address)
        .execute(&mut *tx)
        .await?;
    audit::log(
        &mut tx,
        "UPDATE",
        "company",
        Some(1),
        json!({ "before": { "name": before.name, "address": before.address }, "after": { "name": name, "address": address } }),
    )
    .await?;
    tx.commit().await?;
    Ok(Company { name, address, ..before })
}
