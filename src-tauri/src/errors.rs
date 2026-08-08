use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub error: String,
}

impl AppError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(format!("io error: {e}"))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::new(format!("database error: {e}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::new(format!("network error: {e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
