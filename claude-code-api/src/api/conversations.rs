use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    core::conversation::DefaultConversationManager,
    models::error::{ApiError, ApiResult},
};

#[derive(Clone)]
pub struct ConversationState {
    pub manager: Arc<DefaultConversationManager>,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub model: Option<String>,
    pub project_path: Option<String>,
}

pub async fn create_conversation(
    State(state): State<ConversationState>,
    Json(request): Json<CreateConversationRequest>,
) -> ApiResult<impl IntoResponse> {
    let id = state
        .manager
        .create_conversation(request.model.clone())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if let Some(project_path) = request.project_path {
        state
            .manager
            .update_metadata(&id, |metadata| {
                metadata.project_path = Some(project_path);
            })
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    let conversation =
        state.manager.get_conversation(&id).await.ok_or_else(|| {
            ApiError::Internal("Failed to retrieve created conversation".to_string())
        })?;

    let response = ConversationResponse {
        id: conversation.id,
        created_at: conversation.created_at,
        updated_at: conversation.updated_at,
        message_count: conversation.messages.len(),
        metadata: serde_json::to_value(conversation.metadata)?,
    };

    Ok(Json(response))
}

pub async fn get_conversation(
    State(state): State<ConversationState>,
    Path(conversation_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let conversation = state
        .manager
        .get_conversation(&conversation_id)
        .await
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

    let response = ConversationResponse {
        id: conversation.id,
        created_at: conversation.created_at,
        updated_at: conversation.updated_at,
        message_count: conversation.messages.len(),
        metadata: serde_json::to_value(conversation.metadata)?,
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub id: String,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_conversations(
    State(state): State<ConversationState>,
) -> ApiResult<impl IntoResponse> {
    let conversations = state.manager.list_active_conversations().await;

    let response = ConversationListResponse {
        conversations: conversations
            .into_iter()
            .map(|(id, updated_at)| ConversationSummary { id, updated_at })
            .collect(),
    };

    Ok(Json(response))
}
