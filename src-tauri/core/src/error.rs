use serde::{Serialize, Serializer};

/// Errors are shown to the user as-is, so messages are German.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Datei- oder Ordnerfehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("Netzwerkfehler: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Ungültige Daten: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Archivfehler: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Msg(String),
}

impl AppError {
    pub fn msg(text: impl Into<String>) -> Self {
        AppError::Msg(text.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
