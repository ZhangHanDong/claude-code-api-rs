use axum::{Json, response::IntoResponse};
use chrono::Utc;
use crate::models::{
    openai::{Model, ModelList},
    claude::ClaudeModel,
    error::ApiResult,
};

pub async fn list_models() -> ApiResult<impl IntoResponse> {
    let claude_models = ClaudeModel::all();
    
    let models: Vec<Model> = claude_models.into_iter()
        .map(|m| Model {
            id: m.id,
            object: "model".to_string(),
            created: Utc::now().timestamp(),
            owned_by: "anthropic".to_string(),
        })
        .collect();
    
    let response = ModelList {
        object: "list".to_string(),
        data: models,
    };
    
    Ok(Json(response))
}