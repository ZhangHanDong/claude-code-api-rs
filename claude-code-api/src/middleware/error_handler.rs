use axum::{
    Json,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tracing::{error, warn};

use crate::models::error::{ErrorDetail, ErrorResponse};

pub async fn handle_errors(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    let status = response.status();

    if status.is_server_error() {
        error!(
            "Server error: {} {} - Status: {} - Duration: {:?}",
            method, path, status, elapsed
        );
    } else if status.is_client_error() && status != StatusCode::NOT_FOUND {
        warn!(
            "Client error: {} {} - Status: {} - Duration: {:?}",
            method, path, status, elapsed
        );
    }

    response
}

#[allow(dead_code)]
pub async fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    error!("Panic occurred: {}", details);

    let error_response = ErrorResponse {
        error: ErrorDetail {
            message: "Internal server error".to_string(),
            r#type: "internal_error".to_string(),
            param: None,
            code: Some("panic".to_string()),
        },
    };

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
}
