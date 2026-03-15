//! Entry: load config, start HTTP server, graceful shutdown.

mod config;
mod cursor;
mod openai;
mod server;
mod session;
mod service;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(config::load_config());
    let port = config.port;
    let app = server::app(config);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("ironclaw-cursor-brain listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("ctrl_c");
    tracing::info!("shutting down");
}
