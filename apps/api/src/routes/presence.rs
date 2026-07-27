use axum::{
    extract::{rejection::PathRejection, State},
    Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::realtime::PresenceStatus;
use crate::state::AppState;

use super::response::{parse_path, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new().route("/users/{id}/presence", axum::routing::get(get_presence))
}

async fn get_presence(
    State(state): State<AppState>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<PresenceStatus>>> {
    let user_id = parse_path(path)?;
    let status = state.presence().get(user_id).await;
    Ok(Json(ApiResponse::new(status)))
}
