//! Runs the HTTP adapter: binds the address and serves until a shutdown signal.

use std::net::SocketAddr;

use anyhow::Context;
use tokio::net::TcpListener;

use crate::app_state::AppState;
use crate::http::router;

/// Serves the API on `addr` until Ctrl+C (or SIGTERM on Unix).
///
/// # Errors
///
/// Returns an error when the address cannot be bound or the server fails.
pub async fn serve(addr: SocketAddr, state: AppState) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("cannot listen on {addr}"))?;
    tracing::info!(%addr, "API listening");

    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server error")?;

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
