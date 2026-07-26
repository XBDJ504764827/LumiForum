use std::net::IpAddr;

use anyhow::{bail, Context};

#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: String,
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    #[allow(dead_code)]
    pub jwt_secret: String,
    pub cors_origin: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".into())
            .parse::<IpAddr>()
            .context("invalid HOST")?;
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .context("invalid PORT")?;
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
        let redis_url = std::env::var("REDIS_URL").context("REDIS_URL is required")?;
        let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET is required")?;
        let cors_origin =
            std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());

        if jwt_secret.len() < 16 && app_env != "development" {
            bail!("JWT_SECRET must be at least 16 characters outside development");
        }

        Ok(Self {
            app_env,
            host,
            port,
            database_url,
            redis_url,
            jwt_secret,
            cors_origin,
        })
    }
}
