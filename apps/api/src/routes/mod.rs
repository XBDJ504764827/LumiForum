pub mod auth;
mod categories;
mod comments;
mod health;
mod reactions;
mod response;
mod topics;
pub mod users;

use axum::{
    http::{header, HeaderValue, Method},
    Router,
};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = build_cors(state.config().cors_origin.as_str());

    Router::new()
        .merge(health::router())
        .merge(auth::public_router(state.clone()))
        .merge(auth::protected_router(state.clone()))
        .merge(users::protected_router(state.clone()))
        .merge(categories::router(state.clone()))
        .merge(topics::router(state.clone()))
        .merge(comments::router(state.clone()))
        .merge(reactions::router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(cors)
        .with_state(state)
}

fn build_cors(origin: &str) -> CorsLayer {
    let origin = origin
        .parse::<HeaderValue>()
        .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:3000"));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE])
}
