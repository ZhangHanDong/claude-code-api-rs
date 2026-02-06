use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod core;
mod middleware;
mod models;
mod utils;

use crate::api::chat::ChatState;
use crate::core::{
    claude_manager::ClaudeManager,
    config::Settings,
    process_pool::{PoolConfig, ProcessPool},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::new()?;

    info!(
        "Starting Claude Code API Gateway on {}:{}",
        settings.server.host, settings.server.port
    );

    let app = create_app(settings.clone()).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.server.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Server running on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(settings: Settings) -> Result<Router> {
    use crate::core::{
        cache::{CacheConfig, ResponseCache},
        conversation::{ConversationConfig, ConversationManager},
        interactive_session::InteractiveSessionManager,
        storage::{InMemoryConversationConfig, InMemoryConversationStore},
    };
    use crate::middleware::{error_handler, request_id};
    use axum::middleware;

    let cors = CorsLayer::permissive();

    let claude_manager = Arc::new(ClaudeManager::new(
        settings.claude.command.clone(),
        settings.file_access.clone(),
        settings.mcp.clone(),
    ));

    // 创建进程池配置
    let pool_config = PoolConfig {
        min_idle: settings.process_pool.min_idle,
        max_idle: settings.process_pool.max_idle,
        max_active: settings.process_pool.size,
        idle_timeout_secs: 300,
        default_model: "claude-sonnet-4-20250514".to_string(),
    };

    // 初始化进程池
    info!(
        "Initializing process pool with {} min idle processes",
        pool_config.min_idle
    );
    let process_pool = Arc::new(ProcessPool::new(claude_manager.clone(), pool_config));

    // 初始化交互式会话管理器
    info!("Initializing interactive session manager");
    let interactive_session_manager = Arc::new(InteractiveSessionManager::new(
        claude_manager.clone(),
        settings.claude.command.clone(),
    ));

    // 如果启用了交互式会话，预热一个默认进程
    if settings.claude.use_interactive_sessions
        && let Err(e) = interactive_session_manager.prewarm_default_session().await
    {
        tracing::error!("Failed to pre-warm Claude process: {}", e);
    }

    let conversation_store = InMemoryConversationStore::new(InMemoryConversationConfig::default());
    let conversation_manager = Arc::new(ConversationManager::new(
        conversation_store,
        ConversationConfig::default(),
    ));
    let cache = Arc::new(ResponseCache::new(CacheConfig::default()));

    let chat_state = ChatState::new(
        claude_manager.clone(),
        process_pool.clone(),
        interactive_session_manager.clone(),
        conversation_manager.clone(),
        cache.clone(),
        settings.claude.use_interactive_sessions,
        Arc::new(settings.clone()),
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
        .route(
            "/v1/conversations",
            post(api::conversations::create_conversation),
        )
        .route(
            "/v1/conversations",
            get(api::conversations::list_conversations),
        )
        .route(
            "/v1/conversations/:id",
            get(api::conversations::get_conversation),
        )
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
