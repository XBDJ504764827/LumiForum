use chrono::{Duration, Utc};
use email_address::EmailAddress;
use ipnetwork::IpNetwork;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    AuthResponse, LoginRequest, RegisterRequest, TokenRefreshResponse, UserStatus,
};
use crate::repositories::{
    repository_user_to_response, AuthRepository, RefreshRotation, RepositoryUser,
};

use super::{PasswordService, TokenService};

#[derive(Clone)]
pub struct AuthService {
    repository: AuthRepository,
    passwords: PasswordService,
    tokens: TokenService,
    refresh_token_ttl: Duration,
}

pub struct AuthServiceConfig {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub password_hash_concurrency: usize,
}

pub struct IssuedSession {
    pub response: AuthResponse,
    pub refresh_token: String,
}

pub struct RefreshedSession {
    pub response: TokenRefreshResponse,
    pub refresh_token: String,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid authentication input: {0}")]
    Validation(&'static str),
    #[error("username or email is already in use")]
    IdentityConflict,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account is unavailable")]
    AccountUnavailable,
    #[error("refresh token is invalid or expired")]
    InvalidRefreshToken,
    #[error("refresh token reuse detected")]
    RefreshTokenReused,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AuthService {
    pub fn new(repository: AuthRepository, config: AuthServiceConfig) -> anyhow::Result<Self> {
        let refresh_token_ttl = config.refresh_token_ttl_seconds;
        if !(3_600..=60 * 60 * 24 * 90).contains(&refresh_token_ttl) {
            anyhow::bail!("refresh token TTL must be between 1 hour and 90 days");
        }

        Ok(Self {
            repository,
            passwords: PasswordService::new(config.password_hash_concurrency)?,
            tokens: TokenService::new(
                config.jwt_secret,
                config.jwt_issuer,
                config.jwt_audience,
                config.access_token_ttl_seconds,
            )?,
            refresh_token_ttl: Duration::seconds(refresh_token_ttl),
        })
    }

    pub async fn register(
        &self,
        request: RegisterRequest,
        client_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<IssuedSession, AuthError> {
        let input = RegistrationInput::try_from(request)?;
        let password_hash = self.passwords.hash(input.password).await?;
        let user = self
            .repository
            .create_user(
                &input.username,
                &input.email,
                &password_hash,
                input.nickname.as_deref(),
            )
            .await
            .map_err(map_create_user_error)?;

        self.issue_session(user, client_ip, user_agent).await
    }

    pub async fn login(
        &self,
        request: LoginRequest,
        client_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<IssuedSession, AuthError> {
        let identifier = request.identifier.trim().to_owned();
        if identifier.is_empty() || request.password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        let user = self
            .repository
            .find_user_by_identifier(&identifier)
            .await
            .map_err(internal)?;

        let Some(user) = user else {
            self.passwords.consume_dummy_work(request.password).await?;
            return Err(AuthError::InvalidCredentials);
        };

        let valid = self
            .passwords
            .verify(request.password, user.password_hash.clone())
            .await?;
        if !valid {
            return Err(AuthError::InvalidCredentials);
        }
        ensure_active(&user)?;

        self.issue_session(user, client_ip, user_agent).await
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
        client_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<RefreshedSession, AuthError> {
        if refresh_token.is_empty() || refresh_token.len() > 128 {
            return Err(AuthError::InvalidRefreshToken);
        }

        let successor = TokenService::generate_refresh_token();
        let current_hash = TokenService::hash_refresh_token(refresh_token);
        let successor_hash = TokenService::hash_refresh_token(&successor);
        let (outcome, user) = self
            .repository
            .rotate_refresh_token(
                &current_hash,
                &successor_hash,
                client_ip,
                sanitized_user_agent(user_agent).as_deref(),
            )
            .await
            .map_err(internal)?;

        match outcome {
            RefreshRotation::Rotated => {}
            RefreshRotation::Replayed => return Err(AuthError::RefreshTokenReused),
            RefreshRotation::AccountUnavailable => return Err(AuthError::AccountUnavailable),
            RefreshRotation::NotFound | RefreshRotation::Expired => {
                return Err(AuthError::InvalidRefreshToken);
            }
        }

        let user = user.ok_or_else(|| internal(anyhow::anyhow!("rotated token has no user")))?;
        let (access_token, expires_in) =
            self.tokens
                .issue_access_token(user.id, &user.role_code, user.auth_version)?;

        Ok(RefreshedSession {
            response: TokenRefreshResponse {
                access_token,
                token_type: "Bearer",
                expires_in,
            },
            refresh_token: successor,
        })
    }

    pub async fn logout(&self, refresh_token: Option<&str>) -> Result<(), AuthError> {
        let Some(refresh_token) = refresh_token.filter(|token| !token.is_empty()) else {
            return Ok(());
        };
        let token_hash = TokenService::hash_refresh_token(refresh_token);
        self.repository
            .revoke_token(&token_hash, "logout")
            .await
            .map_err(internal)?;
        Ok(())
    }

    pub fn token_service(&self) -> &TokenService {
        &self.tokens
    }

    pub fn refresh_token_ttl_seconds(&self) -> i64 {
        self.refresh_token_ttl.num_seconds()
    }

    async fn issue_session(
        &self,
        user: RepositoryUser,
        client_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<IssuedSession, AuthError> {
        ensure_active(&user)?;
        let refresh_token = TokenService::generate_refresh_token();
        let refresh_hash = TokenService::hash_refresh_token(&refresh_token);
        let family_id = Uuid::new_v4();
        let expires_at = Utc::now() + self.refresh_token_ttl;
        self.repository
            .create_refresh_token(
                user.id,
                family_id,
                &refresh_hash,
                expires_at,
                client_ip,
                sanitized_user_agent(user_agent).as_deref(),
            )
            .await
            .map_err(internal)?;

        let (access_token, expires_in) =
            self.tokens
                .issue_access_token(user.id, &user.role_code, user.auth_version)?;
        let user = repository_user_to_response(user)
            .map_err(|_| internal(anyhow::anyhow!("unknown persisted user status")))?;

        Ok(IssuedSession {
            response: AuthResponse {
                access_token,
                token_type: "Bearer",
                expires_in,
                user,
            },
            refresh_token,
        })
    }
}

struct RegistrationInput {
    username: String,
    email: String,
    password: String,
    nickname: Option<String>,
}

impl TryFrom<RegisterRequest> for RegistrationInput {
    type Error = AuthError;

    fn try_from(request: RegisterRequest) -> Result<Self, Self::Error> {
        let username = request.username.trim().to_owned();
        if !valid_username(&username) {
            return Err(AuthError::Validation("invalid username"));
        }

        let email = request.email.trim().to_lowercase();
        if email.len() > 254 || !EmailAddress::is_valid(&email) {
            return Err(AuthError::Validation("invalid email"));
        }

        let password_length = request.password.chars().count();
        if !(8..=128).contains(&password_length) {
            return Err(AuthError::Validation(
                "password must contain between 8 and 128 characters",
            ));
        }

        let nickname = request
            .nickname
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if nickname
            .as_ref()
            .is_some_and(|value| value.chars().count() > 64)
        {
            return Err(AuthError::Validation("nickname is too long"));
        }

        Ok(Self {
            username,
            email,
            password: request.password,
            nickname,
        })
    }
}

fn valid_username(username: &str) -> bool {
    let mut chars = username.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (3..=32).contains(&username.len())
        && first.is_ascii_alphanumeric()
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn ensure_active(user: &RepositoryUser) -> Result<(), AuthError> {
    if user.status == UserStatus::Active.as_str() {
        Ok(())
    } else {
        Err(AuthError::AccountUnavailable)
    }
}

fn sanitized_user_agent(user_agent: Option<&str>) -> Option<String> {
    user_agent.map(|value| value.chars().take(512).collect())
}

fn map_create_user_error(error: sqlx::Error) -> AuthError {
    if error
        .as_database_error()
        .is_some_and(|database_error| database_error.code().as_deref() == Some("23505"))
    {
        AuthError::IdentityConflict
    } else {
        internal(error)
    }
}

fn internal(error: impl Into<anyhow::Error>) -> AuthError {
    AuthError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use crate::models::RegisterRequest;

    use super::{AuthError, RegistrationInput};

    #[test]
    fn normalizes_registration_input() {
        let input = RegistrationInput::try_from(RegisterRequest {
            username: "  Lumi_User  ".into(),
            email: "  USER@Example.COM ".into(),
            password: "strong-password".into(),
            nickname: Some("  Lumi  ".into()),
        })
        .expect("input is valid");

        assert_eq!(input.username, "Lumi_User");
        assert_eq!(input.email, "user@example.com");
        assert_eq!(input.nickname.as_deref(), Some("Lumi"));
    }

    #[test]
    fn rejects_invalid_registration_input() {
        let result = RegistrationInput::try_from(RegisterRequest {
            username: "invalid-name".into(),
            email: "not-an-email".into(),
            password: "short".into(),
            nickname: None,
        });

        assert!(matches!(result, Err(AuthError::Validation(_))));
    }
}
