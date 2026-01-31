//! Single Process Mode for DeliDev
//!
//! This module provides the embedded server and worker functionality
//! that allows the desktop app to run in single-process mode without
//! requiring a separate server deployment.
//!
//! In single-process mode:
//! - The server and worker are embedded in the desktop app
//! - No network communication is required
//! - SQLite is used instead of PostgreSQL
//! - Authentication is disabled (trusted local execution)
//! - All services communicate via in-process function calls

mod config;
mod embedded_server;
mod embedded_worker;

pub use config::{ProcessMode, SingleProcessConfig};
pub use embedded_server::EmbeddedServer;
pub use embedded_worker::EmbeddedWorker;

use std::sync::Arc;

/// The combined single-process runtime that includes embedded server and worker
pub struct SingleProcessRuntime {
    /// Embedded server for RPC handling
    server: Arc<EmbeddedServer>,

    /// Embedded worker for task execution
    worker: Arc<EmbeddedWorker>,

    /// Whether the runtime is initialized
    initialized: bool,
}

impl SingleProcessRuntime {
    /// Creates a new single-process runtime
    pub async fn new(config: SingleProcessConfig) -> Result<Self, SingleProcessError> {
        tracing::info!("Initializing single-process runtime");

        // Create embedded server
        let server = Arc::new(EmbeddedServer::new(&config).await?);

        // Create embedded worker that uses the server's store
        let worker = Arc::new(EmbeddedWorker::new(server.store().clone()).await?);

        Ok(Self {
            server,
            worker,
            initialized: true,
        })
    }

    /// Returns the embedded server
    pub fn server(&self) -> &Arc<EmbeddedServer> {
        &self.server
    }

    /// Returns the embedded worker
    pub fn worker(&self) -> &Arc<EmbeddedWorker> {
        &self.worker
    }

    /// Checks if the runtime is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// Errors that can occur in single-process mode
#[derive(Debug, thiserror::Error)]
pub enum SingleProcessError {
    #[error("Failed to initialize embedded server: {0}")]
    ServerInit(String),

    #[error("Failed to initialize embedded worker: {0}")]
    WorkerInit(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Store error: {0}")]
    Store(String),
}
