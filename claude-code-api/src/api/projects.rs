use crate::models::error::ApiResult;
use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[allow(dead_code)]
pub async fn list_projects() -> ApiResult<impl IntoResponse> {
    let projects: Vec<Project> = vec![];
    Ok(Json(projects))
}

#[allow(dead_code)]
pub async fn create_project() -> ApiResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Not implemented"
    })))
}
