use std::net::SocketAddr;

use lumiforum_api::{config::Config, routes, state::AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 开发环境自动加载 .env.development（不存在时回退 .env）。
    // 生产环境由 systemd EnvironmentFile 注入进程环境变量，dotenvy
    // 不会覆盖已存在的环境变量，因此不影响生产部署。
    if dotenvy::from_filename(".env.development").is_err() {
        dotenvy::dotenv().ok();
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(config.clone()).await?;
    let app = routes::create_router(state);

    let addr = SocketAddr::from((config.host, config.port));
    tracing::info!(%addr, env = %config.app_env, "starting lumiforum-api");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
