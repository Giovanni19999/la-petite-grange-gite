//! Entry point: read the configuration, wire the dependencies, start the server.

use std::net::SocketAddr;

use anyhow::Context;
use gite_api::config::Config;
use gite_api::infrastructure::mariadb::{lazy_pool, migrate};
use gite_api::{http, telemetry, wiring};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A missing .env file is fine: variables may come from the real environment.
    let _ = dotenvy::dotenv();

    let config = Config::from_env().context("invalid configuration")?;
    telemetry::init(config.log_format);

    let pool = lazy_pool(&config).context("invalid DATABASE_URL")?;
    if let Err(error) = migrate(&pool).await {
        // The API still starts (health reports the database as unavailable).
        tracing::error!(%error, "database migrations not applied");
    }
    let state = wiring::app_state(&pool, config.session_policy, config.login_throttle)?;

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("cannot listen on {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, env = ?config.app_env, "API listening");

    axum::serve(
        listener,
        http::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server error")?;

    pool.close().await;
    tracing::info!("API stopped");
    Ok(())
}

/// Resolves when the process receives Ctrl+C (or SIGTERM on Unix).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "cannot listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "cannot listen for SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
