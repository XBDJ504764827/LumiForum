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
    pub cookie_domain: Option<String>,
    pub authorization_cache_ttl_seconds: u64,
    pub cors_origin: String,
    pub storage_provider: String,
    pub storage_local_root: String,
    pub storage_public_url: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_force_path_style: bool,
    pub s3_public_url: String,
    pub ws_max_connections_per_user: usize,
    pub ws_heartbeat_secs: u64,
    pub ws_idle_timeout_secs: u64,
    pub presence_ttl_secs: u64,
    pub ws_connect_rate_limit: u64,
    pub steam_api_key: Option<String>,
    pub steam_openid_realm: Option<String>,
    pub steam_return_url: Option<String>,
    pub steam_web_origin: Option<String>,
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
        let access_token_ttl_seconds =
            env_parse_alias("ACCESS_TOKEN_TTL_SECONDS", "JWT_EXPIRE", 900_i64)?;
        let refresh_token_ttl_seconds =
            env_parse_alias("REFRESH_TOKEN_TTL_SECONDS", "REFRESH_EXPIRE", 864_000_i64)?;
        let password_hash_concurrency = env_parse("PASSWORD_HASH_CONCURRENCY", 4_usize)?;
        let refresh_cookie_name =
            std::env::var("REFRESH_COOKIE_NAME").unwrap_or_else(|_| "lumiforum_refresh".into());
        let refresh_cookie_secure = env_parse("REFRESH_COOKIE_SECURE", app_env == "production")?;
        let cookie_domain = optional_env("COOKIE_DOMAIN");
        let authorization_cache_ttl_seconds = env_parse("AUTHORIZATION_CACHE_TTL_SECONDS", 30_u64)?;
        let cors_origin =
            std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());
        let storage_provider = std::env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "local".into());
        let storage_local_root =
            std::env::var("STORAGE_LOCAL_ROOT").unwrap_or_else(|_| "./uploads".into());
        let storage_public_url = std::env::var("STORAGE_PUBLIC_URL")
            .unwrap_or_else(|_| "http://localhost:8080/storage".into());
        let s3_endpoint = std::env::var("S3_ENDPOINT")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let s3_region = std::env::var("S3_REGION").unwrap_or_else(|_| "auto".into());
        let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "lumiforum".into());
        let s3_access_key = std::env::var("S3_ACCESS_KEY").unwrap_or_default();
        let s3_secret_key = std::env::var("S3_SECRET_KEY").unwrap_or_default();
        let s3_force_path_style = env_parse("S3_FORCE_PATH_STYLE", true)?;
        let s3_public_url = std::env::var("S3_PUBLIC_URL").unwrap_or_default();
        let ws_max_connections_per_user = env_parse("WS_MAX_CONNECTIONS_PER_USER", 5_usize)?;
        let ws_heartbeat_secs = env_parse("WS_HEARTBEAT_SECS", 30_u64)?;
        let ws_idle_timeout_secs = env_parse("WS_IDLE_TIMEOUT_SECS", 90_u64)?;
        let presence_ttl_secs = env_parse("PRESENCE_TTL_SECS", 60_u64)?;
        let ws_connect_rate_limit = env_parse("WS_CONNECT_RATE_LIMIT", 30_u64)?;
        let steam_api_key = aliased_optional_env("STEAM_API_KEY", "STEAM_WEB_API_KEY")?;
        let steam_openid_realm = optional_env("STEAM_OPENID_REALM");
        let steam_return_url = optional_env("STEAM_RETURN_URL");
        let steam_web_origin = optional_env("STEAM_WEB_ORIGIN");
        validate_steam_config(
            &app_env,
            steam_api_key.as_ref(),
            steam_openid_realm.as_ref(),
            steam_return_url.as_ref(),
            steam_web_origin.as_ref(),
        )?;

        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET must be at least 32 bytes");
        }
        if cors_origin == "*"
            || cors_origin.ends_with('/')
            || !(cors_origin.starts_with("http://") || cors_origin.starts_with("https://"))
        {
            bail!("CORS_ORIGIN must be one explicit http(s) origin without a trailing slash");
        }
        if !matches!(storage_provider.as_str(), "local" | "s3") {
            bail!("STORAGE_PROVIDER must be local or s3");
        }
        if ws_max_connections_per_user == 0 || ws_max_connections_per_user > 50 {
            bail!("WS_MAX_CONNECTIONS_PER_USER must be between 1 and 50");
        }
        if ws_heartbeat_secs == 0 || ws_idle_timeout_secs <= ws_heartbeat_secs {
            bail!("WS_IDLE_TIMEOUT_SECS must be greater than WS_HEARTBEAT_SECS");
        }
        if presence_ttl_secs < 15 {
            bail!("PRESENCE_TTL_SECS must be at least 15");
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
            cookie_domain,
            authorization_cache_ttl_seconds,
            cors_origin,
            storage_provider,
            storage_local_root,
            storage_public_url,
            s3_endpoint,
            s3_region,
            s3_bucket,
            s3_access_key,
            s3_secret_key,
            s3_force_path_style,
            s3_public_url,
            ws_max_connections_per_user,
            ws_heartbeat_secs,
            ws_idle_timeout_secs,
            presence_ttl_secs,
            ws_connect_rate_limit,
            steam_api_key,
            steam_openid_realm,
            steam_return_url,
            steam_web_origin,
        })
    }
}

