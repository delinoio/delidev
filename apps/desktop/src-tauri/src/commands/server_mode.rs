//! Server mode commands for the desktop app
//!
//! These commands provide information about the process mode
//! (single-process vs remote) to the frontend.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;

use crate::single_process::{ProcessMode, SingleProcessConfig, SingleProcessRuntime};

/// Managed state for single-process mode configuration and runtime
pub struct SingleProcessState {
    /// Current configuration
    pub config: RwLock<SingleProcessConfig>,
    /// Runtime (initialized when in single-process mode)
    pub runtime: RwLock<Option<Arc<SingleProcessRuntime>>>,
}

impl SingleProcessState {
    /// Creates a new single-process state with default configuration
    pub fn new() -> Self {
        Self {
            config: RwLock::new(SingleProcessConfig::default()),
            runtime: RwLock::new(None),
        }
    }

    /// Creates a new single-process state with the given configuration
    pub fn with_config(config: SingleProcessConfig) -> Self {
        Self {
            config: RwLock::new(config),
            runtime: RwLock::new(None),
        }
    }
}

impl Default for SingleProcessState {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for the get_server_mode command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeResponse {
    /// The current process mode
    pub mode: ProcessMode,

    /// Remote server URL (if mode is "remote")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
}

/// Gets the current server mode configuration
///
/// This command is used by the frontend to determine whether the app
/// is running in single-process mode or connecting to a remote server.
#[tauri::command]
pub async fn get_server_mode(
    state: State<'_, SingleProcessState>,
) -> Result<ServerModeResponse, String> {
    let config = state.config.read().await;

    Ok(ServerModeResponse {
        mode: config.mode,
        server_url: config.server_url.clone(),
    })
}

/// Sets the server mode configuration
///
/// This command allows the frontend to switch between single-process
/// mode and remote server mode.
#[tauri::command]
pub async fn set_server_mode(
    mode: ProcessMode,
    server_url: Option<String>,
    state: State<'_, SingleProcessState>,
) -> Result<(), String> {
    // Validate remote mode requires a server URL
    if mode == ProcessMode::Remote && server_url.is_none() {
        return Err("Server URL is required for remote mode".to_string());
    }

    // Create the new configuration
    let new_config = match mode {
        ProcessMode::SingleProcess => SingleProcessConfig::single_process(),
        ProcessMode::Remote => SingleProcessConfig::remote(server_url.unwrap()),
    };

    // Update the configuration
    {
        let mut config = state.config.write().await;
        *config = new_config.clone();
    }

    // If switching to single-process mode and runtime is not initialized,
    // initialize it
    if mode == ProcessMode::SingleProcess {
        let mut runtime_guard = state.runtime.write().await;
        if runtime_guard.is_none() {
            match SingleProcessRuntime::new(new_config).await {
                Ok(runtime) => {
                    *runtime_guard = Some(Arc::new(runtime));
                    tracing::info!("Single-process runtime initialized");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize single-process runtime: {}", e);
                    return Err(format!(
                        "Failed to initialize single-process runtime: {}",
                        e
                    ));
                }
            }
        }
    } else {
        // If switching to remote mode, we can optionally shut down the runtime
        let mut runtime_guard = state.runtime.write().await;
        if runtime_guard.is_some() {
            *runtime_guard = None;
            tracing::info!("Single-process runtime shut down (switched to remote mode)");
        }
    }

    tracing::info!("Server mode set to: {:?}", mode);
    Ok(())
}

/// Checks if the app is running in single-process mode
#[tauri::command]
pub async fn is_single_process_mode(state: State<'_, SingleProcessState>) -> Result<bool, String> {
    let config = state.config.read().await;
    Ok(config.is_single_process())
}
