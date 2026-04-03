use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

use crate::cli::ServeArgs;

#[derive(Clone)]
struct AppState {
    count: Arc<AtomicU64>,
}

pub async fn serve(args: ServeArgs) -> Result<()> {
    let state = AppState {
        count: Arc::new(AtomicU64::new(0)),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/count", get(count))
        .route("/incr", get(incr))
        .with_state(state);

    let listener = TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("failed to bind counter API on {}", args.listen))?;

    info!("counter API listening on {}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("counter API exited unexpectedly")
}

async fn healthz() -> &'static str {
    "ok"
}

async fn count(State(state): State<AppState>) -> String {
    state.count.load(Ordering::Relaxed).to_string()
}

async fn incr(State(state): State<AppState>) -> (StatusCode, String) {
    // Relaxed ordering is enough because we only need a monotonic counter and
    // do not derive any other synchronization guarantees from this value.
    let next = state.count.fetch_add(1, Ordering::Relaxed) + 1;
    info!("counter incremented to {}", next);
    (StatusCode::OK, next.to_string())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!("failed to listen for ctrl-c: {error:#}");
    }

    info!("counter API shutdown requested");
}

#[allow(dead_code)]
fn _assert_send_sync()
where
    AppState: Send + Sync,
{
}

#[allow(dead_code)]
fn _assert_socket_addr_send_sync()
where
    SocketAddr: Send + Sync,
{
}
