use std::net::SocketAddr;

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        ConnectInfo, State,
    },
    http::{HeaderMap, StatusCode},
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{require_permission, AuthorizationLayer};
use crate::models::{
    AdminCommentListQuery, AdminDashboard, AdminFileListQuery, AdminLogListQuery,
    AdminTopicListQuery, AdminTopicUpdateRequest, AdminUserListQuery, AdminUserUpdateRequest,
    AuthenticatedPrincipal, CategoryResponse, CreateCategoryRequest, CreateReportRequestV2,
    Paginated, ReportListQuery, ResolveReportRequest, UpdateCategoryRequest,
    PERMISSION_ADMIN_ACCESS,
};
use crate::services::AdminAuditContext;
use crate::state::AppState;

use super::response::{parse_json, parse_path, parse_query, ApiResponse, MessageResponse};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/admin/dashboard", get(dashboard))
        .route("/admin/roles", get(list_roles))
        .route("/admin/permissions", get(list_permissions))
        .route(
            "/admin/roles/{code}/permissions",
            get(get_role_permissions).put(update_role_permissions),
        )
        .route("/admin/users", get(list_users))
        .route(
            "/admin/users/{id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
        .route("/admin/users/{id}/detail", get(get_user_detail))
        .route("/admin/users/{id}/login-records", get(list_login_records))
        .route("/admin/users/{id}/force-logout", post(force_logout))
        .route("/admin/topics", get(list_topics))
        .route(
            "/admin/topics/{id}",
            patch(update_topic).delete(delete_topic),
        )
        .route("/admin/comments", get(list_comments))
        .route(
            "/admin/comments/{id}",
            axum::routing::delete(delete_comment),
        )
        .route("/admin/comments/{id}/restore", post(restore_comment))
        .route(
            "/admin/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/admin/categories/{id}",
            patch(update_category).delete(delete_category),
        )
        .route("/admin/files", get(list_files))
        .route("/admin/files/cleanup", post(cleanup_files))
        .route("/admin/files/{id}", axum::routing::delete(delete_file))
        .route("/admin/reports", get(list_reports))
        .route("/admin/reports/{id}", patch(resolve_report))
        .route("/admin/polls", get(list_polls))
        .route("/admin/queue", get(queue_summary))
        .route("/admin/analytics", get(analytics))
        .route("/admin/settings", get(list_settings).put(update_settings))
        .route("/admin/logs", get(list_logs))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state.clone(), PERMISSION_ADMIN_ACCESS),
            require_permission,
        ))
}

pub fn public_report_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/reports", post(create_report))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            crate::middleware::require_authenticated,
        ))
}

