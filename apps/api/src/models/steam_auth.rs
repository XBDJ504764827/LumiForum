use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SteamAuthorizationResponse {
    pub authorization_url: String,
}

#[derive(Deserialize)]
pub struct SteamUnbindRequest {
    pub password: String,
}
