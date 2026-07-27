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
    AuthenticatedPrincipal, CreateTopicRequest, ModerateTopicRequest, Paginated, TopicDetail,
    TopicListQuery, TopicSummary, UpdateTopicRequest, PERMISSION_TOPIC_CREATE,
};
use crate::state::AppState;

use super::response::{parse_json, parse_path, parse_query, ApiResponse, MessageResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics", get(list))
        .route("/topics/{topic}", get(get_by_slug))
        .merge(create_router(state.clone()))
        .merge(mutation_router(state))
}

fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics", post(create))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_TOPIC_CREATE),
            require_permission,
        ))
}

fn mutation_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/topics/{topic}", patch(update).delete(delete_topic))
        .route("/topics/{topic}/moderation", patch(moderate))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            require_authenticated,
        ))
}

async fn list(
    State(state): State<AppState>,
    query: Result<axum::extract::Query<TopicListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<TopicSummary>>>> {
    let query = parse_query(query)?;
    let topics = state.topics().list_public(query).await?;
    Ok(Json(ApiResponse::new(topics)))
}

async fn get_by_slug(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<String>, PathRejection>,
) -> AppResult<Json<ApiResponse<TopicDetail>>> {
    let slug = parse_path(path)?;
    let mut topic = state.topics().get_public(&slug).await?;
    if let Some(viewer_id) = optional_viewer(&state, &headers).await {
        let (liked, favorited, following) = state
            .reactions()
            .viewer_topic_flags(topic.id, topic.author.id, Some(viewer_id))
            .await?;
        topic.liked_by_me = liked;
        topic.favorited_by_me = favorited;
        topic.following_author = following;
    }
    Ok(Json(ApiResponse::new(topic)))
}

async fn optional_viewer(state: &AppState, headers: &HeaderMap) -> Option<Uuid> {
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
    let principal = state.authorization().authenticate(claims).await.ok()?;
    Some(principal.user_id)
}

async fn create(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<CreateTopicRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<TopicDetail>>)> {
    let request = parse_json(payload)?;
    let topic = state.topics().create(&principal, request).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(topic))))
}

async fn update(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdateTopicRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<TopicDetail>>> {
    let topic_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let topic = state.topics().update(&principal, topic_id, request).await?;
    Ok(Json(ApiResponse::new(topic)))
}

async fn moderate(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<ModerateTopicRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<TopicDetail>>> {
    let topic_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let topic = state
        .topics()
        .moderate(&principal, topic_id, request)
        .await?;
    Ok(Json(ApiResponse::new(topic)))
}

async fn delete_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let topic_id = parse_path(path)?;
    state.topics().delete(&principal, topic_id).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "topic deleted",
    })))
}
