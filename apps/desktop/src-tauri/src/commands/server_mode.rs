//! Server mode commands for the desktop app
//!
//! These commands provide information about the process mode
//! (single-process vs remote) to the frontend.

use crate::single_process::{ProcessMode, SingleProcessConfig};
use serde::{Deserialize, Serialize};

/// Response for the get_server_mode command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeResponse {
    /// The current process mode
    pub mode: String,

    /// Remote server URL (if mode is "remote")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
}

/// Gets the current server mode configuration
///
/// This command is used by the frontend to determine whether the app
/// is running in single-process mode or connecting to a remote server.
#[tauri::command]
pub async fn get_server_mode() -> Result<ServerModeResponse, String> {
    // Load the configuration from the config manager or use defaults
    let config = SingleProcessConfig::default();

    let mode = match config.mode {
        ProcessMode::SingleProcess => "single_process".to_string(),
        ProcessMode::Remote => "remote".to_string(),
    };

    Ok(ServerModeResponse {
        mode,
        server_url: config.server_url,
    })
}

/// Sets the server mode configuration
///
/// This command allows the frontend to switch between single-process
/// mode and remote server mode.
#[tauri::command]
pub async fn set_server_mode(mode: String, server_url: Option<String>) -> Result<(), String> {
    let _config = match mode.as_str() {
        "single_process" => SingleProcessConfig::single_process(),
        "remote" => {
            let url = server_url.ok_or("Server URL is required for remote mode")?;
            SingleProcessConfig::remote(url)
        }
        _ => return Err(format!("Invalid mode: {}. Expected 'single_process' or 'remote'", mode)),
    };

    // In a full implementation, we would save this configuration
    // and reinitialize the appropriate services
    tracing::info!("Server mode set to: {}", mode);

    Ok(())
}

/// Checks if the app is running in single-process mode
#[tauri::command]
pub fn is_single_process_mode() -> bool {
    // For now, always return true since we default to single-process mode
    true
}
