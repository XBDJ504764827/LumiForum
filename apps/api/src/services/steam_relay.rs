//! Steam relay client — 论坛服务器无法直连 Steam 网络，所有 Steam 交互均
//! 通过 Cloudflare Worker 中继（见 scripts/deploy/steam-auth-worker.js）：
//!   - GET {relay}/login?mode=&state=&return_to=  发起 Steam OpenID 登录
//!   - GET {relay}/verify?token=                  一次性 token 换取 SteamID 与资料
//!   - GET {relay}/profile?steamid=               按 SteamID 同步资料
//!
//! 一次性 token 由 Worker 生成并单次消费，回调中携带的 token 无法被
//! 伪造为任意 SteamID，避免 `?steamid=xxx` 直接伪造登录。

use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use url::Url;

#[derive(Clone)]
pub struct SteamRelayClient {
    http: reqwest::Client,
    relay: Url,
    callback_url: Url,
}

#[derive(Clone, Debug)]
pub struct SteamProfile {
    pub steam_id: String,
    pub persona_name: String,
    pub avatar: Option<String>,
    pub avatar_medium: Option<String>,
    pub avatar_full: Option<String>,
    pub profile_url: Option<String>,
    pub country_code: Option<String>,
}

/// Worker `/verify` 与 `/profile` 的响应体（字段均可选，缺省时回退默认值）。
#[derive(Debug, Deserialize)]
struct RelayProfileResponse {
    success: bool,
    steamid: Option<String>,
    persona_name: Option<String>,
    avatar: Option<String>,
    avatar_medium: Option<String>,
    avatar_full: Option<String>,
    profile_url: Option<String>,
    country_code: Option<String>,
}

impl RelayProfileResponse {
    fn into_profile(self, steam_id: String) -> SteamProfile {
        SteamProfile {
            persona_name: self
                .persona_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format!("steam_{steam_id}")),
            avatar: self.avatar,
            avatar_medium: self.avatar_medium,
            avatar_full: self.avatar_full,
            profile_url: self.profile_url,
            country_code: self
                .country_code
                .map(|code| code.to_ascii_uppercase())
                .filter(|code| code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())),
            steam_id,
        }
    }
}

impl SteamRelayClient {
    pub fn new(
        relay_url: String,
        callback_url: String,
        timeout_seconds: u64,
    ) -> anyhow::Result<Self> {
        let relay = parse_origin(&relay_url, "STEAM_RELAY_URL")?;
        let callback_url = Url::parse(&callback_url).context("invalid STEAM_CALLBACK_URL")?;
        if !matches!(callback_url.scheme(), "http" | "https")
            || callback_url.host_str().is_none()
            || callback_url.fragment().is_some()
            || callback_url.query().is_some()
        {
            bail!("STEAM_CALLBACK_URL must be an absolute http(s) URL without query or fragment");
        }

        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(timeout_seconds.min(10)))
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .user_agent("LumiForum/0.1")
            .build()?;

