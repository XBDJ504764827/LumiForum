use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, CommentLikeState, FavoriteItem, FavoriteState, FollowState, Paginated,
    ReactionListQuery, TopicLikeState, UserPublicSummary, PERMISSION_COMMENT_LIKE,
    PERMISSION_TOPIC_FAVORITE, PERMISSION_TOPIC_LIKE, PERMISSION_USER_FOLLOW,
};
use crate::state::AppState;

use super::response::{parse_path, parse_query, ApiResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(topic_like_router(state.clone()))
        .merge(comment_like_router(state.clone()))
        .merge(favorite_router(state.clone()))
        .merge(follow_router(state.clone()))
        .merge(public_lists_router(state))
}

fn topic_like_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/topics/{topic_id}/like",
            post(like_topic).delete(unlike_topic),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_TOPIC_LIKE),
            require_permission,
        ))
}

fn comment_like_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/comments/{comment_id}/like",
            post(like_comment).delete(unlike_comment),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_COMMENT_LIKE),
            require_permission,
        ))
}

fn favorite_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/topics/{topic_id}/favorite",
            post(favorite_topic).delete(unfavorite_topic),
        )
        .route("/me/favorites", get(list_my_favorites))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_TOPIC_FAVORITE),
            require_permission,
        ))
}

fn follow_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/users/{user_id}/follow",
            post(follow_user).delete(unfollow_user),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_USER_FOLLOW),
            require_permission,
        ))
}

fn public_lists_router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users/{user_id}/followers", get(list_followers))
        .route("/users/{user_id}/following", get(list_following))
}

async fn like_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<TopicLikeState>>)> {
    let topic_id = parse_path(path)?;
    let result = state.reactions().like_topic(&principal, topic_id).await?;
    Ok((StatusCode::OK, Json(ApiResponse::new(result))))
}

async fn unlike_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<TopicLikeState>>> {
    let topic_id = parse_path(path)?;
    let result = state.reactions().unlike_topic(&principal, topic_id).await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn like_comment(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<CommentLikeState>>)> {
    let comment_id = parse_path(path)?;
    let result = state
        .reactions()
        .like_comment(&principal, comment_id)
        .await?;
    Ok((StatusCode::OK, Json(ApiResponse::new(result))))
}

async fn unlike_comment(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<CommentLikeState>>> {
    let comment_id = parse_path(path)?;
    let result = state
        .reactions()
        .unlike_comment(&principal, comment_id)
        .await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn favorite_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<FavoriteState>>)> {
    let topic_id = parse_path(path)?;
    let result = state
        .reactions()
        .favorite_topic(&principal, topic_id)
        .await?;
    Ok((StatusCode::OK, Json(ApiResponse::new(result))))
}

async fn unfavorite_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<FavoriteState>>> {
    let topic_id = parse_path(path)?;
    let result = state
        .reactions()
        .unfavorite_topic(&principal, topic_id)
        .await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn list_my_favorites(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<ReactionListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<FavoriteItem>>>> {
    let query = parse_query(query)?;
    let result = state
        .reactions()
        .list_my_favorites(&principal, query)
        .await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn follow_user(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<FollowState>>)> {
    let user_id = parse_path(path)?;
    let result = state.reactions().follow_user(&principal, user_id).await?;
    Ok((StatusCode::OK, Json(ApiResponse::new(result))))
}

async fn unfollow_user(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<FollowState>>> {
    let user_id = parse_path(path)?;
    let result = state.reactions().unfollow_user(&principal, user_id).await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn list_followers(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<ReactionListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<UserPublicSummary>>>> {
    let user_id = parse_path(path)?;
    let query = parse_query(query)?;
    let viewer_id = optional_viewer(&state, &headers).await;
    let result = state
        .reactions()
        .list_followers(user_id, viewer_id, query)
        .await?;
    Ok(Json(ApiResponse::new(result)))
}

async fn list_following(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<ReactionListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<UserPublicSummary>>>> {
    let user_id = parse_path(path)?;
    let query = parse_query(query)?;
    let viewer_id = optional_viewer(&state, &headers).await;
    let result = state
        .reactions()
        .list_following(user_id, viewer_id, query)
        .await?;
    Ok(Json(ApiResponse::new(result)))
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
