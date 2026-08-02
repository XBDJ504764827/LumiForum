use ipnetwork::IpNetwork;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AdminAnalytics, AdminAnalyticsQuery, AdminCommentItem, AdminCommentListQuery, AdminDashboard,
    AdminDashboardRange, AdminFileItem, AdminFileListQuery, AdminLogItem, AdminLogListQuery,
    AdminLoginRecordQuery, AdminTopicItem, AdminTopicListQuery, AdminTopicUpdateRequest,
    AdminUserDetail, AdminUserItem, AdminUserListQuery, AdminUserUpdateRequest,
    AuthenticatedPrincipal, CategoryResponse, CreateCategoryRequest, CreateReportRequest,
    LoginRecordItem, Paginated, PaginationMeta, PermissionOption, PublicSettings, QueueSummary,
    ReportItem, ReportListQuery, ReportStatus, ResolveReportRequest, RoleOption,
    RolePermissionView, SystemSettingItem, UpdateCategoryRequest, UpdateRolePermissionsRequest,
    UpdateSettingsRequest, UploadCategory, UserStatus, PERMISSION_ADMIN_ACCESS,
    PERMISSION_CATEGORY_MANAGE, PERMISSION_COMMENT_MANAGE, PERMISSION_FILE_MANAGE,
    PERMISSION_REPORT_CREATE, PERMISSION_REPORT_MANAGE, PERMISSION_SETTINGS_MANAGE,
    PERMISSION_SYSTEM_MANAGE, PERMISSION_TOPIC_MANAGE, PERMISSION_USER_MANAGE,
    PERMISSION_USER_ROLE_ASSIGN, ROLE_SUPER_ADMINISTRATOR,
};
use crate::repositories::AdminRepository;
use crate::services::{AuthorizationService, CategoryService, CommentService, UploadService};

