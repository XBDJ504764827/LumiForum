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
            .route_layer(middleware::from_fn(storage_headers))
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
        .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024 + 64 * 1024))
        .layer(cors)
        .with_state(state)
}

/// Security headers applied to every object served from `/storage`.
///
/// - `X-Content-Type-Options: nosniff` prevents content-type confusion: an
///   uploaded file can never be reinterpreted by the browser as HTML.
/// - Processed images (jpg/png/webp/gif) are served inline so `<img>` tags
///   work. Every other verified type is forced to download, so uploaded
///   content (PDF with embedded scripts, XML, …) is never rendered inside the
///   forum origin.
async fn storage_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    let extension = path.rsplit('.').next().map(str::to_ascii_lowercase);
    let is_image = matches!(
        extension.as_deref(),
        Some("jpg" | "jpeg" | "png" | "webp" | "gif")
    );
    if !is_image {
        // Stored filenames are server-generated (`{uuid}.{verified_extension}`),
        // so this value is safe to echo back into a header.
        let filename = path.rsplit('/').next().unwrap_or("file");
        let value = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment"));
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
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
