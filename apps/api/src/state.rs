use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;
use crate::repositories::{AuthRepository, AuthorizationRepository, UserRepository};
use crate::services::{AuthService, AuthServiceConfig, AuthorizationService, UserService};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub config: Config,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub auth: AuthService,
    pub users: UserService,
    pub authorization: AuthorizationService,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&db).await?;

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(redis_client).await?;
        let auth = AuthService::new(
            AuthRepository::new(db.clone()),
            AuthServiceConfig {
                jwt_secret: config.jwt_secret.clone(),
                jwt_issuer: config.jwt_issuer.clone(),
                jwt_audience: config.jwt_audience.clone(),
                access_token_ttl_seconds: config.access_token_ttl_seconds,
                refresh_token_ttl_seconds: config.refresh_token_ttl_seconds,
                password_hash_concurrency: config.password_hash_concurrency,
            },
        )?;
        let users = UserService::new(UserRepository::new(db.clone()));
        let authorization = AuthorizationService::new(
            AuthorizationRepository::new(db.clone()),
            redis.clone(),
            config.authorization_cache_ttl_seconds,
        )?;

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                db,
                redis,
                auth,
                users,
                authorization,
            }),
        })
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn db(&self) -> &PgPool {
        &self.inner.db
    }

    pub fn redis(&self) -> &ConnectionManager {
        &self.inner.redis
    }

    pub fn auth(&self) -> &AuthService {
        &self.inner.auth
    }

    pub fn users(&self) -> &UserService {
        &self.inner.users
    }

    pub fn authorization(&self) -> &AuthorizationService {
        &self.inner.authorization
    }
}
