use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ipnetwork::IpNetwork;
use rand::RngCore;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::models::UserResponse;
use crate::repositories::{
    is_unique_violation, repository_user_to_response, RepositoryUser, SteamAuthRepository,
};

use super::{AuthService, IssuedSession, PasswordService, SteamOpenIdClient};

const STATE_TTL_SECONDS: u64 = 600;
const STATE_VERSION: u8 = 1;

#[derive(Clone)]
pub struct SteamAuthService {
    repository: SteamAuthRepository,
    auth: AuthService,
    passwords: PasswordService,
    state: SteamStateStore,
    client: SteamOpenIdClient,
}

#[derive(Clone)]
struct SteamStateStore {
    redis: ConnectionManager,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SteamAuthMode {
    Login,
    Bind,
}

#[derive(Debug, Deserialize, Serialize)]
struct SteamState {
    version: u8,
    mode: SteamAuthMode,
    user_id: Option<Uuid>,
}

pub struct SteamAuthorization {
    pub authorization_url: String,
    pub state: String,
}

pub enum SteamCallbackResult {
    Login(IssuedSession),
    Bound(UserResponse),
}

#[derive(Debug, Error)]
pub enum SteamAuthError {
    #[error("Steam authentication is not configured")]
    Unavailable,
    #[error("Steam authentication state is invalid or expired")]
    InvalidState,
    #[error("Steam authentication failed")]
    AuthenticationFailed,
    #[error("Steam account is already linked")]
    AccountConflict,
    #[error("Steam account is not linked")]
    NotLinked,
    #[error("current password is invalid")]
    InvalidPassword,
    #[error("Steam is the only login method")]
    SoleLoginMethod,
    #[error("account is unavailable")]
    AccountUnavailable,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl SteamAuthService {
    pub fn new(
        repository: SteamAuthRepository,
        auth: AuthService,
        password_hash_concurrency: usize,
        redis: ConnectionManager,
        client: SteamOpenIdClient,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repository,
            auth,
            passwords: PasswordService::new(password_hash_concurrency)?,
            state: SteamStateStore { redis },
            client,
        })
    }

    pub async fn start_login(&self) -> Result<SteamAuthorization, SteamAuthError> {
        self.start(SteamAuthMode::Login, None).await
    }

    pub async fn start_bind(&self, user_id: Uuid) -> Result<SteamAuthorization, SteamAuthError> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await
            .map_err(internal)?
            .ok_or(SteamAuthError::AccountUnavailable)?;
        ensure_active(&user)?;
        if user.steam_id.is_some() {
            return Err(SteamAuthError::AccountConflict);
        }
        self.start(SteamAuthMode::Bind, Some(user_id)).await
    }

    pub async fn complete(
        &self,
        params: &HashMap<String, String>,
        client_ip: Option<IpNetwork>,
        user_agent: Option<&str>,
    ) -> Result<SteamCallbackResult, SteamAuthError> {
        let state_token = params.get("state").ok_or(SteamAuthError::InvalidState)?;
        let transaction = self.state.consume(state_token).await?;
        let steam_id = self
            .client
            .verify_callback(params, state_token)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Steam OpenID assertion rejected");
                SteamAuthError::AuthenticationFailed
            })?;
        let profile = self
            .client
            .fetch_profile(&steam_id)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Steam profile request failed");
                SteamAuthError::AuthenticationFailed
            })?;

        match transaction.mode {
            SteamAuthMode::Login => {
                let user = self.find_or_create_user(&profile).await?;
                ensure_active(&user)?;
                let session = self
                    .auth
                    .issue_session(user, client_ip, user_agent)
                    .await
                    .map_err(|error| SteamAuthError::Internal(anyhow::Error::new(error)))?;
                Ok(SteamCallbackResult::Login(session))
            }
            SteamAuthMode::Bind => {
                let user_id = transaction.user_id.ok_or(SteamAuthError::InvalidState)?;
                if let Some(existing) = self
                    .repository
                    .find_by_steam_id(&profile.steam_id)
                    .await
                    .map_err(internal)?
                {
                    if existing.id != user_id {
                        return Err(SteamAuthError::AccountConflict);
                    }
                    return Ok(SteamCallbackResult::Bound(to_response(existing)?));
                }
                let user = self
                    .repository
                    .bind(user_id, &profile)
                    .await
                    .map_err(|error| {
                        if is_unique_violation(&error) {
                            SteamAuthError::AccountConflict
                        } else {
                            internal(error)
                        }
                    })?;
                let user = user.ok_or(SteamAuthError::AccountConflict)?;
                Ok(SteamCallbackResult::Bound(to_response(user)?))
            }
        }
    }

    pub async fn unbind(
        &self,
        user_id: Uuid,
        password: String,
    ) -> Result<UserResponse, SteamAuthError> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await
            .map_err(internal)?
            .ok_or(SteamAuthError::NotLinked)?;
        if user.steam_id.is_none() {
            return Err(SteamAuthError::NotLinked);
        }
        let hash = user
            .password_hash
            .clone()
            .ok_or(SteamAuthError::SoleLoginMethod)?;
        if password.is_empty()
            || !self
                .passwords
                .verify(password, hash)
                .await
                .map_err(SteamAuthError::Internal)?
        {
            return Err(SteamAuthError::InvalidPassword);
        }
        let updated = self
            .repository
            .unbind(user_id)
            .await
            .map_err(internal)?
            .ok_or(SteamAuthError::SoleLoginMethod)?;
        to_response(updated)
    }

    pub async fn sync(&self, user_id: Uuid) -> Result<UserResponse, SteamAuthError> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await
            .map_err(internal)?
            .ok_or(SteamAuthError::NotLinked)?;
        ensure_active(&user)?;
        let steam_id = user.steam_id.ok_or(SteamAuthError::NotLinked)?;
        let profile = self
            .client
            .fetch_profile(&steam_id)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Steam profile sync failed");
                SteamAuthError::AuthenticationFailed
            })?;
        let updated = self
            .repository
            .sync_profile(user_id, &profile)
            .await
            .map_err(internal)?
            .ok_or(SteamAuthError::NotLinked)?;
        to_response(updated)
    }

    async fn start(
        &self,
        mode: SteamAuthMode,
        user_id: Option<Uuid>,
    ) -> Result<SteamAuthorization, SteamAuthError> {
        let state = self.state.create(mode, user_id).await?;
        let authorization_url = self
            .client
            .authorization_url(&state)
            .map_err(SteamAuthError::Internal)?;
        Ok(SteamAuthorization {
            authorization_url,
            state,
        })
    }

    async fn find_or_create_user(
        &self,
        profile: &super::SteamProfile,
    ) -> Result<RepositoryUser, SteamAuthError> {
        if let Some(user) = self
            .repository
            .find_by_steam_id(&profile.steam_id)
            .await
            .map_err(internal)?
        {
            return self
                .repository
                .sync_profile(user.id, profile)
                .await
                .map_err(internal)?
                .ok_or(SteamAuthError::AccountUnavailable);
        }

        let email = format!("steam_{}@steam.local", profile.steam_id);
        match self.repository.create_steam_user(&email, profile).await {
            Ok(user) => Ok(user),
            Err(error) if is_unique_violation(&error) => self
                .repository
                .find_by_steam_id(&profile.steam_id)
                .await
                .map_err(internal)?
                .ok_or(SteamAuthError::AccountConflict),
            Err(error) => Err(internal(error)),
        }
    }
}