fn validate_steam_config(
    app_env: &str,
    api_key: Option<&String>,
    realm: Option<&String>,
    return_url: Option<&String>,
    web_origin: Option<&String>,
) -> anyhow::Result<()> {
    let configured = [
        api_key.is_some(),
        realm.is_some(),
        return_url.is_some(),
        web_origin.is_some(),
    ];
    if configured.iter().any(|value| *value) && !configured.iter().all(|value| *value) {
        bail!("STEAM_API_KEY, STEAM_OPENID_REALM, STEAM_RETURN_URL, and STEAM_WEB_ORIGIN must be configured together");
    }
    let (Some(realm), Some(return_url), Some(web_origin)) = (realm, return_url, web_origin) else {
        return Ok(());
    };
    let realm = crate::services::parse_origin(realm, "STEAM_OPENID_REALM")?;
    let web_origin = crate::services::parse_origin(web_origin, "STEAM_WEB_ORIGIN")?;
    let return_url = url::Url::parse(return_url).context("invalid STEAM_RETURN_URL")?;
    if return_url.host_str().is_none()
        || return_url.query().is_some()
        || return_url.fragment().is_some()
    {
        bail!("STEAM_RETURN_URL must be an absolute URL without query or fragment");
    }
    if app_env == "production"
        && (realm.scheme() != "https"
            || web_origin.scheme() != "https"
            || return_url.scheme() != "https")
    {
        bail!("Steam URLs must use HTTPS in production");
    }
    Ok(())
}

fn aliased_optional_env(primary: &str, alias: &str) -> anyhow::Result<Option<String>> {
    let primary_value = optional_env(primary);
    let alias_value = optional_env(alias);
    if let (Some(left), Some(right)) = (&primary_value, &alias_value) {
        if left != right {
            bail!("{primary} and {alias} must match when both are set");
        }
    }
    Ok(primary_value.or(alias_value))
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_parse_alias<T>(primary: &str, alias: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr + PartialEq,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let primary_value = optional_env(primary)
        .map(|value| {
            value
                .parse::<T>()
                .with_context(|| format!("invalid {primary}"))
        })
        .transpose()?;
    let alias_value = optional_env(alias)
        .map(|value| {
            value
                .parse::<T>()
                .with_context(|| format!("invalid {alias}"))
        })
        .transpose()?;
    if let (Some(left), Some(right)) = (&primary_value, &alias_value) {
        if left != right {
            bail!("{primary} and {alias} must match when both are set");
        }
    }
    Ok(primary_value.or(alias_value).unwrap_or(default))
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
