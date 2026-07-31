use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

use crate::services::{
    AdminError, AuthError, AuthorizationError, CategoryError, CommentError, NotificationError,
    ReactionError, SearchError, TopicError, UploadError, UserError,
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("identity conflict")]
    IdentityConflict,
    #[error("slug conflict")]
    SlugConflict,
    #[error("category contains topics")]
    CategoryNotEmpty,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("authentication required")]
    Unauthorized,
    #[error("refresh token is invalid or expired")]
    InvalidRefreshToken,
    #[error("account unavailable")]
    AccountUnavailable,
    #[error("permission denied")]
    Forbidden,
    #[error("origin validation failed")]
    CsrfValidationFailed,
    #[error("resource not found")]
    NotFound,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("request body is too large")]
    PayloadTooLarge,
    #[error("unsupported media type")]
    UnsupportedMediaType,
    #[error("storage is unavailable")]
    StorageUnavailable,
    #[error("refresh token reuse detected")]
    RefreshTokenReused,
    #[error("Steam authentication is unavailable")]
    SteamUnavailable,
    #[error("Steam authentication failed")]
    SteamAuthenticationFailed,
    #[error("Steam account is already linked")]
    SteamAccountConflict,
    #[error("Steam is the only login method")]
    SoleLoginMethod,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Validation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                *message,
            ),
            Self::IdentityConflict => (
                StatusCode::CONFLICT,
                "identity_conflict",
                "username or email is already in use",
            ),
            Self::SlugConflict => (
                StatusCode::CONFLICT,
                "slug_conflict",
                "slug is already in use",
            ),
            Self::CategoryNotEmpty => (
                StatusCode::CONFLICT,
                "category_not_empty",
                "category contains topics",
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "invalid credentials",
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "authentication required",
            ),
            Self::InvalidRefreshToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_refresh_token",
                "refresh token is invalid or expired",
            ),
            Self::AccountUnavailable => (
                StatusCode::FORBIDDEN,
                "account_unavailable",
                "account is unavailable",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "permission_denied",
                "permission denied",
            ),
            Self::CsrfValidationFailed => (
                StatusCode::FORBIDDEN,
                "csrf_validation_failed",
                "request origin is not allowed",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "resource not found"),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests",
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "uploaded file is too large",
            ),
            Self::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "file type is not allowed",
            ),
            Self::StorageUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_unavailable",
                "file storage is temporarily unavailable",
            ),
            Self::RefreshTokenReused => {
                tracing::warn!("refresh token reuse detected; token family revoked");
                (
                    StatusCode::UNAUTHORIZED,
                    "invalid_refresh_token",
                    "refresh token is invalid or expired",
                )
            }
            Self::SteamUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "steam_unavailable",
                "Steam authentication is unavailable",
            ),
            Self::SteamAuthenticationFailed => (
                StatusCode::UNAUTHORIZED,
                "steam_auth_failed",
                "Steam authentication failed",
            ),
            Self::SteamAccountConflict => (
                StatusCode::CONFLICT,
                "steam_account_conflict",
                "Steam account is already linked",
            ),
            Self::SoleLoginMethod => (
                StatusCode::CONFLICT,
                "sole_login_method",
                "Steam is the only login method",
            ),
            Self::Internal(error) => {
                tracing::error!(error = %error, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error",
                )
            }
        };

        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

impl From<AuthError> for AppError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Validation(message) => Self::Validation(message),
            AuthError::IdentityConflict => Self::IdentityConflict,
            AuthError::InvalidCredentials => Self::InvalidCredentials,
            AuthError::AccountUnavailable => Self::AccountUnavailable,
            AuthError::InvalidRefreshToken => Self::InvalidRefreshToken,
            AuthError::RefreshTokenReused => Self::RefreshTokenReused,
            AuthError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<UserError> for AppError {
    fn from(error: UserError) -> Self {
        match error {
            UserError::EmptyUpdate => Self::Validation("profile update contains no fields"),
            UserError::Validation(message) => Self::Validation(message),
            UserError::NotFound => Self::NotFound,
            UserError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<AuthorizationError> for AppError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Unauthorized => Self::Unauthorized,
            AuthorizationError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<CategoryError> for AppError {
    fn from(error: CategoryError) -> Self {
        match error {
            CategoryError::Validation(message) => Self::Validation(message),
            CategoryError::SlugConflict => Self::SlugConflict,
            CategoryError::NotFound => Self::NotFound,
            CategoryError::NotEmpty => Self::CategoryNotEmpty,
            CategoryError::Forbidden => Self::Forbidden,
            CategoryError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<TopicError> for AppError {
    fn from(error: TopicError) -> Self {
        match error {
            TopicError::Validation(message) => Self::Validation(message),
            TopicError::NotFound | TopicError::CategoryUnavailable => Self::NotFound,
            TopicError::SlugConflict => Self::SlugConflict,
            TopicError::Forbidden => Self::Forbidden,
            TopicError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<CommentError> for AppError {
    fn from(error: CommentError) -> Self {
        match error {
            CommentError::Validation(message) => Self::Validation(message),
            CommentError::NotFound | CommentError::TopicNotFound => Self::NotFound,
            CommentError::Forbidden => Self::Forbidden,
            CommentError::RateLimited => Self::RateLimited,
            CommentError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<ReactionError> for AppError {
    fn from(error: ReactionError) -> Self {
        match error {
            ReactionError::Validation(message) => Self::Validation(message),
            ReactionError::NotFound => Self::NotFound,
            ReactionError::Forbidden => Self::Forbidden,
            ReactionError::RateLimited => Self::RateLimited,
            ReactionError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<NotificationError> for AppError {
    fn from(error: NotificationError) -> Self {
        match error {
            NotificationError::Validation(message) => Self::Validation(message),
            NotificationError::NotFound => Self::NotFound,
            NotificationError::Forbidden => Self::Forbidden,
            NotificationError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<SearchError> for AppError {
    fn from(error: SearchError) -> Self {
        match error {
            SearchError::Validation(message) => Self::Validation(message),
            SearchError::RateLimited => Self::RateLimited,
            SearchError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<AdminError> for AppError {
    fn from(error: AdminError) -> Self {
        match error {
            AdminError::Validation(message) => Self::Validation(message),
            AdminError::NotFound => Self::NotFound,
            AdminError::Forbidden => Self::Forbidden,
            AdminError::Internal(error) => Self::Internal(error),
        }
    }
}

impl From<UploadError> for AppError {
    fn from(error: UploadError) -> Self {
        match error {
            UploadError::Validation(message) => Self::Validation(message),
            UploadError::TooLarge => Self::PayloadTooLarge,
            UploadError::UnsupportedMediaType => Self::UnsupportedMediaType,
            UploadError::NotFound => Self::NotFound,
            UploadError::Forbidden => Self::Forbidden,
            UploadError::StorageUnavailable => Self::StorageUnavailable,
            UploadError::Internal(error) => Self::Internal(error),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