        Ok(Self {
            http,
            relay,
            callback_url,
        })
    }

    /// 生成中继登录地址：`{relay}/login?mode=..&state=..&return_to={callback}`。
    /// Worker 完成 Steam 验证后会携带一次性 token 跳回 `callback_url`。
    pub fn authorization_url(&self, mode: &str, state: &str) -> anyhow::Result<Url> {
        if state.is_empty() || state.len() > 128 {
            bail!("Steam state must be between 1 and 128 characters");
        }
        if !matches!(mode, "login" | "bind") {
            bail!("invalid steam auth mode");
        }
        let mut url = self.relay.join("/login")?;
        url.query_pairs_mut()
            .append_pair("mode", mode)
            .append_pair("state", state)
            .append_pair("return_to", self.callback_url.as_str());
        Ok(url)
    }

    /// 用一次性 token 向中继换取 SteamID 与资料（token 单次有效，防伪造登录）。
    pub async fn verify_token(&self, token: &str) -> anyhow::Result<SteamProfile> {
        if token.is_empty() || token.len() > 128 {
            bail!("invalid steam relay token");
        }
        let url = self.relay.join("/verify")?;
        let response = self
            .http
            .get(url)
            .query(&[("token", token)])
            .send()
            .await
            .context("steam relay verify request failed")?
            .error_for_status()
            .context("steam relay verify HTTP error")?
            .json::<RelayProfileResponse>()
            .await
            .context("steam relay verify decode")?;
        if !response.success {
            bail!("steam relay rejected the one-time token");
        }
        let steam_id = response
            .steamid
            .clone()
            .ok_or_else(|| anyhow!("steam relay response is missing steamid"))?;
        validate_steam_id(&steam_id)?;
        Ok(response.into_profile(steam_id))
    }

    /// 按 SteamID 同步资料（登录后刷新昵称/头像）。
    pub async fn fetch_profile(&self, steam_id: &str) -> anyhow::Result<SteamProfile> {
        validate_steam_id(steam_id)?;
        let url = self.relay.join("/profile")?;
        let response = self
            .http
            .get(url)
            .query(&[("steamid", steam_id)])
            .send()
            .await
            .context("steam relay profile request failed")?
            .error_for_status()
            .context("steam relay profile HTTP error")?
            .json::<RelayProfileResponse>()
            .await
            .context("steam relay profile decode")?;
        if !response.success {
            bail!("steam relay profile lookup failed");
        }
        Ok(response.into_profile(steam_id.to_owned()))
    }
}

pub fn parse_origin(value: &str, name: &str) -> anyhow::Result<Url> {
    let url = Url::parse(value).with_context(|| format!("invalid {name}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        bail!("{name} must be one absolute http(s) origin");
    }
    Ok(url)
}

fn validate_steam_id(steam_id: &str) -> anyhow::Result<()> {
    if steam_id.len() != 17 || !steam_id.chars().all(|c| c.is_ascii_digit()) {
        bail!("invalid steam id format");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_origin, SteamRelayClient};
    use std::collections::HashMap;

    #[test]
    fn builds_authorization_url_against_the_relay() {
        let client = SteamRelayClient::new(
            "https://cngokz-steam-auth.iquankz.cn".into(),
            "https://chatapi.cngokz.com/auth/steam/callback".into(),
            15,
        )
        .unwrap();
        let url = client.authorization_url("bind", "state-value").unwrap();
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(
            url.origin().ascii_serialization(),
            "https://cngokz-steam-auth.iquankz.cn"
        );
        assert_eq!(url.path(), "/login");
        assert_eq!(query["mode"], "bind");
        assert_eq!(query["state"], "state-value");
        assert_eq!(
            query["return_to"],
            "https://chatapi.cngokz.com/auth/steam/callback"
        );
    }

    #[test]
    fn rejects_invalid_relay_and_callback_urls() {
        assert!(SteamRelayClient::new(
            "https://cngokz-steam-auth.iquankz.cn/path".into(),
            "https://chatapi.cngokz.com/auth/steam/callback".into(),
            15,
        )
        .is_err());
        assert!(SteamRelayClient::new(
            "https://cngokz-steam-auth.iquankz.cn".into(),
            "https://chatapi.cngokz.com/auth/steam/callback?state=x".into(),
            15,
        )
        .is_err());
        assert!(SteamRelayClient::new(
            "https://cngokz-steam-auth.iquankz.cn".into(),
            "ftp://chatapi.cngokz.com/auth/steam/callback".into(),
            15,
        )
        .is_err());
    }

    #[test]
    fn parses_only_valid_origins() {
        assert!(parse_origin("https://example.com", "test").is_ok());
        assert!(parse_origin("https://example.com/", "test").is_ok());
        assert!(parse_origin("https://example.com/path", "test").is_err());
        assert!(parse_origin("https://user@example.com", "test").is_err());
    }
}
