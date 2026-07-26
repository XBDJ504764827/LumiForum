use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        State,
    },
    http::StatusCode,
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AuthenticatedPrincipal, CategoryResponse, CreateCategoryRequest, UpdateCategoryRequest,
    PERMISSION_CATEGORY_MANAGE,
};
use crate::state::AppState;

use super::response::{parse_json, parse_path, ApiResponse, MessageResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/categories", get(list))
        .route("/categories/{category}", get(get_by_slug))
        .merge(management_router(state))
}

fn management_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/categories", post(create))
        .route(
            "/categories/{category}",
            patch(update).delete(delete_category),
        )
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_CATEGORY_MANAGE),
            require_permission,
        ))
}

async fn list(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<CategoryResponse>>>> {
    let categories = state.categories().list_public().await?;
    Ok(Json(ApiResponse::new(categories)))
}

async fn get_by_slug(
    State(state): State<AppState>,
    path: Result<axum::extract::Path<String>, PathRejection>,
) -> AppResult<Json<ApiResponse<CategoryResponse>>> {
    let slug = parse_path(path)?;
    let category = state.categories().get_public(&slug).await?;
    Ok(Json(ApiResponse::new(category)))
}

async fn create(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<CreateCategoryRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<CategoryResponse>>)> {
    let request = parse_json(payload)?;
    let category = state.categories().create(&principal, request).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(category))))
}

async fn update(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdateCategoryRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<CategoryResponse>>> {
    let category_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let category = state
        .categories()
        .update(&principal, category_id, request)
        .await?;
    Ok(Json(ApiResponse::new(category)))
}

async fn delete_category(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let category_id = parse_path(path)?;
    state.categories().delete(&principal, category_id).await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "category deleted",
    })))
}