async fn dashboard(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<DashboardQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<AdminDashboard>>> {
    let query = parse_query(query)?;
    let range = query.range.unwrap_or_default();
    let system = crate::models::SystemStats {
        api_requests_total: state.metrics().counter_value("http_requests_total"),
        ws_connections: state.realtime().hub().connection_count(),
        online_users: state.presence().count_online().await,
    };
    let data = state.admin().dashboard(&principal, range, system).await?;
    Ok(Json(ApiResponse::new(data)))
}

#[derive(Default, serde::Deserialize)]
struct DashboardQuery {
    range: Option<crate::models::AdminDashboardRange>,
}

async fn get_user_detail(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminUserDetail>>> {
    let user_id = parse_path(path)?;
    let data = state.admin().get_user_detail(&principal, user_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_login_records(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<crate::models::AdminLoginRecordQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::LoginRecordItem>>>> {
    let user_id = parse_path(path)?;
    let query = parse_query(query)?;
    let data = state
        .admin()
        .list_login_records(&principal, user_id, query)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn force_logout(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let user_id = parse_path(path)?;
    let audit = crate::services::AdminAuditContext {
        ip: Some(ipnetwork::IpNetwork::from(addr.ip())),
        user_agent: headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned()),
    };
    state
        .admin()
        .force_logout(&principal, user_id, &audit)
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "已强制该用户退出登录",
    })))
}

async fn list_permissions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<Vec<crate::models::PermissionOption>>>> {
    let data = state.admin().list_permissions(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_role_permissions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<String>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::RolePermissionView>>> {
    let role_code = parse_path(path)?;
    let data = state
        .admin()
        .get_role_permissions(&principal, &role_code)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn update_role_permissions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<String>, PathRejection>,
    payload: Result<Json<crate::models::UpdateRolePermissionsRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::RolePermissionView>>> {
    let role_code = parse_path(path)?;
    let request = parse_json(payload)?;
    let audit = crate::services::AdminAuditContext {
        ip: Some(ipnetwork::IpNetwork::from(addr.ip())),
        user_agent: headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned()),
    };
    let data = state
        .admin()
        .update_role_permissions(&principal, &role_code, request, &audit)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_roles(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<Vec<crate::models::RoleOption>>>> {
    let data = state.admin().list_roles(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_users(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminUserListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::AdminUserItem>>>> {
    let data = state
        .admin()
        .list_users(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_user(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminUserItem>>> {
    let data = state
        .admin()
        .get_user(&principal, parse_path(path)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn update_user(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<AdminUserUpdateRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminUserItem>>> {
    let data = state
        .admin()
        .update_user(
            &principal,
            parse_path(path)?,
            parse_json(payload)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_user(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminUserItem>>> {
    let data = state
        .admin()
        .delete_user(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_topics(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminTopicListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::AdminTopicItem>>>> {
    let data = state
        .admin()
        .list_topics(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn update_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<AdminTopicUpdateRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminTopicItem>>> {
    let data = state
        .admin()
        .update_topic(
            &principal,
            parse_path(path)?,
            parse_json(payload)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_topic(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state
        .admin()
        .delete_topic(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "topic deleted",
    })))
}

async fn list_comments(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminCommentListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::AdminCommentItem>>>> {
    let data = state
        .admin()
        .list_comments(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_comment(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state
        .admin()
        .delete_comment(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "comment deleted",
    })))
}

async fn restore_comment(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminCommentItem>>> {
    let data = state
        .admin()
        .restore_comment(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_categories(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<Vec<CategoryResponse>>>> {
    let data = state.admin().list_categories(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn create_category(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<CreateCategoryRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<CategoryResponse>>)> {
    let data = state
        .admin()
        .create_category(
            &principal,
            parse_json(payload)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn update_category(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<UpdateCategoryRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<CategoryResponse>>> {
    let data = state
        .admin()
        .update_category(
            &principal,
            parse_path(path)?,
            parse_json(payload)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_category(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state
        .admin()
        .delete_category(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "category deleted",
    })))
}

async fn list_files(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminFileListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::AdminFileItem>>>> {
    let data = state
        .admin()
        .list_files(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_file(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    state
        .admin()
        .delete_file(
            &principal,
            parse_path(path)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "file deleted",
    })))
}

async fn cleanup_files(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let cleaned = state
        .admin()
        .cleanup_orphan_files(&principal, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(
        serde_json::json!({ "cleaned": cleaned }),
    )))
}

async fn create_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<CreateReportRequestV2>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<crate::models::ReportItemV2>>)> {
    let data = state
        .moderation()
        .create_report(&principal, parse_json(payload)?)
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn list_reports(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<ReportListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::ReportItem>>>> {
    let data = state
        .admin()
        .list_reports(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn resolve_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<axum::extract::Path<Uuid>, PathRejection>,
    payload: Result<Json<ResolveReportRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItem>>> {
    let data = state
        .admin()
        .resolve_report(
            &principal,
            parse_path(path)?,
            parse_json(payload)?,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_polls(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<crate::models::AdminPollListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::AdminPollItem>>>> {
    let query = parse_query(query)?;
    let data = state.polls().list_admin(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn queue_summary(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<crate::models::QueueSummary>>> {
    let data = state.admin().queue_summary(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn analytics(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<crate::models::AdminAnalyticsQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AdminAnalytics>>> {
    let query = parse_query(query)?;
    let data = state.admin().analytics(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_settings(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<Vec<crate::models::SystemSettingItem>>>> {
    let data = state.admin().list_settings(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn update_settings(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<crate::models::UpdateSettingsRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<Vec<crate::models::SystemSettingItem>>>> {
    let request = parse_json(payload)?;
    let audit = crate::services::AdminAuditContext {
        ip: Some(ipnetwork::IpNetwork::from(addr.ip())),
        user_agent: headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned()),
    };
    let data = state
        .admin()
        .update_settings(&principal, request, &audit)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_logs(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminLogListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<Paginated<crate::models::AdminLogItem>>>> {
    let data = state
        .admin()
        .list_logs(&principal, parse_query(query)?)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

fn audit_context(addr: SocketAddr, headers: &HeaderMap) -> AdminAuditContext {
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(512).collect::<String>());
    AdminAuditContext {
        ip: Some(IpNetwork::from(addr.ip())),
        user_agent,
    }
}
