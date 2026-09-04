use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        State,
    },
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, ConversationDetail, ConversationSummary, MessageListQuery, MessagePage,
    MessageResponse, Paginated, SendMessageRequest, StartConversationRequest,
    PERMISSION_DM_READ_SELF, PERMISSION_DM_WRITE,
};
use crate::state::AppState;

use super::response::{parse_path, parse_query, ApiResponse, MessageResponse as ApiMessage};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(read_router(state.clone()))
        .merge(write_router(state))
}

fn read_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/messages", get(list_conversations))
        .route("/messages/unread-count", get(unread_count))
        .route(
            "/messages/conversations/{conversation_id}",
            get(get_conversation),
        )
        .route(
            "/messages/conversations/{conversation_id}/messages",
            get(list_messages),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_DM_READ_SELF),
            require_permission,
        ))
}

fn write_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/messages/conversations", post(start_conversation))
        .route(
            "/messages/conversations/{conversation_id}/messages",
            post(send_message),
        )
        .route(
            "/messages/conversations/{conversation_id}/read",
            post(mark_read),
        )
        .route(
            "/messages/{message_id}",
            axum::routing::delete(delete_message),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_DM_WRITE),
            require_permission,
        ))
}

async fn list_conversations(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<MessageListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<ConversationSummary>>>> {
    let query = parse_query(query)?;
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(30);
    let items = state
        .dm()
        .list_conversations(&principal, page, page_size)
        .await?;
    Ok(Json(ApiResponse::new(items)))
}

async fn unread_count(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<UnreadCount>>> {
    let count = state.dm().unread_count(&principal).await?;
    Ok(Json(ApiResponse::new(UnreadCount { count })))
}

async fn get_conversation(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<ConversationDetail>>> {
    let conversation_id = parse_path(path)?;
    let detail = state
        .dm()
        .get_conversation(&principal, conversation_id)
        .await?;
    Ok(Json(ApiResponse::new(detail)))
}

async fn list_messages(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<MessageListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<MessagePage>>> {
    let conversation_id = parse_path(path)?;
    let query = parse_query(query)?;
    let page = state
        .dm()
        .list_messages(&principal, conversation_id, query)
        .await?;
    Ok(Json(ApiResponse::new(page)))
}

async fn start_conversation(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<StartConversationRequest>,
) -> AppResult<Json<ApiResponse<ConversationDetail>>> {
    let detail = state.dm().start_conversation(&principal, request).await?;
    Ok(Json(ApiResponse::new(detail)))
}

async fn send_message(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    Json(request): Json<SendMessageRequest>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let conversation_id = parse_path(path)?;
    let message = state
        .dm()
        .send_message(&principal, conversation_id, request)
        .await?;
    Ok(Json(ApiResponse::new(message)))
}

async fn mark_read(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<ApiMessage>>> {
    let conversation_id = parse_path(path)?;
    state.dm().mark_read(&principal, conversation_id).await?;
    Ok(Json(ApiResponse::new(ApiMessage {
        message: "conversation marked as read",
    })))
}

async fn delete_message(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<ApiMessage>>> {
    let message_id = parse_path(path)?;
    state.dm().delete_message(&principal, message_id).await?;
    Ok(Json(ApiResponse::new(ApiMessage {
        message: "message deleted",
    })))
}

#[derive(serde::Serialize)]
struct UnreadCount {
    count: i64,
}
