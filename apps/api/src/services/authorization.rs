use redis::{aio::ConnectionManager, AsyncCommands};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{AccessTokenClaims, AuthenticatedPrincipal, UserStatus};
use crate::repositories::{AuthorizationRepository, AuthorizationSnapshot};

#[derive(Clone)]
pub struct AuthorizationService {
    repository: AuthorizationRepository,
    redis: ConnectionManager,
    cache_ttl_seconds: u64,
}

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("authentication is invalid")]
    Unauthorized,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AuthorizationService {
    pub fn new(
        repository: AuthorizationRepository,
        redis: ConnectionManager,
        cache_ttl_seconds: u64,
    ) -> anyhow::Result<Self> {
        if !(1..=300).contains(&cache_ttl_seconds) {
            anyhow::bail!("authorization cache TTL must be between 1 and 300 seconds");
        }
        Ok(Self {
            repository,
            redis,
            cache_ttl_seconds,
        })
    }

    pub async fn authenticate(
        &self,
        claims: AccessTokenClaims,
    ) -> Result<AuthenticatedPrincipal, AuthorizationError> {
        let snapshot = self.snapshot(claims.sub).await?;
        if snapshot.status != UserStatus::Active.as_str()
            || snapshot.auth_version != claims.auth_version
            || snapshot.role != claims.role
        {
            return Err(AuthorizationError::Unauthorized);
        }

        Ok(AuthenticatedPrincipal::new(
            claims.sub,
            snapshot.role,
            snapshot.auth_version,
            claims.jti,
            snapshot.permissions,
        ))
    }

    pub async fn invalidate(&self, user_id: Uuid) {
        let key = cache_key(user_id);
        let mut redis = self.redis.clone();
        if let Err(error) = redis.del::<_, ()>(&key).await {
            tracing::warn!(%error, %user_id, "failed to invalidate authorization cache");
        }
    }

    async fn snapshot(&self, user_id: Uuid) -> Result<AuthorizationSnapshot, AuthorizationError> {
        let key = cache_key(user_id);
        let mut redis = self.redis.clone();
        match redis.get::<_, Option<String>>(&key).await {
            Ok(Some(value)) => match serde_json::from_str(&value) {
                Ok(snapshot) => return Ok(snapshot),
                Err(error) => {
                    tracing::warn!(%error, %user_id, "invalid authorization cache entry");
                }
            },
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, %user_id, "authorization cache unavailable; using database");
            }
        }

        let snapshot = self
            .repository
            .find_snapshot(user_id)
            .await
            .map_err(|error| AuthorizationError::Internal(error.into()))?
            .ok_or(AuthorizationError::Unauthorized)?;

        if let Ok(value) = serde_json::to_string(&snapshot) {
            if let Err(error) = redis
                .set_ex::<_, _, ()>(&key, value, self.cache_ttl_seconds)
                .await
            {
                tracing::warn!(%error, %user_id, "failed to cache authorization snapshot");
            }
        }

        Ok(snapshot)
    }
}

fn cache_key(user_id: Uuid) -> String {
    format!("authz:user:{user_id}")
}
