use axum::{extract::Request, http::HeaderName, middleware::Next, response::Response};
use uuid::Uuid;

pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub async fn add_request_id(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.headers_mut()
        .insert(X_REQUEST_ID.clone(), request_id.parse().unwrap());

    let mut response = next.run(req).await;

    response
        .headers_mut()
        .insert(X_REQUEST_ID.clone(), request_id.parse().unwrap());

    response
}
