//! Minimal Steam OpenID 2.0 client (no third-party Steam SDK).

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use url::Url;

const OPENID_NS: &str = "http://specs.openid.net/auth/2.0";
const STEAM_OPENID_ENDPOINT: &str = "https://steamcommunity.com/openid/login";
const STEAM_CLAIMED_ID_PREFIX: &str = "https://steamcommunity.com/openid/id/";
const STEAM_PLAYER_SUMMARIES: &str =
    "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/";

#[derive(Clone)]
pub struct SteamOpenIdClient {
    http: reqwest::Client,
    api_key: String,
    realm: Url,
    return_to: Url,
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

impl SteamOpenIdClient {
    pub fn new(
        api_key: String,
        realm: String,
        return_to: String,
        proxy_url: Option<String>,
        timeout_seconds: u64,
    ) -> anyhow::Result<Self> {
        if api_key.trim().is_empty() {
            bail!("STEAM_API_KEY is required");
        }
        let realm = parse_origin(&realm, "STEAM_OPENID_REALM")?;
        let return_to = Url::parse(&return_to).context("invalid STEAM_RETURN_URL")?;
        if !matches!(return_to.scheme(), "http" | "https")
            || return_to.host_str().is_none()
            || return_to.fragment().is_some()
            || return_to.query().is_some()
        {
            bail!("STEAM_RETURN_URL must be an absolute http(s) URL without query or fragment");
        }
        if origin(&realm) != origin(&return_to) {
            bail!("STEAM_RETURN_URL must use the STEAM_OPENID_REALM origin");
        }

        let mut http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(timeout_seconds.min(10)))
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .user_agent("LumiForum/0.1");
        if let Some(proxy_url) = proxy_url {
            http = http.proxy(reqwest::Proxy::all(&proxy_url).context("invalid STEAM_PROXY_URL")?);
        }

        Ok(Self {
            http: http.build()?,
            api_key: api_key.trim().to_owned(),
            realm,
            return_to,
        })
    }

    pub fn authorization_url(&self, state: &str) -> anyhow::Result<String> {
        if state.is_empty() {
            bail!("Steam state must not be empty");
        }
        let return_to = self.return_to_with_state(state);
        let mut auth = Url::parse(STEAM_OPENID_ENDPOINT)?;
        auth.query_pairs_mut()
            .append_pair("openid.ns", OPENID_NS)
            .append_pair("openid.mode", "checkid_setup")
            .append_pair(
                "openid.claimed_id",
                &format!("{OPENID_NS}/identifier_select"),
            )
            .append_pair("openid.identity", &format!("{OPENID_NS}/identifier_select"))
            .append_pair("openid.return_to", return_to.as_str())
            .append_pair("openid.realm", self.realm.as_str());
        Ok(auth.into())
    }

    pub async fn verify_callback(
        &self,
        params: &HashMap<String, String>,
        expected_state: &str,
    ) -> anyhow::Result<String> {
        require(params, "openid.mode", "id_res")?;
        require(params, "openid.ns", OPENID_NS)?;
        require(params, "openid.op_endpoint", STEAM_OPENID_ENDPOINT)?;
        let nonce = params
            .get("openid.response_nonce")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("missing openid.response_nonce"))?;
        if nonce.len() > 255 {
            bail!("invalid openid.response_nonce");
        }

        let actual_return_to = Url::parse(
            params
                .get("openid.return_to")
                .ok_or_else(|| anyhow!("missing openid.return_to"))?,
        )
        .context("invalid openid.return_to")?;
        if actual_return_to != self.return_to_with_state(expected_state) {
            bail!("openid.return_to does not match configured callback and state");
        }

        let claimed_id = params
            .get("openid.claimed_id")
            .ok_or_else(|| anyhow!("missing openid.claimed_id"))?;
        let identity = params
            .get("openid.identity")
            .ok_or_else(|| anyhow!("missing openid.identity"))?;
        if claimed_id != identity {
            bail!("openid.claimed_id and openid.identity mismatch");
        }
        let steam_id = parse_steam_id(claimed_id)?;

