use std::net::SocketAddr;

use axum::{
    extract::rejection::JsonRejection,
    extract::{ConnectInfo, State},
    http::{header::USER_AGENT, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use ipnetwork::IpNetwork;

use crate::error::AppResult;
use crate::middleware::{enforce_origin, require_permission, AuthorizationLayer, CsrfLayer};
use crate::models::{
    AuthResponse, AuthenticatedPrincipal, LoginRequest, RegisterRequest, UserResponse,
    PERMISSION_PROFILE_READ_SELF,
};
use crate::state::AppState;

use super::response::{parse_json, ApiResponse, MessageResponse};

pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route_layer(middleware::from_fn_with_state(
            CsrfLayer::new(state.config().cors_origin.clone()),
            enforce_origin,
        ))
}

pub fn protected_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationLayer::new(state, PERMISSION_PROFILE_READ_SELF),
            require_permission,
        ))
}

async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    payload: Result<Json<RegisterRequest>, JsonRejection>,
) -> AppResult<(StatusCode, CookieJar, Json<ApiResponse<AuthResponse>>)> {
    let request = parse_json(payload)?;
    let session = state
        .auth()
        .register(request, peer_ip(peer), user_agent(&headers))
        .await?;
    let jar = jar.add(refresh_cookie(&state, session.refresh_token));
    Ok((
        StatusCode::CREATED,
        jar,
        Json(ApiResponse::new(session.response)),
    ))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> AppResult<(CookieJar, Json<ApiResponse<AuthResponse>>)> {
    let request = parse_json(payload)?;
    let session = state
        .auth()
        .login(request, peer_ip(peer), user_agent(&headers))
        .await?;
    let jar = jar.add(refresh_cookie(&state, session.refresh_token));
    Ok((jar, Json(ApiResponse::new(session.response))))
}

async fn refresh(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
) -> AppResult<Response> {
    let Some(token) = jar
        .get(&state.config().refresh_cookie_name)
        .map(Cookie::value)
    else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    let session = state
        .auth()
        .refresh(token, peer_ip(peer), user_agent(&headers))
        .await?;
    let jar = jar.add(refresh_cookie(&state, session.refresh_token));
    Ok((jar, Json(ApiResponse::new(session.response))).into_response())
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<ApiResponse<MessageResponse>>)> {
    let token = jar
        .get(&state.config().refresh_cookie_name)
        .map(Cookie::value);
    state.auth().logout(token).await?;
    let jar = jar.remove(removal_cookie(&state));
    Ok((
        jar,
        Json(ApiResponse::new(MessageResponse {
            message: "logged out",
        })),
    ))
}

async fn me(
    State(state): State<AppState>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user = state.users().get_profile(principal.user_id).await?;
    Ok(Json(ApiResponse::new(user)))
}

pub(crate) fn refresh_cookie(state: &AppState, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::build((state.config().refresh_cookie_name.clone(), value))
        .http_only(true)
        .secure(state.config().refresh_cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/auth")
        .max_age(time::Duration::seconds(
            state.auth().refresh_token_ttl_seconds(),
        ));
    if let Some(domain) = &state.config().cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

fn removal_cookie(state: &AppState) -> Cookie<'static> {
    let mut cookie = Cookie::build((state.config().refresh_cookie_name.clone(), String::new()))
        .http_only(true)
        .secure(state.config().refresh_cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/auth")
        .max_age(time::Duration::ZERO);
    if let Some(domain) = &state.config().cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

fn peer_ip(peer: SocketAddr) -> Option<IpNetwork> {
    Some(IpNetwork::from(peer.ip()))
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
}
