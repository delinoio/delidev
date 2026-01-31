use serde::Serialize;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::AppHandle;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tracing::info;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::services::{UpdateInfo, UpdateService};

/// Response for update check command.
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    /// Whether an update is available.
    pub update_available: bool,
    /// Information about the available update, if any.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub update_info: Option<UpdateInfo>,
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub update_info: Option<()>,
}

/// Checks if an application update is available.
///
/// Returns information about the available update if one exists.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateCheckResponse, String> {
    info!("Checking for application updates...");

    let update_service = UpdateService::new(app);

    match update_service.check_for_update().await {
        Ok(Some(info)) => {
            info!(
                "Update available: {} -> {}",
                info.current_version, info.version
            );
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

/// On mobile platforms, updates are handled by app stores.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub async fn check_for_update() -> Result<UpdateCheckResponse, String> {
    // On mobile, updates are handled by the App Store / Play Store
    Ok(UpdateCheckResponse {
        update_available: false,
        update_info: None,
    })
}

/// Downloads and installs the available update.
///
/// This will restart the application after the update is installed.
/// Returns an error if no update is available or if the update fails.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    info!("Starting update download and installation...");

    let update_service = UpdateService::new(app);

    update_service
        .download_and_install()
        .await
        .map_err(|e| e.to_string())
}

/// On mobile platforms, updates are handled by app stores.
#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub async fn download_and_install_update() -> Result<(), String> {
    // On mobile, updates are handled by the App Store / Play Store
    Err("Updates are handled by the app store on mobile platforms".to_string())
}
