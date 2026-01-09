use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Application-level errors for HTTP handlers
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Upload failed: {0}")]
    UploadError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

impl AppError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NetworkError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ParseError(_) => StatusCode::BAD_REQUEST,
            AppError::UploadError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::JsonError(_) => StatusCode::BAD_REQUEST,
            AppError::ReqwestError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code for the response
    pub fn error_code(&self) -> i32 {
        match self {
            AppError::BadRequest(_) => 1,
            AppError::Unauthorized(_) => 1,
            AppError::NotFound(_) => 1,
            AppError::InternalError(_) => 1,
            AppError::NetworkError(_) => 1,
            AppError::ParseError(_) => 1,
            AppError::UploadError(_) => 1,
            AppError::DatabaseError(_) => 1,
            AppError::JsonError(_) => 1,
            AppError::ReqwestError(_) => 1,
        }
    }

    /// Get the error message for the response
    pub fn error_message(&self) -> Option<String> {
        match self {
            AppError::BadRequest(msg) => Some(msg.clone()),
            AppError::ParseError(msg) => Some(msg.clone()),
            AppError::UploadError(_) => Some("upload file fail".to_string()),
            AppError::NetworkError(_) => Some("create dynamic fail with network fatal".to_string()),
            AppError::InternalError(_) => Some("create dynamic fail".to_string()),
            AppError::ReqwestError(_) => Some("create dynamic fail with network fatal".to_string()),
            // For these, we don't return a message to the client (only log)
            AppError::Unauthorized(_) => None,
            AppError::NotFound(_) => None,
            AppError::DatabaseError(_) => None,
            AppError::JsonError(_) => Some("create dynamic fail".to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let error_message = self.error_message();

        // Log the detailed error
        tracing::error!("Handler error: {:?}", self);

        let body = json!({
            "code": error_code,
            "msg": error_message,
        });

        (status, Json(body)).into_response()
    }
}

/// Result type alias for handlers
pub type AppResult<T> = Result<T, AppError>;
