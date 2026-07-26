mod auth;
mod category;
mod pagination;
mod rbac;
mod refresh_token;
mod topic;
mod user;

pub use auth::{
    AccessTokenClaims, AuthResponse, LoginRequest, RegisterRequest, TokenRefreshResponse,
};
pub use category::{
    CategoryRecord, CategoryResponse, CategorySummary, CreateCategoryRequest, UpdateCategoryRequest,
};
pub use pagination::{Paginated, PaginationMeta};
pub use rbac::{
    AuthenticatedPrincipal, PermissionRecord, Principal, RolePermissionRecord, RoleRecord,
    RoleSummary, PERMISSION_CATEGORY_MANAGE, PERMISSION_PROFILE_READ_SELF,
    PERMISSION_PROFILE_UPDATE_SELF, PERMISSION_TOPIC_CREATE, PERMISSION_TOPIC_DELETE_ANY,
    PERMISSION_TOPIC_DELETE_SELF, PERMISSION_TOPIC_FEATURE, PERMISSION_TOPIC_PIN,
    PERMISSION_TOPIC_UPDATE_ANY, PERMISSION_TOPIC_UPDATE_SELF, ROLE_ADMINISTRATOR, ROLE_GUEST,
    ROLE_MODERATOR, ROLE_SUPER_ADMINISTRATOR, ROLE_USER,
};
pub use refresh_token::RefreshTokenRecord;
pub use topic::{
    CreateTopicRequest, ModerateTopicRequest, TopicAuthorSummary, TopicDetail, TopicListQuery,
    TopicListSort, TopicRecord, TopicStats, TopicStatus, TopicSummary, UpdateTopicRequest,
};
pub use user::{PatchField, ProfileUpdateRequest, UserRecord, UserResponse, UserStatus};
