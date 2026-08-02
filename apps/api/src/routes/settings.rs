//! Phase 15: public settings — read-only subset for the forum frontend,
//! Redis-cached so DB-backed settings apply with low latency.

use axum::{extract::State, routing::get, Json, Router};
use redis::AsyncCommands;

use crate::error::AppResult;
use crate::models::PublicSettings;
use crate::state::AppState;

use super::response::ApiResponse;

const SETTINGS_CACHE_KEY: &str = "settings:public:v1";
const SETTINGS_CACHE_TTL_SECS: u64 = 60;

pub fn router() -> Router<AppState> {
    Router::new().route("/public/settings", get(get_public_settings))
}

async fn get_public_settings(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<PublicSettings>>> {
    let mut redis = state.redis().clone();
    if let Ok(Some(cached)) = redis.get::<_, Option<Vec<u8>>>(SETTINGS_CACHE_KEY).await {
        if let Ok(settings) = serde_json::from_slice::<PublicSettings>(&cached) {
            return Ok(Json(ApiResponse::new(settings)));
        }
    }

    let settings = state.admin().public_settings().await?;
    if let Ok(payload) = serde_json::to_vec(&settings) {
        let _ = redis
            .set_ex::<_, _, ()>(SETTINGS_CACHE_KEY, payload, SETTINGS_CACHE_TTL_SECS)
            .await;
    }
    Ok(Json(ApiResponse::new(settings)))
}
