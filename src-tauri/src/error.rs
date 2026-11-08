use serde::Serialize;

/// Error aplikasi. Dikirim ke frontend sebagai `{ code, message }`.
// Validation/Other dipakai mulai Fase 1 (aturan bisnis).
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Kesalahan database: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migrasi database gagal: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Kesalahan berkas: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE",
            AppError::Migration(_) => "MIGRATION",
            AppError::Io(_) => "IO",
            AppError::Validation(_) => "VALIDATION",
            AppError::Other(_) => "OTHER",
        }
    }
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