impl SteamStateStore {
    async fn create(
        &self,
        mode: SteamAuthMode,
        user_id: Option<Uuid>,
    ) -> Result<String, SteamAuthError> {
        for _ in 0..3 {
            let mut random = [0_u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut random);
            let token = URL_SAFE_NO_PAD.encode(random);
            let key = state_key(&token);
            let value = serde_json::to_string(&SteamState {
                version: STATE_VERSION,
                mode,
                user_id,
            })
            .map_err(|error| SteamAuthError::Internal(error.into()))?;
            let mut connection = self.redis.clone();
            let result: Option<String> = redis::cmd("SET")
                .arg(&key)
                .arg(value)
                .arg("NX")
                .arg("EX")
                .arg(STATE_TTL_SECONDS)
                .query_async(&mut connection)
                .await
                .map_err(|error| SteamAuthError::Internal(error.into()))?;
            if result.as_deref() == Some("OK") {
                return Ok(token);
            }
        }
        Err(SteamAuthError::Internal(anyhow::anyhow!(
            "failed to allocate Steam authentication state"
        )))
    }

    async fn consume(&self, token: &str) -> Result<SteamState, SteamAuthError> {
        if token.len() < 32 || token.len() > 128 {
            return Err(SteamAuthError::InvalidState);
        }
        let mut connection = self.redis.clone();
        let value: Option<String> = redis::cmd("GETDEL")
            .arg(state_key(token))
            .query_async(&mut connection)
            .await
            .map_err(|error| SteamAuthError::Internal(error.into()))?;
        let state: SteamState =
            serde_json::from_str(value.as_deref().ok_or(SteamAuthError::InvalidState)?)
                .map_err(|_| SteamAuthError::InvalidState)?;
        if state.version != STATE_VERSION
            || (state.mode == SteamAuthMode::Login && state.user_id.is_some())
            || (state.mode == SteamAuthMode::Bind && state.user_id.is_none())
        {
            return Err(SteamAuthError::InvalidState);
        }
        Ok(state)
    }
}

fn state_key(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("auth:steam:state:{}", URL_SAFE_NO_PAD.encode(digest))
}

fn ensure_active(user: &RepositoryUser) -> Result<(), SteamAuthError> {
    if user.status == "active" {
        Ok(())
    } else {
        Err(SteamAuthError::AccountUnavailable)
    }
}

fn to_response(user: RepositoryUser) -> Result<UserResponse, SteamAuthError> {
    repository_user_to_response(user)
        .map_err(|message| SteamAuthError::Internal(anyhow::anyhow!(message)))
}

fn internal(error: sqlx::Error) -> SteamAuthError {
    SteamAuthError::Internal(error.into())
}

#[cfg(test)]
mod tests {
    use super::state_key;

    #[test]
    fn steam_id_is_a_valid_forum_username() {
        let steam_id = "76561198000000000";
        assert!((3..=32).contains(&steam_id.len()));
        assert!(steam_id.chars().all(|character| character.is_ascii_digit()));
    }

    #[test]
    fn state_keys_do_not_contain_raw_tokens() {
        let key = state_key("sensitive-state-token-value");
        assert!(key.starts_with("auth:steam:state:"));
        assert!(!key.contains("sensitive-state-token-value"));
    }
}
