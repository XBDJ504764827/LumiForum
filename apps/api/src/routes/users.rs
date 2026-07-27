use axum::{
    extract::{rejection::JsonRejection, State},
    middleware,
    routing::patch,
    Extension, Json, Router,
};

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, ProfileUpdateRequest, UserResponse, PERMISSION_PROFILE_UPDATE_SELF,
};
use crate::state::AppState;

use super::response::{parse_json, ApiResponse};

pub fn protected_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users/profile", patch(update_profile))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_PROFILE_UPDATE_SELF),
            require_permission,
        ))
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<ProfileUpdateRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let request = parse_json(payload)?;
    let user = state
        .users()
        .update_profile(principal.user_id, request)
        .await?;
    Ok(Json(ApiResponse::new(user)))
}
