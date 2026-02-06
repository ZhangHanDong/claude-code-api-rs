use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Claude process error: {0}")]
    ClaudeProcess(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Invalid model: {0}")]
    InvalidModel(String),

    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, code) = match &self {
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request_error", None),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "authentication_error", None),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found_error", None),
            ApiError::ClaudeProcess(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "claude_process_error",
                None,
            ),
            ApiError::Timeout(_) => (
                StatusCode::GATEWAY_TIMEOUT,
                "timeout_error",
                Some("timeout"),
            ),
            ApiError::RateLimit(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_error",
                Some("rate_limit_exceeded"),
            ),
            ApiError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable", None)
            },
            ApiError::InvalidModel(_) => (
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                Some("invalid_model"),
            ),
            ApiError::ContextLengthExceeded(_) => (
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                Some("context_length_exceeded"),
            ),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None),
        };

        let error_response = ErrorResponse {
            error: ErrorDetail {
                message: self.to_string(),
                r#type: error_type.to_string(),
                param: None,
                code: code.map(String::from),
            },
        };

        (status, Json(error_response)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
