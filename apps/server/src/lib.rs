//! DeliDev Main Server library
//!
//! This module exposes the server components for use in single-process mode
//! or for testing.

pub mod auth_routes;
pub mod config;
pub mod log_broadcaster;
pub mod middleware;
pub mod rpc;
pub mod state;
pub mod websocket;
pub mod worker_registry;

pub use auth::AuthStateStore;
pub use config::ServerConfig;
pub use log_broadcaster::LogBroadcaster;
pub use rpc::dispatch_method;
pub use state::AppState;
pub use worker_registry::WorkerRegistry;
