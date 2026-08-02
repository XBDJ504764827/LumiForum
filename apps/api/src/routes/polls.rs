//! Phase 14: poll routes — create/read/vote/results + author & admin management.

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_authenticated, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, CancelVoteRequest, CreatePollDraft, HotPollItem, PollDetail,
    PollResults, UpdatePollRequest, VotePollRequest,
};
use crate::state::AppState;

use super::response::{parse_json, parse_path, ApiResponse, MessageResponse};

/// Public read-only routes: poll detail, results, hot polls.
/// Guests may read; authenticated viewers get personalized fields (my_votes…).
pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/polls/{poll}", get(get_poll))
        .route("/polls/{poll}/results", get(get_results))
        .route("/polls/hot", get(get_hot))
        .route("/topics/{topic}/poll", get(get_poll_by_topic))
        .merge(create_router(state.clone()))
        .merge(authenticated_router(state))
}

/// POST /topics/{topic}/poll — attach a poll to a topic (topic author only).
fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics/{topic}/poll", post(create_poll))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            require_authenticated,
        ))
}

/// Voting + author/staff management (authenticated).
fn authenticated_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/polls/{poll}/vote", post(cast_vote).delete(cancel_vote))
        .route("/polls/{poll}", patch(update_poll).delete(delete_poll))
        .route("/polls/{poll}/close", post(close_poll))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            require_authenticated,
        ))
}

async fn get_poll(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<PollDetail>>> {
    let poll_id = parse_path(path)?;
    let viewer = optional_principal(&state, &headers).await;
    let poll = state.polls().get_by_id(viewer.as_ref(), poll_id).await?;
    Ok(Json(ApiResponse::new(poll)))
}

async fn get_poll_by_topic(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<PollDetail>>> {
    let topic_id = parse_path(path)?;
    let viewer = optional_principal(&state, &headers).await;
    let poll = state
        .polls()
        .get_by_topic(viewer.as_ref(), topic_id)
        .await?;
    Ok(Json(ApiResponse::new(poll)))
}

async fn get_results(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<PollResults>>> {
    let poll_id = parse_path(path)?;
    let viewer = optional_principal(&state, &headers).await;
    let results = state.polls().results(viewer.as_ref(), poll_id).await?;
    Ok(Json(ApiResponse::new(results)))
}

async fn get_hot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ApiResponse<Vec<HotPollItem>>>> {
    let _viewer = optional_principal(&state, &headers).await;
    let items = state.polls().hot().await?;
    Ok(Json(ApiResponse::new(items)))
}

async fn create_poll(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<CreatePollDraft>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<PollDetail>>)> {
    let topic_id = parse_path(path)?;
    let draft = parse_json(payload)?;
    let poll = state
        .polls()
        .create_for_topic(&principal, topic_id, draft)
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(poll))))
}

async fn cast_vote(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<VotePollRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<PollDetail>>> {
    let poll_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let poll = state.polls().vote(&principal, poll_id, request).await?;
    Ok(Json(ApiResponse::new(poll)))
}

async fn cancel_vote(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<CancelVoteRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<PollDetail>>> {
    let poll_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let poll = state
        .polls()
        .cancel_vote(&principal, poll_id, request.option_id)
        .await?;
    Ok(Json(ApiResponse::new(poll)))
}

async fn update_poll(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdatePollRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<PollDetail>>> {
    let poll_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let poll = state.polls().update(&principal, poll_id, request).await?;
    Ok(Json(ApiResponse::new(poll)))
}

async fn close_poll(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let poll_id = parse_path(path)?;
    state.polls().close(&principal, poll_id).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "投票已关闭",
    })))
}

/// Staff-only hard delete (service enforces PERMISSION_ADMIN_ACCESS).
async fn delete_poll(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let poll_id = parse_path(path)?;
    state.polls().delete(&principal, poll_id).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "投票已删除",
    })))
}

/// Resolve the viewer from the Authorization header when present.
/// Mirrors the optional-viewer pattern used by public topic routes.
async fn optional_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<AuthenticatedPrincipal> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") || token.is_empty() || token.contains(' ') {
        return None;
    }
    let claims = state
        .auth()
        .token_service()
        .decode_access_token(token)
        .ok()?;
    state.authorization().authenticate(claims).await.ok()
}
