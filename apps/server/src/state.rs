//! Application state

#![allow(dead_code)]

use std::sync::Arc;

use auth::JwtAuth;
use task_store::{MemoryStore, TaskStore};
use tokio::sync::RwLock;

use crate::{
    config::ServerConfig, log_broadcaster::LogBroadcaster, worker_registry::WorkerRegistry,
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Task store
    pub store: Arc<dyn TaskStore>,

    /// JWT authentication (None in single-user mode)
    pub auth: Option<Arc<JwtAuth>>,

    /// Worker registry
    pub worker_registry: Arc<RwLock<WorkerRegistry>>,

    /// Log broadcaster for real-time streaming
    pub log_broadcaster: Arc<LogBroadcaster>,

    /// Server configuration
    pub config: Arc<ServerConfig>,
}

impl AppState {
    /// Create a new application state
    pub async fn new(config: ServerConfig) -> Result<Self, StateError> {
        // Initialize store based on mode
        let store: Arc<dyn TaskStore> = if config.single_user_mode {
            // Use in-memory store for now (SQLite can be added later)
            Arc::new(MemoryStore::new())
        } else {
            // In multi-user mode, would use PostgreSQL
            // For now, use memory store as a placeholder
            tracing::warn!("PostgreSQL store not yet implemented, using in-memory store");
            Arc::new(MemoryStore::new())
        };

        // Initialize auth (skip in single-user mode)
        let auth = if config.single_user_mode {
            None
        } else {
            Some(Arc::new(JwtAuth::new_hs256(config.jwt_secret.as_bytes())))
        };

        // Initialize worker registry
        let worker_registry = Arc::new(RwLock::new(WorkerRegistry::new(
            config.worker_heartbeat_timeout_secs,
        )));

        // Initialize log broadcaster
        let log_broadcaster = Arc::new(LogBroadcaster::new());

        Ok(Self {
            store,
            auth,
            worker_registry,
            log_broadcaster,
            config: Arc::new(config),
        })
    }
}

/// State initialization errors
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Failed to initialize database: {0}")]
    Database(String),

    #[error("Failed to initialize auth: {0}")]
    Auth(String),
}
