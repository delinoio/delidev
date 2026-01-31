//! DeliDev Main Server
//!
//! The main server handles:
//! - Task management (UnitTask, CompositeTask)
//! - Worker coordination and assignment
//! - Real-time log streaming via WebSocket
//! - User authentication (in multi-user mode)

use std::time::Duration;

use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod log_broadcaster;
mod middleware;
mod rpc;
mod state;
mod websocket;
mod worker_registry;

use config::ServerConfig;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ServerConfig::load().expect("Failed to load configuration");

    // Initialize tracing
    let log_level = match config.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("delidev_server={},tower_http=debug", log_level).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        single_user_mode = config.single_user_mode,
        "Starting DeliDev Server"
    );

    // Initialize application state
    let state = AppState::new(config.clone())
        .await
        .expect("Failed to initialize application state");

    // Build CORS layer
    let cors = if config.enable_cors {
        if config.cors_origins.is_empty() {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            let origins: Vec<_> = config
                .cors_origins
                .iter()
                .map(|s| s.parse().expect("Invalid CORS origin"))
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    } else {
        CorsLayer::new()
    };

    // Build router
    let app = Router::new()
        .route("/rpc", post(rpc::handle_rpc))
        .route("/ws", get(websocket::handle_websocket))
        .route("/health", get(health_check))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Start worker cleanup task
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let mut registry = cleanup_state.worker_registry.write().await;
            let stale = registry.cleanup_stale_workers();
            if !stale.is_empty() {
                info!(count = stale.len(), "Cleaned up stale workers");
            }
        }
    });

    // Start log broadcaster cleanup task
    let log_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            log_state.log_broadcaster.cleanup_empty_channels();
        }
    });

    // Bind and serve
    let listener = TcpListener::bind(&config.bind_address).await?;
    info!(address = %config.bind_address, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down");
        }
    }
}
