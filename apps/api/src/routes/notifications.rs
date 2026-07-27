use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        State,
    },
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, NotificationQuery, NotificationResponse, Paginated,
    UnreadCountResponse, PERMISSION_NOTIFICATION_READ_SELF, PERMISSION_NOTIFICATION_UPDATE_SELF,
};
use crate::state::AppState;

use super::response::{parse_path, parse_query, ApiResponse, MessageResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(read_router(state.clone()))
        .merge(update_router(state))
}

fn read_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list))
        .route("/notifications/unread-count", get(unread_count))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_NOTIFICATION_READ_SELF),
            require_permission,
        ))
}

fn update_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/notifications/{notification_id}/read", patch(mark_read))
        .route("/notifications/read-all", post(mark_all_read))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_NOTIFICATION_UPDATE_SELF),
            require_permission,
        ))
}

async fn list(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<NotificationQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<NotificationResponse>>>> {
    let query = parse_query(query)?;
    let items = state.notifications().list(&principal, query).await?;
    Ok(Json(ApiResponse::new(items)))
}

async fn unread_count(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<UnreadCountResponse>>> {
    let count = state.notifications().unread_count(&principal).await?;
    Ok(Json(ApiResponse::new(count)))
}

async fn mark_read(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let notification_id = parse_path(path)?;
    state
        .notifications()
        .mark_read(&principal, notification_id)
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "notification marked as read",
    })))
}

async fn mark_all_read(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state.notifications().mark_all_read(&principal).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "all notifications marked as read",
    })))
}
