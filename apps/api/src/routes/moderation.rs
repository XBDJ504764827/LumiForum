//! Phase 13: moderation routes — user-facing endpoints and the /admin/moderation
//! console. Follows the existing route conventions (ApiResponse, audit context,
//! AuthorizationLayer middleware).

use std::net::SocketAddr;

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        ConnectInfo, Path, State,
    },
    http::{HeaderMap, StatusCode},
    middleware,
    routing::{get, patch, post},
    Extension, Json, Router,
};
use ipnetwork::IpNetwork;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppResult;
use crate::middleware::{
    enforce_mutation_origin, require_authenticated, require_permission, AuthorizationLayer,
    CsrfLayer,
};
use crate::models::{
    AdminLogListQuery, AppealListQuery, AuthenticatedPrincipal, CaseActionRequest, CaseQuery,
    CreateAppealRequest, CreateSanctionRequest, ModerationReportQuery,
    NoteRequest, ResolveReportRequestV2, ReviewAppealRequest, RevokeSanctionRequest, RuleListQuery,
    RuleRequest, SanctionListQuery, PERMISSION_ADMIN_ACCESS,
};
use crate::services::{AdminAuditContext, BatchResult};
use crate::state::AppState;

use super::response::{parse_json, parse_path, parse_query, ApiResponse, MessageResponse};

// ---------------------------------------------------------------------------
// User-facing endpoints
// ---------------------------------------------------------------------------

pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/reports/me", get(list_my_reports))
        .route("/reports/{id}", get(get_my_report).delete(cancel_report))
        .route("/moderation/sanctions/me", get(list_my_sanctions))
        .route("/moderation/sanctions/{id}", get(get_my_sanction))
        .route("/moderation/sanctions/{id}/appeals", post(appeal_sanction))
        .route("/moderation/appeals", post(create_appeal))
        .route("/moderation/appeals/me", get(list_my_appeals))
        .route("/moderation/appeals/{id}", get(get_my_appeal))
        .route_layer(middleware::from_fn_with_state(
            CsrfLayer::new(state.config().cors_origin.clone()),
            enforce_mutation_origin,
        ))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::authenticated(state),
            require_authenticated,
        ))
}

// ---------------------------------------------------------------------------
// Admin moderation console
// ---------------------------------------------------------------------------

pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/admin/moderation/reports", get(list_reports))
        .route("/admin/moderation/reports/batch", post(batch_reports))
        .route("/admin/moderation/reports/{id}", get(get_report_detail))
        .route("/admin/moderation/reports/{id}/resolve", post(resolve_report))
        .route("/admin/moderation/reports/{id}/reject", post(reject_report))
        .route("/admin/moderation/reports/{id}/duplicate", post(duplicate_report))
        .route("/admin/moderation/cases", get(list_cases))
        .route("/admin/moderation/cases/{id}", get(get_case_detail))
        .route("/admin/moderation/cases/{id}/assign", post(assign_case))
        .route("/admin/moderation/cases/{id}/release", post(release_case))
        .route("/admin/moderation/cases/{id}/transfer", post(transfer_case))
        .route("/admin/moderation/cases/{id}/close", post(close_case))
        .route("/admin/moderation/cases/{id}/notes", post(add_note))
        .route("/admin/moderation/topics/{id}/actions", post(topic_action))
        .route("/admin/moderation/comments/{id}/actions", post(comment_action))
        .route("/admin/moderation/users/{id}/sanctions", get(list_user_sanctions).post(issue_sanction))
        .route("/admin/moderation/sanctions", get(list_sanctions))
        .route("/admin/moderation/sanctions/{id}/revoke", post(revoke_sanction))
        .route("/admin/moderation/appeals", get(list_appeals))
        .route("/admin/moderation/appeals/{id}/review", post(review_appeal))
        .route("/admin/moderation/rules", get(list_rules).post(create_rule))
        .route(
            "/admin/moderation/rules/{id}",
            patch(update_rule).delete(delete_rule),
        )
        .route("/admin/moderation/audit-logs", get(list_audit_logs))
        .route("/admin/moderation/metrics", get(governance_metrics))
        .route_layer(middleware::from_fn_with_state(
            CsrfLayer::new(state.config().cors_origin.clone()),
            enforce_mutation_origin,
        ))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state.clone(), PERMISSION_ADMIN_ACCESS),
            require_permission,
        ))
}

