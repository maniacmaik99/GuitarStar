use serde::{Serialize, Serializer};

use crate::tr;

/// Errors are shown to the user as-is, in the language of the UI (see `i18n`).
/// `Msg` texts are translated when they are created; the prefixes of the
/// library errors when they are displayed, which happens right away.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{}", tr!("error.io", error = .0))]
    Io(#[from] std::io::Error),
    #[error("{}", tr!("error.http", error = .0))]
    Http(#[from] reqwest::Error),
    #[error("{}", tr!("error.json", error = .0))]
    Json(#[from] serde_json::Error),
    #[error("{}", tr!("error.zip", error = .0))]
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