#[derive(Clone)]
pub struct AdminService {
    repository: AdminRepository,
    categories: CategoryService,
    comments: CommentService,
    uploads: UploadService,
    authorization: AuthorizationService,
}

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("invalid admin input: {0}")]
    Validation(&'static str),
    #[error("resource not found")]
    NotFound,
    #[error("permission denied")]
    Forbidden,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct AdminAuditContext {
    pub ip: Option<IpNetwork>,
    pub user_agent: Option<String>,
}

impl AdminService {
    pub fn new(
        repository: AdminRepository,
        categories: CategoryService,
        comments: CommentService,
        uploads: UploadService,
        authorization: AuthorizationService,
    ) -> Self {
        Self {
            repository,
            categories,
            comments,
            uploads,
            authorization,
        }
    }

    pub async fn dashboard(
        &self,
        principal: &AuthenticatedPrincipal,
        range: AdminDashboardRange,
        system: crate::models::SystemStats,
    ) -> Result<AdminDashboard, AdminError> {
        require(principal, PERMISSION_SYSTEM_MANAGE)?;
        let mut dashboard = self.repository.dashboard(range).await.map_err(internal)?;
        dashboard.online_users = system.online_users;
        dashboard.api_requests_total = system.api_requests_total;
        dashboard.ws_connections = system.ws_connections;
        Ok(dashboard)
    }

    pub async fn list_users(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminUserListQuery,
    ) -> Result<Paginated<AdminUserItem>, AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let q = normalize_search(query.q)?;
        let status = query.status.map(|value| value.as_str().to_owned());
        let role = query
            .role
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let (items, total) = self
            .repository
            .list_users(
                q.as_deref(),
                status.as_deref(),
                role.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn get_user(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
    ) -> Result<AdminUserItem, AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        self.repository
            .get_user(user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)
    }

    pub async fn get_user_detail(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
    ) -> Result<AdminUserDetail, AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        self.repository
            .user_detail(user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)
    }

    pub async fn list_login_records(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
        query: AdminLoginRecordQuery,
    ) -> Result<Paginated<LoginRecordItem>, AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let (items, total) = self
            .repository
            .login_records(user_id, limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    /// Force logout: bump auth_version (kills access tokens) and revoke all
    /// refresh tokens. Cannot be applied to higher-priority roles.
    pub async fn force_logout(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        if principal.user_id == user_id {
            return Err(AdminError::Validation("不能对自己执行强制下线"));
        }
        let mut tx = self.repository.pool().begin().await.map_err(internal)?;
        let actor = self
            .repository
            .lock_user(&mut tx, principal.user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::Forbidden)?;
        let target = self
            .repository
            .lock_user(&mut tx, user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;
        if target.role_priority >= actor.role_priority {
            return Err(AdminError::Forbidden);
        }
        tx.commit().await.map_err(internal)?;

        self.repository
            .force_logout(user_id)
            .await
            .map_err(internal)?;
        self.authorization.invalidate(user_id).await;
        self.log(
            principal.user_id,
            "force_logout",
            "user",
            Some(user_id),
            &format!("force logout user {}", target.username),
            json!({ "target_username": target.username }),
            audit,
        )
        .await?;
        Ok(())
    }

    pub async fn list_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<PermissionOption>, AdminError> {
        require_any(
            principal,
            &[PERMISSION_USER_MANAGE, PERMISSION_USER_ROLE_ASSIGN],
        )?;
        self.repository.list_permissions().await.map_err(internal)
    }

    pub async fn get_role_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
        role_code: &str,
    ) -> Result<RolePermissionView, AdminError> {
        require_any(
            principal,
            &[PERMISSION_USER_MANAGE, PERMISSION_USER_ROLE_ASSIGN],
        )?;
        let role_code = role_code.trim().to_owned();
        if role_code.is_empty() {
            return Err(AdminError::Validation("role is required"));
        }
        let (code, permissions) = self
            .repository
            .role_permissions(&role_code)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;
        Ok(RolePermissionView {
            role_code: code.clone(),
            role_name: code,
            permissions,
        })
    }

    /// Replace a role's permission set. super_administrator is protected to
    /// prevent lock-out; lower-priority roles can only manage roles below them.
    pub async fn update_role_permissions(
        &self,
        principal: &AuthenticatedPrincipal,
        role_code: &str,
        request: UpdateRolePermissionsRequest,
        audit: &AdminAuditContext,
    ) -> Result<RolePermissionView, AdminError> {
        require(principal, PERMISSION_USER_ROLE_ASSIGN)?;
        let role_code = role_code.trim().to_owned();
        if role_code.is_empty() {
            return Err(AdminError::Validation("role is required"));
        }
        if role_code == ROLE_SUPER_ADMINISTRATOR {
            return Err(AdminError::Forbidden);
        }
        // Only roles below the actor's priority are manageable.
        let actor_role_priority = self
            .repository
            .role_priority(
                &mut self.repository.pool().begin().await.map_err(internal)?,
                &principal.role,
            )
            .await
            .map_err(internal)?
            .map(|(_, priority)| priority)
            .unwrap_or(i16::MAX);
        let target_priority = self
            .repository
            .role_priority(
                &mut self.repository.pool().begin().await.map_err(internal)?,
                &role_code,
            )
            .await
            .map_err(internal)?
            .map(|(_, priority)| priority)
            .ok_or(AdminError::NotFound)?;
        if target_priority >= actor_role_priority && principal.role != role_code {
            return Err(AdminError::Forbidden);
        }

        let mut codes = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for code in request.permission_codes {
            let code = code.trim().to_owned();
            if !code.is_empty() && seen.insert(code.clone()) {
                codes.push(code);
            }
        }

        let updated = self
            .repository
            .update_role_permissions(&role_code, &codes)
            .await
            .map_err(internal)?;
        if !updated {
            return Err(AdminError::NotFound);
        }

        // Invalidate cached permission snapshots for every user of the role.
        let user_ids = self
            .repository
            .user_ids_by_role(&role_code)
            .await
            .map_err(internal)?;
        for user_id in user_ids {
            self.authorization.invalidate(user_id).await;
        }

        let (code, permissions) = self
            .repository
            .role_permissions(&role_code)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;
        self.log(
            principal.user_id,
            "role.permissions.update",
            "role",
            None,
            &format!("updated permissions of role {role_code}"),
            json!({ "role_code": role_code, "permission_codes": codes }),
            audit,
        )
        .await?;
        Ok(RolePermissionView {
            role_code: code.clone(),
            role_name: code,
            permissions,
        })
    }

    pub async fn list_roles(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<RoleOption>, AdminError> {
        require_any(
            principal,
            &[PERMISSION_USER_MANAGE, PERMISSION_USER_ROLE_ASSIGN],
        )?;
        self.repository.list_roles().await.map_err(internal)
    }

    pub async fn update_user(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
        request: AdminUserUpdateRequest,
        audit: &AdminAuditContext,
    ) -> Result<AdminUserItem, AdminError> {
        require(principal, PERMISSION_USER_MANAGE)?;
        if request.status.is_none() && request.role.is_none() {
            return Err(AdminError::Validation("user update contains no fields"));
        }
        if request.role.is_some() {
            require(principal, PERMISSION_USER_ROLE_ASSIGN)?;
        }

        let mut tx = self.repository.pool().begin().await.map_err(internal)?;
        let actor = self
            .repository
            .lock_user(&mut tx, principal.user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::Forbidden)?;
        let target = self
            .repository
            .lock_user(&mut tx, user_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;

        if target.role_priority >= actor.role_priority && target.id != actor.id {
            return Err(AdminError::Forbidden);
        }

        let mut role_id = None;
        if let Some(role_code) = request.role.as_deref() {
            let role_code = role_code.trim();
            if role_code.is_empty() {
                return Err(AdminError::Validation("role is required"));
            }
            let (id, priority) = self
                .repository
                .role_priority(&mut tx, role_code)
                .await
                .map_err(internal)?
                .ok_or(AdminError::Validation("unknown role"))?;
            if priority >= actor.role_priority {
                return Err(AdminError::Forbidden);
            }
            if target.role_code == ROLE_SUPER_ADMINISTRATOR
                && role_code != ROLE_SUPER_ADMINISTRATOR
                && target.status == UserStatus::Active.as_str()
            {
                let supers = self
                    .repository
                    .count_super_admins(&mut tx)
                    .await
                    .map_err(internal)?;
                if supers <= 1 {
                    return Err(AdminError::Validation(
                        "cannot demote the last active super administrator",
                    ));
                }
            }
            role_id = Some(id);
        }

        if let Some(status) = request.status {
            if target.role_code == ROLE_SUPER_ADMINISTRATOR
                && target.status == UserStatus::Active.as_str()
                && status != UserStatus::Active
            {
                let supers = self
                    .repository
                    .count_super_admins(&mut tx)
                    .await
                    .map_err(internal)?;
                if supers <= 1 {
                    return Err(AdminError::Validation(
                        "cannot disable the last active super administrator",
                    ));
                }
            }
            if target.id == actor.id && status != UserStatus::Active {
                return Err(AdminError::Validation("cannot disable your own account"));
            }
        }

        let bump_auth = request.status.is_some() || request.role.is_some();
        let updated = self
            .repository
            .update_user(
                &mut tx,
                user_id,
                request.status.map(UserStatus::as_str),
                role_id,
                bump_auth,
            )
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;

        if bump_auth {
            self.repository
                .revoke_refresh_tokens(&mut tx, user_id)
                .await
                .map_err(internal)?;
        }

        self.repository
            .insert_log(
                Some(&mut tx),
                principal.user_id,
                "user.update",
                "user",
                Some(user_id),
                &format!("updated user {}", updated.username),
                json!({
                    "status": request.status.map(|value| value.as_str()),
                    "role": request.role,
                }),
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)?;

        tx.commit().await.map_err(internal)?;
        self.authorization.invalidate(user_id).await;
        Ok(updated)
    }

    pub async fn delete_user(
        &self,
        principal: &AuthenticatedPrincipal,
        user_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<AdminUserItem, AdminError> {
        self.update_user(
            principal,
            user_id,
            AdminUserUpdateRequest {
                status: Some(UserStatus::Disabled),
                role: None,
            },
            audit,
        )
        .await
    }

    pub async fn list_topics(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminTopicListQuery,
    ) -> Result<Paginated<AdminTopicItem>, AdminError> {
        require(principal, PERMISSION_TOPIC_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let q = normalize_search(query.q)?;
        let status = query
            .status
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if let Some(status) = status.as_deref() {
            if !matches!(status, "published" | "hidden" | "deleted") {
                return Err(AdminError::Validation("invalid topic status filter"));
            }
        }
        let sort = query
            .sort
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if let Some(sort) = sort.as_deref() {
            if !matches!(sort, "latest" | "hot" | "most_reported" | "violating") {
                return Err(AdminError::Validation("invalid topic sort"));
            }
        }
        let (items, total) = self
            .repository
            .list_topics(
                q.as_deref(),
                status.as_deref(),
                query.category_id,
                sort.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn update_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        request: AdminTopicUpdateRequest,
        audit: &AdminAuditContext,
    ) -> Result<AdminTopicItem, AdminError> {
        require(principal, PERMISSION_TOPIC_MANAGE)?;
        if request.status.is_none()
            && request.is_pinned.is_none()
            && request.is_featured.is_none()
            && request.is_locked.is_none()
        {
            return Err(AdminError::Validation("topic update contains no fields"));
        }

        let mut topic = if let Some(status) = request.status.as_deref() {
            let status = status.trim();
            if !matches!(status, "published" | "hidden" | "deleted") {
                return Err(AdminError::Validation("invalid topic status"));
            }
            self.repository
                .set_topic_status(topic_id, status)
                .await
                .map_err(internal)?
                .ok_or(AdminError::NotFound)?
        } else {
            self.repository
                .get_topic(topic_id)
                .await
                .map_err(internal)?
                .ok_or(AdminError::NotFound)?
        };

        if request.is_pinned.is_some()
            || request.is_featured.is_some()
            || request.is_locked.is_some()
        {
            topic = self
                .repository
                .set_topic_flags(
                    topic_id,
                    request.is_pinned,
                    request.is_featured,
                    request.is_locked,
                )
                .await
                .map_err(internal)?
                .ok_or(AdminError::NotFound)?;
        }

        self.log(
            principal.user_id,
            "topic.update",
            "topic",
            Some(topic_id),
            &format!("updated topic {}", topic.title),
            json!({
                "status": request.status,
                "is_pinned": request.is_pinned,
                "is_featured": request.is_featured,
                "is_locked": request.is_locked,
            }),
            audit,
        )
        .await?;
        Ok(topic)
    }

    pub async fn delete_topic(
        &self,
        principal: &AuthenticatedPrincipal,
        topic_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        self.update_topic(
            principal,
            topic_id,
            AdminTopicUpdateRequest {
                status: Some("deleted".into()),
                is_pinned: None,
                is_featured: None,
                is_locked: None,
            },
            audit,
        )
        .await?;
        Ok(())
    }

    pub async fn list_comments(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminCommentListQuery,
    ) -> Result<Paginated<AdminCommentItem>, AdminError> {
        require(principal, PERMISSION_COMMENT_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let q = normalize_search(query.q)?;
        let status = query
            .status
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let filter = query
            .filter
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if let Some(filter) = filter.as_deref() {
            if !matches!(filter, "reported" | "high_frequency") {
                return Err(AdminError::Validation("invalid comment filter"));
            }
        }
        let (items, total) = self
            .repository
            .list_comments(
                q.as_deref(),
                status.as_deref(),
                query.topic_id,
                filter.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn delete_comment(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        require(principal, PERMISSION_COMMENT_MANAGE)?;
        self.comments
            .delete(principal, comment_id)
            .await
            .map_err(map_comment)?;
        self.log(
            principal.user_id,
            "comment.delete",
            "comment",
            Some(comment_id),
            "deleted comment",
            json!({}),
            audit,
        )
        .await?;
        Ok(())
    }

    pub async fn restore_comment(
        &self,
        principal: &AuthenticatedPrincipal,
        comment_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<AdminCommentItem, AdminError> {
        require(principal, PERMISSION_COMMENT_MANAGE)?;
        self.comments
            .restore(principal, comment_id)
            .await
            .map_err(map_comment)?;
        let item = self
            .repository
            .get_comment(comment_id)
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;
        self.log(
            principal.user_id,
            "comment.restore",
            "comment",
            Some(comment_id),
            "restored comment",
            json!({}),
            audit,
        )
        .await?;
        Ok(item)
    }

    pub async fn list_categories(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<CategoryResponse>, AdminError> {
        require(principal, PERMISSION_CATEGORY_MANAGE)?;
        self.categories.list_admin().await.map_err(map_category)
    }

    pub async fn create_category(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateCategoryRequest,
        audit: &AdminAuditContext,
    ) -> Result<CategoryResponse, AdminError> {
        let category = self
            .categories
            .create(principal, request)
            .await
            .map_err(map_category)?;
        self.log(
            principal.user_id,
            "category.create",
            "category",
            Some(category.id),
            &format!("created category {}", category.name),
            json!({}),
            audit,
        )
        .await?;
        Ok(category)
    }

    pub async fn update_category(
        &self,
        principal: &AuthenticatedPrincipal,
        category_id: Uuid,
        request: UpdateCategoryRequest,
        audit: &AdminAuditContext,
    ) -> Result<CategoryResponse, AdminError> {
        let category = self
            .categories
            .update(principal, category_id, request)
            .await
            .map_err(map_category)?;
        self.log(
            principal.user_id,
            "category.update",
            "category",
            Some(category_id),
            &format!("updated category {}", category.name),
            json!({}),
            audit,
        )
        .await?;
        Ok(category)
    }

    pub async fn delete_category(
        &self,
        principal: &AuthenticatedPrincipal,
        category_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        self.categories
            .delete(principal, category_id)
            .await
            .map_err(map_category)?;
        self.log(
            principal.user_id,
            "category.delete",
            "category",
            Some(category_id),
            "deleted category",
            json!({}),
            audit,
        )
        .await?;
        Ok(())
    }

    pub async fn list_files(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminFileListQuery,
    ) -> Result<Paginated<AdminFileItem>, AdminError> {
        require(principal, PERMISSION_FILE_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let q = normalize_search(query.q)?;
        let category = query.category.map(UploadCategory::as_str);
        let status = query
            .status
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let (items, total) = self
            .repository
            .list_files(q.as_deref(), category, status.as_deref(), limit, offset)
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn delete_file(
        &self,
        principal: &AuthenticatedPrincipal,
        upload_id: Uuid,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        require(principal, PERMISSION_FILE_MANAGE)?;
        self.uploads
            .admin_delete(upload_id)
            .await
            .map_err(map_upload)?;
        self.log(
            principal.user_id,
            "file.delete",
            "upload",
            Some(upload_id),
            "deleted upload",
            json!({}),
            audit,
        )
        .await?;
        Ok(())
    }

    pub async fn cleanup_orphan_files(
        &self,
        principal: &AuthenticatedPrincipal,
        audit: &AdminAuditContext,
    ) -> Result<u32, AdminError> {
        require(principal, PERMISSION_FILE_MANAGE)?;
        let ids = self
            .repository
            .list_orphan_file_ids(100)
            .await
            .map_err(internal)?;
        let mut cleaned = 0_u32;
        for id in ids {
            if self.uploads.admin_delete(id).await.is_ok() {
                cleaned += 1;
            }
        }
        self.log(
            principal.user_id,
            "file.cleanup",
            "upload",
            None,
            &format!("cleaned {cleaned} orphan uploads"),
            json!({ "cleaned": cleaned }),
            audit,
        )
        .await?;
        Ok(cleaned)
    }

    pub async fn create_report(
        &self,
        principal: &AuthenticatedPrincipal,
        request: CreateReportRequest,
    ) -> Result<ReportItem, AdminError> {
        require(principal, PERMISSION_REPORT_CREATE)?;
        let reason = request.reason.trim().to_owned();
        if !(3..=500).contains(&reason.chars().count()) {
            return Err(AdminError::Validation(
                "reason must contain between 3 and 500 characters",
            ));
        }
        let details = request
            .details
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if details
            .as_ref()
            .is_some_and(|value| value.chars().count() > 2000)
        {
            return Err(AdminError::Validation("details are too long"));
        }
        self.repository
            .create_report(
                principal.user_id,
                request.target_type.as_str(),
                request.target_id,
                &reason,
                details.as_deref(),
            )
            .await
            .map_err(internal)
    }

    pub async fn list_reports(
        &self,
        principal: &AuthenticatedPrincipal,
        query: ReportListQuery,
    ) -> Result<Paginated<ReportItem>, AdminError> {
        require(principal, PERMISSION_REPORT_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let (items, total) = self
            .repository
            .list_reports(
                query.status.map(ReportStatus::as_str),
                query.target_type.map(|value| value.as_str()),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn resolve_report(
        &self,
        principal: &AuthenticatedPrincipal,
        report_id: Uuid,
        request: ResolveReportRequest,
        audit: &AdminAuditContext,
    ) -> Result<ReportItem, AdminError> {
        require(principal, PERMISSION_REPORT_MANAGE)?;
        if !matches!(
            request.status,
            ReportStatus::Reviewing | ReportStatus::Resolved | ReportStatus::Rejected
        ) {
            return Err(AdminError::Validation("invalid report status transition"));
        }
        let note = request
            .resolution_note
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let report = self
            .repository
            .resolve_report(
                report_id,
                principal.user_id,
                request.status.as_str(),
                note.as_deref(),
            )
            .await
            .map_err(internal)?
            .ok_or(AdminError::NotFound)?;
        self.log(
            principal.user_id,
            "report.resolve",
            "report",
            Some(report_id),
            &format!("set report status to {}", request.status.as_str()),
            json!({ "status": request.status.as_str() }),
            audit,
        )
        .await?;
        Ok(report)
    }

    pub async fn queue_summary(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<QueueSummary, AdminError> {
        require(principal, PERMISSION_ADMIN_ACCESS)?;
        self.repository.queue_summary().await.map_err(internal)
    }

    pub async fn analytics(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminAnalyticsQuery,
    ) -> Result<AdminAnalytics, AdminError> {
        require(principal, PERMISSION_SYSTEM_MANAGE)?;
        let days = query.days.unwrap_or(30);
        if !(1..=90).contains(&days) {
            return Err(AdminError::Validation("days must be between 1 and 90"));
        }
        self.repository.analytics(days).await.map_err(internal)
    }

    pub async fn list_settings(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<SystemSettingItem>, AdminError> {
        require(principal, PERMISSION_SETTINGS_MANAGE)?;
        self.repository.list_settings().await.map_err(internal)
    }

    /// Update whitelisted system settings. Value types are validated per key.
    pub async fn update_settings(
        &self,
        principal: &AuthenticatedPrincipal,
        request: UpdateSettingsRequest,
        audit: &AdminAuditContext,
    ) -> Result<Vec<SystemSettingItem>, AdminError> {
        require(principal, PERMISSION_SETTINGS_MANAGE)?;
        const WHITELIST: [&str; 7] = [
            "site_name",
            "site_description",
            "registration_enabled",
            "topic_create_enabled",
            "comment_enabled",
            "upload_enabled",
            "upload_max_bytes",
        ];
        let mut updates = Vec::new();
        for setting in request.settings {
            let key = setting.key.trim().to_owned();
            if !WHITELIST.contains(&key.as_str()) {
                return Err(AdminError::Validation("unknown setting key"));
            }
            let valid = match key.as_str() {
                "site_name" => setting
                    .value
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty() && value.chars().count() <= 100),
                "site_description" => setting
                    .value
                    .as_str()
                    .is_some_and(|value| value.chars().count() <= 500),
                "upload_max_bytes" => setting
                    .value
                    .as_i64()
                    .is_some_and(|value| (1024..=100 * 1024 * 1024).contains(&value)),
                _ => setting.value.is_boolean(),
            };
            if !valid {
                return Err(AdminError::Validation("invalid setting value type"));
            }
            updates.push((key, setting.value));
        }
        if updates.is_empty() {
            return Err(AdminError::Validation("settings update is empty"));
        }
        self.repository
            .upsert_settings(principal.user_id, &updates)
            .await
            .map_err(internal)?;
        self.log(
            principal.user_id,
            "settings.update",
            "system",
            None,
            "updated system settings",
            json!({
                "keys": updates.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(),
            }),
            audit,
        )
        .await?;
        self.repository.list_settings().await.map_err(internal)
    }

    /// Public settings snapshot (used by the forum frontend; Redis-cached).
    pub async fn public_settings(&self) -> Result<PublicSettings, AdminError> {
        self.repository.public_settings().await.map_err(internal)
    }

    pub async fn list_logs(
        &self,
        principal: &AuthenticatedPrincipal,
        query: AdminLogListQuery,
    ) -> Result<Paginated<AdminLogItem>, AdminError> {
        require(principal, PERMISSION_SYSTEM_MANAGE)?;
        let (page, page_size, limit, offset) = page_bounds(query.page, query.page_size);
        let q = normalize_search(query.q)?;
        let action = query
            .action
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let (items, total) = self
            .repository
            .list_logs(
                q.as_deref(),
                action.as_deref(),
                query.target_type.as_deref(),
                limit,
                offset,
            )
            .await
            .map_err(internal)?;
        Ok(paginate(items, page, page_size, total))
    }

    pub async fn touch_last_login(&self, user_id: Uuid) {
        if let Err(error) = self.repository.touch_last_login(user_id).await {
            tracing::warn!(%error, %user_id, "failed to update last_login_at");
        }
    }

    async fn log(
        &self,
        admin_id: Uuid,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        summary: &str,
        metadata: serde_json::Value,
        audit: &AdminAuditContext,
    ) -> Result<(), AdminError> {
        self.repository
            .insert_log(
                None,
                admin_id,
                action,
                target_type,
                target_id,
                summary,
                metadata,
                audit.ip,
                audit.user_agent.as_deref(),
            )
            .await
            .map_err(internal)
    }
}

fn require(principal: &AuthenticatedPrincipal, permission: &str) -> Result<(), AdminError> {
    if principal.has_permission(PERMISSION_ADMIN_ACCESS) && principal.has_permission(permission) {
        Ok(())
    } else {
        Err(AdminError::Forbidden)
    }
}

fn require_any(principal: &AuthenticatedPrincipal, permissions: &[&str]) -> Result<(), AdminError> {
    if !principal.has_permission(PERMISSION_ADMIN_ACCESS) {
        return Err(AdminError::Forbidden);
    }
    if permissions
        .iter()
        .any(|permission| principal.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AdminError::Forbidden)
    }
}

fn page_bounds(page: Option<u32>, page_size: Option<u32>) -> (u32, u32, i64, i64) {
    let page = page.unwrap_or(1).clamp(1, 1_000_000);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = i64::from((page - 1).saturating_mul(page_size));
    (page, page_size, i64::from(page_size), offset)
}

fn paginate<T>(items: Vec<T>, page: u32, page_size: u32, total: i64) -> Paginated<T> {
    Paginated {
        items,
        pagination: PaginationMeta::new(page, page_size, u64::try_from(total.max(0)).unwrap_or(0)),
    }
}

fn normalize_search(value: Option<String>) -> Result<Option<String>, AdminError> {
    let value = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > 100)
    {
        return Err(AdminError::Validation("search query is too long"));
    }
    Ok(value)
}

fn internal(error: impl Into<anyhow::Error>) -> AdminError {
    AdminError::Internal(error.into())
}

fn map_category(error: crate::services::CategoryError) -> AdminError {
    match error {
        crate::services::CategoryError::Validation(message) => AdminError::Validation(message),
        crate::services::CategoryError::NotFound => AdminError::NotFound,
        crate::services::CategoryError::Forbidden => AdminError::Forbidden,
        crate::services::CategoryError::NotEmpty => {
            AdminError::Validation("category contains topics")
        }
        crate::services::CategoryError::SlugConflict => {
            AdminError::Validation("category slug is already in use")
        }
        crate::services::CategoryError::Internal(error) => AdminError::Internal(error),
    }
}

fn map_comment(error: crate::services::CommentError) -> AdminError {
    match error {
        crate::services::CommentError::Validation(message) => AdminError::Validation(message),
        crate::services::CommentError::NotFound | crate::services::CommentError::TopicNotFound => {
            AdminError::NotFound
        }
        crate::services::CommentError::Forbidden => AdminError::Forbidden,
        crate::services::CommentError::RateLimited => AdminError::Validation("rate limited"),
        crate::services::CommentError::Internal(error) => AdminError::Internal(error),
    }
}

fn map_upload(error: crate::services::UploadError) -> AdminError {
    match error {
        crate::services::UploadError::Validation(message) => AdminError::Validation(message),
        crate::services::UploadError::NotFound => AdminError::NotFound,
        crate::services::UploadError::Forbidden => AdminError::Forbidden,
        crate::services::UploadError::TooLarge => AdminError::Validation("file is too large"),
        crate::services::UploadError::UnsupportedMediaType => {
            AdminError::Validation("unsupported media type")
        }
        crate::services::UploadError::StorageUnavailable => {
            AdminError::Validation("storage is unavailable")
        }
        crate::services::UploadError::Internal(error) => AdminError::Internal(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{page_bounds, require};
    use crate::models::{AuthenticatedPrincipal, PERMISSION_ADMIN_ACCESS, PERMISSION_USER_MANAGE};
    use uuid::Uuid;

    #[test]
    fn admin_access_requires_both_permissions() {
        let principal = AuthenticatedPrincipal::new(
            Uuid::new_v4(),
            "administrator".into(),
            0,
            Uuid::new_v4(),
            [PERMISSION_ADMIN_ACCESS.to_owned()],
        );
        assert!(require(&principal, PERMISSION_USER_MANAGE).is_err());
    }

    #[test]
    fn page_bounds_are_clamped() {
        let (page, size, limit, offset) = page_bounds(Some(0), Some(500));
        assert_eq!((page, size, limit, offset), (1, 100, 100, 0));
    }
}