pub fn metrics_router(state: AppState) -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_text)).with_state(state)
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AssignCaseRequest {
    assignee_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct TransferCaseRequest {
    assignee_id: Uuid,
}

#[derive(Deserialize)]
struct DuplicateReportRequest {
    duplicate_of: Uuid,
}

#[derive(Default, Deserialize)]
struct PageQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

// ---------------------------------------------------------------------------
// User handlers
// ---------------------------------------------------------------------------

async fn list_my_reports(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<PageQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::ReportItemV2>>>> {
    let query = parse_query(query)?;
    let data = state
        .moderation()
        .list_my_reports(&principal, query.page, query.page_size)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_my_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItemV2>>> {
    let report_id = parse_path(path)?;
    let data = state.moderation().get_my_report(&principal, report_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn cancel_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItemV2>>> {
    let report_id = parse_path(path)?;
    let data = state.moderation().cancel_report(&principal, report_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_my_sanctions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<PageQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::SanctionItem>>>> {
    let query = parse_query(query)?;
    let data = state
        .moderation()
        .list_my_sanctions(&principal, query.page, query.page_size)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_my_sanction(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::SanctionItem>>> {
    let sanction_id = parse_path(path)?;
    let data = state.moderation().get_my_sanction(&principal, sanction_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn appeal_sanction(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<CreateAppealRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<crate::models::AppealItem>>)> {
    let sanction_id = parse_path(path)?;
    let mut request = parse_json(payload)?;
    request.sanction_id = Some(sanction_id);
    request.content_type = None;
    request.content_id = None;
    let data = state.moderation().create_appeal(&principal, request).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn create_appeal(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<CreateAppealRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<crate::models::AppealItem>>)> {
    let request = parse_json(payload)?;
    let data = state.moderation().create_appeal(&principal, request).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn list_my_appeals(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<PageQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::AppealItem>>>> {
    let query = parse_query(query)?;
    let data = state
        .moderation()
        .list_my_appeals(&principal, query.page, query.page_size)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_my_appeal(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AppealItem>>> {
    let appeal_id = parse_path(path)?;
    let data = state.moderation().get_my_appeal(&principal, appeal_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

// ---------------------------------------------------------------------------
// Admin handlers
// ---------------------------------------------------------------------------

async fn list_reports(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<ModerationReportQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::ReportItemV2>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_reports(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_report_detail(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let report_id = parse_path(path)?;
    let (report, case) = state.moderation().get_report_detail(&principal, report_id).await?;
    Ok(Json(ApiResponse::new(serde_json::json!({
        "report": report,
        "case": case,
    }))))
}

async fn batch_reports(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<Vec<crate::models::BatchReportItem>>, JsonRejection>,
) -> AppResult<Json<ApiResponse<BatchResult>>> {
    let items = parse_json(payload)?;
    let data = state
        .moderation()
        .batch_reports(&principal, items, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn resolve_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<ResolveReportRequestV2>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItemV2>>> {
    let report_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .handle_report(&principal, report_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn reject_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<serde_json::Value>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItemV2>>> {
    let report_id = parse_path(path)?;
    let note = parse_json(payload)?
        .get("note")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let data = state
        .moderation()
        .reject_report(&principal, report_id, note, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn duplicate_report(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<DuplicateReportRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ReportItemV2>>> {
    let report_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .duplicate_report(
            &principal,
            report_id,
            request.duplicate_of,
            &audit_context(addr, &headers),
        )
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_cases(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<CaseQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::CaseItem>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_cases(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn get_case_detail(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::CaseDetail>>> {
    let case_id = parse_path(path)?;
    let data = state.moderation().case_detail(&principal, case_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn assign_case(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Option<Json<AssignCaseRequest>>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::CaseItem>>> {
    let case_id = parse_path(path)?;
    let assignee_id = match payload {
        Ok(Some(Json(body))) => body.assignee_id,
        _ => None,
    };
    let data = state
        .moderation()
        .assign_case(&principal, case_id, assignee_id)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn release_case(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<crate::models::CaseItem>>> {
    let case_id = parse_path(path)?;
    let data = state.moderation().release_case(&principal, case_id).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn transfer_case(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<TransferCaseRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::CaseItem>>> {
    let case_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .transfer_case(&principal, case_id, request.assignee_id)
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn close_case(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<serde_json::Value>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::CaseItem>>> {
    let case_id = parse_path(path)?;
    let reason = parse_json(payload)?
        .get("reason")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let data = state.moderation().close_case(&principal, case_id, reason).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn add_note(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<NoteRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<Vec<crate::models::ModerationNoteItem>>>> {
    let case_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state.moderation().add_note(&principal, case_id, request).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn topic_action(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<CaseActionRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ContentActionResult>>> {
    let topic_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .topic_action(&principal, topic_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn comment_action(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<CaseActionRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::ContentActionResult>>> {
    let comment_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .comment_action(&principal, comment_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_user_sanctions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<axum::extract::Query<SanctionListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::SanctionItem>>>> {
    let user_id = parse_path(path)?;
    let mut query = parse_query(query)?;
    query.user_id = Some(user_id);
    let data = state.moderation().list_sanctions(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn issue_sanction(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<CreateSanctionRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<crate::models::SanctionItem>>)> {
    let user_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .issue_sanction(&principal, user_id, request, &audit_context(addr, &headers))
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn list_sanctions(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<SanctionListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::SanctionItem>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_sanctions(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn revoke_sanction(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<RevokeSanctionRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::SanctionItem>>> {
    let sanction_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .revoke_sanction(&principal, sanction_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_appeals(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AppealListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::AppealItem>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_appeals(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn review_appeal(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<ReviewAppealRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::AppealItem>>> {
    let appeal_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .review_appeal(&principal, appeal_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn list_rules(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<RuleListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::RuleItem>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_rules(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn create_rule(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<RuleRequest>, JsonRejection>,
) -> AppResult<(StatusCode, Json<ApiResponse<crate::models::RuleItem>>)> {
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .create_rule(&principal, request, &audit_context(addr, &headers))
        .await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::new(data))))
}

async fn update_rule(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<RuleRequest>, JsonRejection>,
) -> AppResult<Json<ApiResponse<crate::models::RuleItem>>> {
    let rule_id = parse_path(path)?;
    let request = parse_json(payload)?;
    let data = state
        .moderation()
        .update_rule(&principal, rule_id, request, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn delete_rule(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: Result<Path<Uuid>, PathRejection>,
) -> AppResult<Json<ApiResponse<MessageResponse>>> {
    let rule_id = parse_path(path)?;
    state
        .moderation()
        .delete_rule(&principal, rule_id, &audit_context(addr, &headers))
        .await?;
    Ok(Json(ApiResponse::new(MessageResponse {
        message: "rule deleted",
    })))
}

async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    query: Result<axum::extract::Query<AdminLogListQuery>, QueryRejection>,
) -> AppResult<Json<ApiResponse<crate::models::Paginated<crate::models::AdminLogItem>>>> {
    let query = parse_query(query)?;
    let data = state.moderation().list_audit_logs(&principal, query).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn governance_metrics(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<crate::models::GovernanceMetrics>>> {
    let data = state.moderation().governance_metrics(&principal).await?;
    Ok(Json(ApiResponse::new(data)))
}

async fn metrics_text(
    State(state): State<AppState>,
) -> AppResult<axum::response::Response> {
    let mut body = state.metrics().render();
    // Append live gauges computed from the database (low cardinality, no auth).
    match state.moderation().metrics_snapshot().await {
        Ok(metrics) => {
            body.push_str("# HELP moderation_reports_pending Reports currently open or under review\n");
            body.push_str("# TYPE moderation_reports_pending gauge\n");
            body.push_str(&format!("moderation_reports_pending {}\n", metrics.reports_pending));
            body.push_str("# HELP moderation_sanctions_active Active sanctions\n");
            body.push_str("# TYPE moderation_sanctions_active gauge\n");
            body.push_str(&format!("moderation_sanctions_active {}\n", metrics.sanctions_active));
            body.push_str("# HELP moderation_appeals_pending Appeals currently pending\n");
            body.push_str("# TYPE moderation_appeals_pending gauge\n");
            body.push_str(&format!("moderation_appeals_pending {}\n", metrics.appeals_pending));
            body.push_str("# HELP moderation_queue_backlog Open moderation cases\n");
            body.push_str("# TYPE moderation_queue_backlog gauge\n");
            body.push_str(&format!("moderation_queue_backlog {}\n", metrics.queue_backlog));
        }
        Err(error) => {
            tracing::warn!(%error, "metrics gauge refresh failed");
        }
    }
    Ok(axum::response::Response::new(axum::body::Body::from(body)))
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
