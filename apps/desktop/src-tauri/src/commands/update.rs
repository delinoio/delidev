use serde::Serialize;
use tauri::AppHandle;
use tracing::info;

use crate::services::{UpdateInfo, UpdateService};

/// Response for update check command.
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    /// Whether an update is available.
    pub update_available: bool,
    /// Information about the available update, if any.
    pub update_info: Option<UpdateInfo>,
}

/// Checks if an application update is available.
///
/// Returns information about the available update if one exists.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateCheckResponse, String> {
    info!("Checking for application updates...");

    let update_service = UpdateService::new(app);

    match update_service.check_for_update().await {
        Ok(Some(info)) => {
            info!("Update available: {} -> {}", info.current_version, info.version);
            Ok(UpdateCheckResponse {
                update_available: true,
                update_info: Some(info),
            })
        }
        Ok(None) => {
            info!("Application is up to date");
            Ok(UpdateCheckResponse {
                update_available: false,
                update_info: None,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to check for updates: {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Downloads and installs the available update.
///
/// This will restart the application after the update is installed.
/// Returns an error if no update is available or if the update fails.
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    info!("Starting update download and installation...");

    let update_service = UpdateService::new(app);

    update_service
        .download_and_install()
        .await
        .map_err(|e| e.to_string())
}
