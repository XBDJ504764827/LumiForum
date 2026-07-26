use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub config: Config,
    pub db: PgPool,
    pub redis: ConnectionManager,
}

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(redis_client).await?;

        Ok(Self {
            inner: Arc::new(AppStateInner { config, db, redis }),
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
}
