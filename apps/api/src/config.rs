use std::net::IpAddr;

use anyhow::{bail, Context};

#[derive(Clone)]
pub struct Config {
    pub app_env: String,
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub password_hash_concurrency: usize,
    pub refresh_cookie_name: String,
    pub refresh_cookie_secure: bool,
    pub authorization_cache_ttl_seconds: u64,
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
        let jwt_issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "lumiforum-api".into());
        let jwt_audience = std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "lumiforum-web".into());
        let access_token_ttl_seconds = env_parse("ACCESS_TOKEN_TTL_SECONDS", 900_i64)?;
        let refresh_token_ttl_seconds = env_parse("REFRESH_TOKEN_TTL_SECONDS", 2_592_000_i64)?;
        let password_hash_concurrency = env_parse("PASSWORD_HASH_CONCURRENCY", 4_usize)?;
        let refresh_cookie_name =
            std::env::var("REFRESH_COOKIE_NAME").unwrap_or_else(|_| "lumiforum_refresh".into());
        let refresh_cookie_secure = env_parse("REFRESH_COOKIE_SECURE", app_env == "production")?;
        let authorization_cache_ttl_seconds = env_parse("AUTHORIZATION_CACHE_TTL_SECONDS", 30_u64)?;
        let cors_origin =
            std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());

        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET must be at least 32 bytes");
        }
        if cors_origin == "*"
            || cors_origin.ends_with('/')
            || !(cors_origin.starts_with("http://") || cors_origin.starts_with("https://"))
        {
            bail!("CORS_ORIGIN must be one explicit http(s) origin without a trailing slash");
        }

        Ok(Self {
            app_env,
            host,
            port,
            database_url,
            redis_url,
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            access_token_ttl_seconds,
            refresh_token_ttl_seconds,
            password_hash_concurrency,
            refresh_cookie_name,
            refresh_cookie_secure,
            authorization_cache_ttl_seconds,
            cors_origin,
        })
    }
}

fn env_parse<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match std::env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .with_context(|| format!("invalid {name}")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error).with_context(|| format!("invalid {name}")),
    }
}
