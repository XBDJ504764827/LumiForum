use std::{collections::HashMap, net::SocketAddr};

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header::USER_AGENT, HeaderMap},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use ipnetwork::IpNetwork;

use crate::error::{AppError, AppResult};
use crate::middleware::{enforce_origin, require_permission, AuthorizationLayer, CsrfLayer};
use crate::models::{
    AuthenticatedPrincipal, SteamAuthorizationResponse, SteamUnbindRequest, UserResponse,
    PERMISSION_PROFILE_UPDATE_SELF,
};
use crate::services::{SteamAuthError, SteamCallbackResult};
use crate::state::AppState;

use super::{auth::refresh_cookie, response::ApiResponse};

pub fn public_router(state: AppState) -> Router<AppState> {
    let start = Router::new()
        .route("/auth/steam/login", get(login_start))
        .route_layer(middleware::from_fn_with_state(
            CsrfLayer::new(state.config().cors_origin.clone()),
            optional_origin,
        ));
    start.route("/auth/steam/callback", get(callback))
}

pub fn protected_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/steam/bind", post(bind_start))
        .route("/auth/steam/unbind", delete(unbind))
        .route("/auth/steam/sync", post(sync))
        .route_layer(middleware::from_fn_with_state(
            CsrfLayer::new(state.config().cors_origin.clone()),
            enforce_origin,
        ))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_PROFILE_UPDATE_SELF),
            require_permission,
        ))
}

async fn optional_origin(
    State(layer): State<CsrfLayer>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> AppResult<Response> {
    if request.headers().contains_key(axum::http::header::ORIGIN) {
        enforce_origin(State(layer), request, next).await
    } else {
        Ok(next.run(request).await)
    }
}

async fn login_start(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Redirect)> {
    let authorization = steam_service(&state)?.start_login().await?;
    let jar = jar.add(state_cookie(&state, authorization.state));
    Ok((jar, Redirect::temporary(&authorization.authorization_url)))
}

async fn bind_start(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<ApiResponse<SteamAuthorizationResponse>>)> {
    let authorization = steam_service(&state)?.start_bind(principal.user_id).await?;
    let jar = jar.add(state_cookie(&state, authorization.state));
    Ok((
        jar,
        Json(ApiResponse::new(SteamAuthorizationResponse {
            authorization_url: authorization.authorization_url,
        })),
    ))
}

async fn callback(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let query_state = params.get("state").map(String::as_str);
    let cookie_state = jar
        .get("lumiforum_steam_state")
        .map(Cookie::value)
        .map(str::to_owned);
    let jar = jar.remove(state_removal_cookie(&state));
    if query_state.is_none() || query_state != cookie_state.as_deref() {
        return (jar, callback_redirect(&state, "steam_invalid_state")).into_response();
    }
    if params.get("openid.mode").map(String::as_str) == Some("cancel") {
        return (jar, callback_redirect(&state, "steam_access_denied")).into_response();
    }
    let Some(service) = state.steam_auth() else {
        return (jar, callback_redirect(&state, "steam_unavailable")).into_response();
    };
    match service
        .complete(
            &params,
            Some(IpNetwork::from(peer.ip())),
            user_agent(&headers),
        )
        .await
    {
        Ok(SteamCallbackResult::Login(session)) => {
            let jar = jar.add(refresh_cookie(&state, session.refresh_token));
            (jar, Redirect::to(&completion_url(&state, "login"))).into_response()
        }
        Ok(SteamCallbackResult::Bound(_)) => {
            (jar, Redirect::to(&completion_url(&state, "bind"))).into_response()
        }
        Err(error) => {
            tracing::warn!(%error, "Steam callback failed");
            (jar, callback_redirect(&state, callback_error_code(&error))).into_response()
        }
    }
}

async fn unbind(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<SteamUnbindRequest>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user = steam_service(&state)?
        .unbind(principal.user_id, request.password)
        .await?;
    Ok(Json(ApiResponse::new(user)))
}

async fn sync(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user = steam_service(&state)?.sync(principal.user_id).await?;
    Ok(Json(ApiResponse::new(user)))
}

fn steam_service(state: &AppState) -> Result<&crate::services::SteamAuthService, AppError> {
    state.steam_auth().ok_or(AppError::SteamUnavailable)
}

fn completion_url(state: &AppState, mode: &str) -> String {
    let origin = state
        .config()
        .steam_web_origin
        .as_deref()
        .unwrap_or(&state.config().cors_origin);
    format!("{origin}/auth/steam/complete?mode={mode}")
}

fn callback_redirect(state: &AppState, code: &str) -> Redirect {
    let origin = state
        .config()
        .steam_web_origin
        .as_deref()
        .unwrap_or(&state.config().cors_origin);
    Redirect::to(&format!("{origin}/auth/steam/complete?error={code}"))
}

fn state_cookie(state: &AppState, value: String) -> Cookie<'static> {
    Cookie::build(("lumiforum_steam_state", value))
        .http_only(true)
        .secure(state.config().refresh_cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/auth/steam")
        .max_age(time::Duration::minutes(10))
        .build()
}

fn state_removal_cookie(state: &AppState) -> Cookie<'static> {
    Cookie::build(("lumiforum_steam_state", String::new()))
        .http_only(true)
        .secure(state.config().refresh_cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/auth/steam")
        .max_age(time::Duration::ZERO)
        .build()
}

fn callback_error_code(error: &SteamAuthError) -> &'static str {
    match error {
        SteamAuthError::InvalidState => "steam_invalid_state",
        SteamAuthError::AccountConflict => "steam_account_conflict",
        SteamAuthError::Unavailable => "steam_unavailable",
        _ => "steam_auth_failed",
    }
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
}

impl From<SteamAuthError> for AppError {
    fn from(error: SteamAuthError) -> Self {
        match error {
            SteamAuthError::Unavailable => Self::SteamUnavailable,
            SteamAuthError::InvalidState | SteamAuthError::AuthenticationFailed => {
                Self::SteamAuthenticationFailed
            }
            SteamAuthError::AccountConflict => Self::SteamAccountConflict,
            SteamAuthError::NotLinked => Self::NotFound,
            SteamAuthError::InvalidPassword => Self::InvalidCredentials,
            SteamAuthError::SoleLoginMethod => Self::SoleLoginMethod,
            SteamAuthError::AccountUnavailable => Self::AccountUnavailable,
            SteamAuthError::Internal(error) => Self::Internal(error),
        }
    }
}
