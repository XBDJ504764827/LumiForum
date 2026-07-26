mod auth;
mod rbac;
mod refresh_token;
mod user;

pub use auth::{
    AccessTokenClaims, AuthResponse, LoginRequest, RegisterRequest, TokenRefreshResponse,
};
pub use rbac::{
    AuthenticatedPrincipal, PermissionRecord, Principal, RolePermissionRecord, RoleRecord,
    RoleSummary, PERMISSION_PROFILE_READ_SELF, PERMISSION_PROFILE_UPDATE_SELF, ROLE_ADMINISTRATOR,
    ROLE_GUEST, ROLE_MODERATOR, ROLE_SUPER_ADMINISTRATOR, ROLE_USER,
};
pub use refresh_token::RefreshTokenRecord;
pub use user::{PatchField, ProfileUpdateRequest, UserRecord, UserResponse, UserStatus};
