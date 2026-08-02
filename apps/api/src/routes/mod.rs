mod admin;
pub mod auth;
mod categories;
mod comments;
mod health;
mod moderation;
mod notifications;
mod polls;
mod presence;
mod reactions;
mod response;
mod search;
mod settings;
mod steam_auth;
mod topics;
mod uploads;
pub mod users;
mod ws;

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, services::ServeDir,
    set_header::SetResponseHeaderLayer, trace::TraceLayer,
};

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = build_cors(state.config().cors_origin.as_str());

    let router = Router::new()
        .merge(health::router())
        .merge(auth::public_router(state.clone()))
        .merge(auth::protected_router(state.clone()))
        .merge(steam_auth::public_router(state.clone()))
        .merge(steam_auth::protected_router(state.clone()))
        .merge(users::protected_router(state.clone()))
        .merge(categories::router(state.clone()))
        .merge(topics::router(state.clone()))
        .merge(comments::router(state.clone()))
        .merge(reactions::router(state.clone()))
        .merge(notifications::router(state.clone()))
        .merge(polls::public_router(state.clone()))
        .merge(search::router())
        .merge(settings::router())
        .merge(uploads::router(state.clone()))
        .merge(admin::router(state.clone()))
        .merge(admin::public_report_router(state.clone()))
        .merge(moderation::public_router(state.clone()))
        .merge(moderation::admin_router(state.clone()))
        .merge(moderation::metrics_router(state.clone()))
        .merge(presence::router())
        .route("/ws", get(ws::ws_handler));
    let router = if state.config().storage_provider == "local" {
        let files = Router::new()
            .nest_service(
                "/storage",
                ServeDir::new(state.config().storage_local_root.clone()),
            )
            .layer(SetResponseHeaderLayer::if_not_present(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ));
        router.merge(files)
    } else {
        router
    };

    router
        .layer(middleware::from_fn_with_state(
            state.clone(),
            count_http_request,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(20 * 1024 * 1024 + 64 * 1024))
        .layer(cors)
        .with_state(state)
}

/// Lightweight per-request counter backing the admin dashboard.
async fn count_http_request(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    state.metrics().inc("http_requests_total", &[]);
    next.run(request).await
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
