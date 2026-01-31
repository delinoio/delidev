//! DeliDev Main Server
//!
//! The Main Server is the central hub of DeliDev's distributed architecture.
//! It maintains the task list, coordinates workers, and provides the RPC interface for clients.

pub mod api;
pub mod config;
pub mod error;
pub mod middleware;
pub mod services;
pub mod state;

use std::sync::Arc;

use auth::{JwtConfig, JwtManager};
use axum::Router;
use task_store::TaskStore;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::config::Config;
use crate::state::{AppState, create_shared_state};

/// Creates the application router with all routes configured.
pub fn create_app<S: TaskStore + 'static>(state: Arc<AppState<S>>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    api::create_router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

/// Creates the application state with the given configuration and store.
pub fn create_state<S: TaskStore>(config: Config, store: S) -> Arc<AppState<S>> {
    let jwt_manager = if config.auth_enabled() {
        config.jwt_secret.as_ref().map(|secret| {
            let jwt_config = JwtConfig::new(secret)
                .with_expiration_hours(config.jwt_expiration_hours);
            JwtManager::new(jwt_config)
        })
    } else {
        None
    };

    create_shared_state(config, store, jwt_manager)
}

/// Initializes tracing with the given log level.
pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}
