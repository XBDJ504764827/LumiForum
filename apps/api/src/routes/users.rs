use axum::{
    extract::{rejection::JsonRejection, State},
    middleware,
    routing::{patch, post},
    Extension, Json, Router,
};

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, ChangePasswordRequest, ProfileUpdateRequest, UserResponse,
    PERMISSION_PROFILE_UPDATE_SELF,
};
use crate::state::AppState;

use super::response::{parse_json, ApiResponse};

pub fn protected_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users/profile", patch(update_profile))
        .route("/users/profile/password", post(change_password))
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

async fn change_password(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<ChangePasswordRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let request = parse_json(payload)?;
    let user = state
        .users()
        .set_password(principal.user_id, request)
        .await?;
    Ok(Json(ApiResponse::new(user)))
}