        let mut form: Vec<(String, String)> = params
            .iter()
            .filter(|(key, _)| key.starts_with("openid."))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if let Some((_, mode)) = form.iter_mut().find(|(key, _)| key == "openid.mode") {
            *mode = "check_authentication".into();
        }
        let response = self
            .http
            .post(STEAM_OPENID_ENDPOINT)
            .form(&form)
            .send()
            .await
            .context("steam check_authentication request failed")?
            .error_for_status()
            .context("steam check_authentication HTTP error")?
            .text()
            .await
            .context("steam check_authentication body")?;
        if !response.lines().any(|line| line.trim() == "is_valid:true") {
            bail!("steam openid assertion is not valid");
        }
        Ok(steam_id)
    }

    pub async fn fetch_profile(&self, steam_id: &str) -> anyhow::Result<SteamProfile> {
        if steam_id.len() != 17 || !steam_id.chars().all(|c| c.is_ascii_digit()) {
            bail!("invalid steam id format");
        }
        let response = self
            .http
            .get(STEAM_PLAYER_SUMMARIES)
            .query(&[("key", self.api_key.as_str()), ("steamids", steam_id)])
            .send()
            .await
            .context("steam GetPlayerSummaries request failed")?
            .error_for_status()
            .context("steam GetPlayerSummaries HTTP error")?
            .json::<PlayerSummariesResponse>()
            .await
            .context("steam GetPlayerSummaries decode")?;
        let player = response
            .response
            .players
            .into_iter()
            .find(|player| player.steamid == steam_id)
            .ok_or_else(|| anyhow!("steam profile not found"))?;
        Ok(SteamProfile {
            steam_id: player.steamid,
            persona_name: player
                .personaname
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format!("steam_{steam_id}")),
            avatar: player.avatar,
            avatar_medium: player.avatarmedium,
            avatar_full: player.avatarfull,
            profile_url: player.profileurl,
            country_code: player
                .loccountrycode
                .map(|code| code.to_ascii_uppercase())
                .filter(|code| code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase())),
        })
    }

    fn return_to_with_state(&self, state: &str) -> Url {
        let mut url = self.return_to.clone();
        url.query_pairs_mut().append_pair("state", state);
        url
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

fn origin(url: &Url) -> (String, Option<String>, Option<u16>) {
    (
        url.scheme().to_owned(),
        url.host_str().map(str::to_owned),
        url.port_or_known_default(),
    )
}

fn require(params: &HashMap<String, String>, key: &str, expected: &str) -> anyhow::Result<()> {
    if params.get(key).map(String::as_str) != Some(expected) {
        bail!("invalid {key}");
    }
    Ok(())
}

fn parse_steam_id(claimed_id: &str) -> anyhow::Result<String> {
    let id = claimed_id
        .strip_prefix(STEAM_CLAIMED_ID_PREFIX)
        .ok_or_else(|| anyhow!("invalid steam claimed_id prefix"))?;
    if id.len() != 17 || !id.chars().all(|c| c.is_ascii_digit()) {
        bail!("invalid steam id format");
    }
    Ok(id.to_owned())
}

#[derive(Debug, Deserialize)]
struct PlayerSummariesResponse {
    response: PlayerSummariesInner,
}

#[derive(Debug, Deserialize)]
struct PlayerSummariesInner {
    players: Vec<SteamPlayer>,
}

#[derive(Debug, Deserialize)]
struct SteamPlayer {
    steamid: String,
    personaname: Option<String>,
    profileurl: Option<String>,
    avatar: Option<String>,
    avatarmedium: Option<String>,
    avatarfull: Option<String>,
    loccountrycode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{parse_steam_id, SteamOpenIdClient};
    use std::collections::HashMap;

    #[test]
    fn builds_authorization_url_with_exact_state_callback() {
        let client = SteamOpenIdClient::new(
            "key".into(),
            "https://chatapi.cngokz.com".into(),
            "https://chatapi.cngokz.com/auth/steam/callback".into(),
            None,
            15,
        )
        .unwrap();
        let url = url::Url::parse(&client.authorization_url("state-value").unwrap()).unwrap();
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["openid.mode"], "checkid_setup");
        assert_eq!(
            query["openid.return_to"],
            "https://chatapi.cngokz.com/auth/steam/callback?state=state-value"
        );
    }

    #[test]
    fn rejects_invalid_origins_and_return_urls() {
        assert!(SteamOpenIdClient::new(
            "key".into(),
            "https://chatapi.cngokz.com/path".into(),
            "https://chatapi.cngokz.com/auth/steam/callback".into(),
            None,
            15,
        )
        .is_err());
        assert!(SteamOpenIdClient::new(
            "key".into(),
            "https://chatapi.cngokz.com".into(),
            "https://evil.example/auth/steam/callback".into(),
            None,
            15,
        )
        .is_err());
    }

    #[test]
    fn parses_only_valid_claimed_ids() {
        assert_eq!(
            parse_steam_id("https://steamcommunity.com/openid/id/76561198000000000").unwrap(),
            "76561198000000000"
        );
        assert!(parse_steam_id("https://example.com/id/76561198000000000").is_err());
        assert!(parse_steam_id("https://steamcommunity.com/openid/id/123").is_err());
    }
}
