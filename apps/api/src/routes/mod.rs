mod health;

use axum::{
    http::{HeaderValue, Method},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = build_cors(state.config().cors_origin.as_str());

    Router::new()
        .merge(health::router())
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(cors)
        .with_state(state)
}

fn build_cors(origin: &str) -> CorsLayer {
    if origin == "*" {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any);
    }

    let origin = origin
        .parse::<HeaderValue>()
        .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:3000"));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
}
