//! Embedded Server for Single Process Mode
//!
//! This module provides an embedded server that handles JSON-RPC requests
//! locally without network communication. It uses the same RPC dispatch
//! logic as the standalone server but runs entirely in-process.

use std::sync::Arc;

use rpc_protocol::{JsonRpcRequest, JsonRpcResponse};
use tokio::sync::RwLock;

use super::{SingleProcessConfig, SingleProcessError};

/// Embedded server for single-process mode
///
/// This server handles JSON-RPC requests locally without network I/O.
/// It provides the same functionality as the standalone server but
/// communicates via direct function calls instead of HTTP/WebSocket.
pub struct EmbeddedServer {
    /// Server application state (cached and reused across requests)
    app_state: delidev_server::AppState,

    /// Configuration
    _config: SingleProcessConfig,
}

impl EmbeddedServer {
    /// Creates a new embedded server
    pub async fn new(config: &SingleProcessConfig) -> Result<Self, SingleProcessError> {
        tracing::info!("Initializing embedded server for single-process mode");

        // Create server configuration for single-user mode
        let server_config = delidev_server::ServerConfig {
            single_user_mode: true,
            ..Default::default()
        };

        // Initialize the server's AppState once and cache it
        let app_state = delidev_server::AppState::new(server_config)
            .await
            .map_err(|e| SingleProcessError::ServerInit(e.to_string()))?;

        Ok(Self {
            app_state,
            _config: config.clone(),
        })
    }

    /// Returns the task store
    pub fn store(&self) -> &Arc<dyn task_store::TaskStore> {
        &self.app_state.store
    }

    /// Returns the log broadcaster
    pub fn log_broadcaster(&self) -> &Arc<delidev_server::LogBroadcaster> {
        &self.app_state.log_broadcaster
    }

    /// Returns the worker registry
    pub fn worker_registry(&self) -> &Arc<RwLock<delidev_server::WorkerRegistry>> {
        &self.app_state.worker_registry
    }

    /// Handles a JSON-RPC request locally
    ///
    /// This method dispatches the request to the appropriate handler
    /// without any network communication. It reuses the server's
    /// dispatch_method to avoid code duplication.
    pub async fn handle_rpc(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // In single-process mode, there is no authenticated user
        // (auth is disabled for trusted local execution)
        // We pass None since authentication is skipped in single-user mode
        let user = None;

        // Use the server's shared dispatch_method to handle the request
        match delidev_server::dispatch_method(&self.app_state, &user, &request).await {
            Ok(result) => JsonRpcResponse::success(request.id, result),
            Err(error) => JsonRpcResponse::error(request.id, error),
        }
    }
}
