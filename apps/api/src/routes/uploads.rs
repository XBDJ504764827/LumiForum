use std::str::FromStr;

use axum::{
    extract::{
        multipart::MultipartRejection, rejection::PathRejection, DefaultBodyLimit, Multipart, State,
    },
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use bytes::Bytes;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, Paginated, UploadCategory, UploadListQuery, UploadResponse,
    UserResponse, PERMISSION_UPLOAD_CREATE, PERMISSION_UPLOAD_DELETE_SELF,
    PERMISSION_UPLOAD_READ_SELF,
};
use crate::services::UploadInput;
use crate::state::AppState;

use super::response::{parse_path, parse_query, ApiResponse, MessageResponse};

const MULTIPART_LIMIT: usize = 50 * 1024 * 1024 + 64 * 1024;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(create_router(state.clone()))
        .merge(read_router(state.clone()))
        .merge(delete_router(state))
}

fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/uploads", post(create))
        .route("/users/profile/avatar", post(create_avatar))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_UPLOAD_CREATE),
            require_permission,
        ))
        .layer(DefaultBodyLimit::max(MULTIPART_LIMIT))
}

fn read_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/uploads/{id}", get(get_upload))
        .route("/users/{id}/uploads", get(list_user_uploads))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_UPLOAD_READ_SELF),
            require_permission,
        ))
}

fn delete_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/uploads/{id}", delete(delete_upload))
        .route("/users/profile/avatar", delete(delete_avatar))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_UPLOAD_DELETE_SELF),
            require_permission,
        ))
}

async fn create(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    multipart: Result<Multipart, MultipartRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<UploadResponse>>)> {
    let input = parse_multipart(multipart, None).await?;
    let upload = state.uploads().create(principal.user_id, input).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(upload))))
}

async fn create_avatar(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    multipart: Result<Multipart, MultipartRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<UserResponse>>)> {
    let input = parse_multipart(multipart, Some(UploadCategory::Avatar)).await?;
    state
        .uploads()
        .create_avatar(principal.user_id, input)
        .await?;
    let user = state.users().get_profile(principal.user_id).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(user))))
}

async fn get_upload(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<UploadResponse>>> {
    let upload_id = parse_path(path)?;
    let upload = state.uploads().get(principal.user_id, upload_id).await?;
    Ok(Json(ApiResponse::new(upload)))
}

async fn list_user_uploads(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<UploadListQuery>, axum::extract::rejection::QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<UploadResponse>>>> {
    let user_id = parse_path(path)?;
    let query = parse_query(query)?;
    let uploads = state
        .uploads()
        .list_user(principal.user_id, user_id, query)
        .await?;
    Ok(Json(ApiResponse::new(uploads)))
}

async fn delete_upload(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state
        .uploads()
        .delete(principal.user_id, parse_path(path)?)
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "upload deleted",
    })))
}

async fn delete_avatar(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    state.uploads().delete_avatar(principal.user_id).await?;
    let user = state.users().get_profile(principal.user_id).await?;
    Ok(Json(ApiResponse::new(user)))
}

async fn parse_multipart(
    multipart: Result<Multipart, MultipartRejection>,
    forced_category: Option<UploadCategory>,
) -> AppResult<UploadInput> {
    let mut multipart = multipart.map_err(|_| AppError::Validation("invalid multipart body"))?;
    let mut category = forced_category;
    let mut file: Option<(String, Option<String>, Bytes)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("invalid multipart field"))?
    {
        match field.name() {
            Some("category") if forced_category.is_none() && category.is_none() => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| AppError::Validation("invalid upload category"))?;
                category = Some(
                    UploadCategory::from_str(value.trim())
                        .map_err(|_| AppError::Validation("invalid upload category"))?,
                );
            }
            Some("file") if file.is_none() => {
                let filename = field.file_name().unwrap_or("upload").to_owned();
                let content_type = field.content_type().map(str::to_owned);
                let data = field.bytes().await.map_err(|_| AppError::PayloadTooLarge)?;
                file = Some((filename, content_type, data));
            }
            Some("category" | "file") => {
                return Err(AppError::Validation("duplicate multipart field"));
            }
            _ => return Err(AppError::Validation("unknown multipart field")),
        }
    }

    let category = category.ok_or(AppError::Validation("upload category is required"))?;
    let (original_filename, claimed_mime_type, data) =
        file.ok_or(AppError::Validation("file is required"))?;
    Ok(UploadInput {
        original_filename,
        claimed_mime_type,
        category,
        data,
    })
}
