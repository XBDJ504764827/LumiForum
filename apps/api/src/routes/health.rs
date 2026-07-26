use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
    postgres: &'static str,
    redis: &'static str,
    timestamp: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "lumiforum-api",
        timestamp: Utc::now(),
    })
}

async fn ready(State(state): State<AppState>) -> AppResult<Json<ReadyResponse>> {
    sqlx::query("SELECT 1")
        .fetch_one(state.db())
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let mut redis = state.redis().clone();
    redis::cmd("PING")
        .query_async::<String>(&mut redis)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    Ok(Json(ReadyResponse {
        status: "ready",
        postgres: "up",
        redis: "up",
        timestamp: Utc::now(),
    }))
}
