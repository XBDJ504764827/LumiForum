use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::UserResponse;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessTokenClaims {
    pub sub: Uuid,
    pub role: String,
    pub auth_version: i32,
    pub jti: Uuid,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
}

/// Inbound credentials intentionally do not implement `Debug` or `Serialize`.
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub nickname: Option<String>,
}

/// `identifier` accepts either a username or an email address.
#[derive(Deserialize)]
pub struct LoginRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}
