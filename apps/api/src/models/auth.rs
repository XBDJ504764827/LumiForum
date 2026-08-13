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
    /// 联系方式：用于管理员追溯（QQ、手机号、微信号等）。
    pub contact: String,
}

/// 联系方式补充（Steam 登录后填写并绑定到 Steam 账户）。
#[derive(Deserialize)]
pub struct SteamContactRequest {
    pub contact: String,
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
