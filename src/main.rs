use anyhow::Result;
use axum::{routing::{get, post}, Router};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;

mod api;
mod core;
mod models;
mod utils;
mod middleware;

use crate::core::{config::Settings, claude_manager::ClaudeManager};
use crate::api::chat::ChatState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "claude_code_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::new()?;

    info!("Starting Claude Code API Gateway on {}:{}",
          settings.server.host, settings.server.port);

    let app = create_app(settings.clone()).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.server.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Server running on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(settings: Settings) -> Result<Router> {
    use crate::core::{
        cache::{ResponseCache, CacheConfig},
        conversation::{ConversationManager, ConversationConfig},
    };
    use crate::middleware::{error_handler, request_id};
    use axum::middleware;

    let cors = CorsLayer::permissive();

    let claude_manager = Arc::new(ClaudeManager::new(
        settings.claude.command.clone(),
        settings.file_access.clone(),
        settings.mcp.clone()
    ));
    let conversation_manager = Arc::new(ConversationManager::new(ConversationConfig::default()));
    let cache = Arc::new(ResponseCache::new(CacheConfig::default()));

    let chat_state = ChatState::new(
        claude_manager.clone(),
        conversation_manager.clone(),
        cache.clone(),
    );

    let conversation_state = api::conversations::ConversationState {
        manager: conversation_manager.clone(),
    };

    let stats_state = api::stats::StatsState {
        cache: cache.clone(),
    };

    let api_routes = Router::new()
        .route("/v1/chat/completions", post(api::chat::chat_completions))
        .with_state(chat_state);

    let conversation_routes = Router::new()
        .route("/v1/conversations", post(api::conversations::create_conversation))
        .route("/v1/conversations", get(api::conversations::list_conversations))
        .route("/v1/conversations/:id", get(api::conversations::get_conversation))
        .with_state(conversation_state);

    let stats_routes = Router::new()
        .route("/stats", get(api::stats::get_stats))
        .with_state(stats_state);

    // 组合所有路由
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/models", get(api::models::list_models))
        .merge(api_routes)
        .merge(conversation_routes)
        .merge(stats_routes)
        .layer(middleware::from_fn(request_id::add_request_id))
        .layer(middleware::from_fn(error_handler::handle_errors))
        .layer(cors);

    Ok(app)
}

async fn health_check() -> &'static str {
    "OK"
}
