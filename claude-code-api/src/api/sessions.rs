use crate::models::error::ApiResult;
use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_path: Option<String>,
}

#[allow(dead_code)]
pub async fn list_sessions() -> ApiResult<impl IntoResponse> {
    let sessions: Vec<SessionInfo> = vec![];
    Ok(Json(sessions))
}

#[allow(dead_code)]
pub async fn create_session() -> ApiResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Not implemented"
    })))
}
