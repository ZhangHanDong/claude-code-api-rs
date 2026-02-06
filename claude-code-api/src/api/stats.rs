use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;
use std::sync::Arc;

use crate::{core::cache::ResponseCache, models::error::ApiResult};

#[derive(Clone)]
pub struct StatsState {
    pub cache: Arc<ResponseCache>,
}

#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub cache: crate::core::cache::CacheStats,
    pub version: &'static str,
}

pub async fn get_stats(State(state): State<StatsState>) -> ApiResult<impl IntoResponse> {
    let stats = SystemStats {
        cache: state.cache.stats(),
        version: env!("CARGO_PKG_VERSION"),
    };

    Ok(Json(stats))
}
