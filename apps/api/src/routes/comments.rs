use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_authenticated, require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, CommentListQuery, CommentNode, CreateCommentRequest, Paginated,
    UpdateCommentRequest, PERMISSION_COMMENT_CREATE, PERMISSION_COMMENT_REPLY,
    PERMISSION_COMMENT_RESTORE,
};
use crate::state::AppState;

use super::response::{parse_json, parse_path, parse_query, ApiResponse, MessageResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics/{topic_id}/comments", get(list_for_topic))
        .merge(create_root_router(state.clone()))
        .merge(reply_router(state.clone()))
        .merge(mutation_router(state.clone()))
        .merge(restore_router(state))
}

fn create_root_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics/{topic_id}/comments", post(create_root))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_COMMENT_CREATE),
            require_permission,
        ))
}

fn reply_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/comments/{comment_id}/reply", post(reply))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_COMMENT_REPLY),
            require_permission,
        ))
}

fn mutation_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/comments/{comment_id}",
            patch(update).delete(delete_comment),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            require_authenticated,
        ))
}

fn restore_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/comments/{comment_id}/restore", post(restore))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_COMMENT_RESTORE),
            require_permission,
        ))
}

async fn list_for_topic(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<CommentListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<CommentNode>>>> {
    let topic_id = parse_path(path)?;
    let query = parse_query(query)?;
    let mut comments = state.comments().list_for_topic(topic_id, query).await?;
    let viewer_id = optional_viewer(&state, &headers).await;
    state
        .reactions()
        .mark_comment_likes(&mut comments.items, viewer_id)
        .await?;
    Ok(Json(ApiResponse::new(comments)))
}

async fn optional_viewer(state: &AppState, headers: &HeaderMap) -> Option<Uuid> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") || token.is_empty() || token.contains(' ') {
        return None;
    }
    let claims = state.auth().token_service().decode_access_token(token).ok()?;
    let principal = state.authorization().authenticate(claims).await.ok()?;
    Some(principal.user_id)
}

async fn create_root(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<CreateCommentRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<CommentNode>>)> {
    let topic_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let comment = state
        .comments()
        .create_root(&principal, topic_id, request)
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(comment))))
}

async fn reply(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<CreateCommentRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<CommentNode>>)> {
    let comment_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let comment = state
        .comments()
        .reply(&principal, comment_id, request)
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(comment))))
}

async fn update(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdateCommentRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<CommentNode>>> {
    let comment_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let comment = state
        .comments()
        .update(&principal, comment_id, request)
        .await?;
    Ok(Json(ApiResponse::new(comment)))
}

async fn delete_comment(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let comment_id = parse_path(path)?;
    state.comments().delete(&principal, comment_id).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "comment deleted",
    })))
}

async fn restore(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<CommentNode>>> {
    let comment_id = parse_path(path)?;
    let comment = state.comments().restore(&principal, comment_id).await?;
    Ok(Json(ApiResponse::new(comment)))
}
