use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use thiserror::Error;
use tracing::{error, info};

/// Errors that can occur during update operations.
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Update check failed: {0}")]
    CheckFailed(String),
    #[error("Update download failed: {0}")]
    DownloadFailed(String),
    #[error("Update installation failed: {0}")]
    InstallFailed(String),
    #[error("No update available")]
    NoUpdateAvailable,
}

/// Information about an available update.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// The version of the available update.
    pub version: String,
    /// The current version of the application.
    pub current_version: String,
    /// The release notes/body of the update, if available.
    pub body: Option<String>,
    /// The date of the release, if available.
    pub date: Option<String>,
}

/// Service for managing application updates.
pub struct UpdateService {
    app_handle: AppHandle,
}

impl UpdateService {
    /// Creates a new UpdateService.
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Checks if an update is available.
    ///
    /// Returns `Ok(Some(UpdateInfo))` if an update is available,
    /// `Ok(None)` if the app is up to date, or an error if the check fails.
    pub async fn check_for_update(&self) -> Result<Option<UpdateInfo>, UpdateError> {
        info!("Checking for updates...");

        let updater = self
            .app_handle
            .updater_builder()
            .build()
            .map_err(|e| UpdateError::CheckFailed(e.to_string()))?;

        match updater.check().await {
            Ok(Some(update)) => {
                let info = UpdateInfo {
                    version: update.version.clone(),
                    current_version: update.current_version.clone(),
                    body: update.body.clone(),
                    date: update.date.map(|d| d.to_string()),
                };
                info!(
                    "Update available: {} -> {}",
                    info.current_version, info.version
                );
                Ok(Some(info))
            }
            Ok(None) => {
                info!("No update available");
                Ok(None)
            }
            Err(e) => {
                error!("Failed to check for updates: {}", e);
                Err(UpdateError::CheckFailed(e.to_string()))
            }
        }
    }

    /// Downloads and installs the available update.
    ///
    /// This will restart the application after the update is installed.
    pub async fn download_and_install(&self) -> Result<(), UpdateError> {
        info!("Starting update download and installation...");

        let updater = self
            .app_handle
            .updater_builder()
            .build()
            .map_err(|e| UpdateError::CheckFailed(e.to_string()))?;

        let update = updater
            .check()
            .await
            .map_err(|e| UpdateError::CheckFailed(e.to_string()))?
            .ok_or(UpdateError::NoUpdateAvailable)?;

        info!("Downloading update version {}...", update.version);

        // Download the update
        let mut downloaded = 0;
        let bytes = update
            .download(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    info!(
                        "Downloaded {} of {:?} bytes",
                        downloaded,
                        content_length.unwrap_or(0)
                    );
                },
                || {
                    info!("Download finished");
                },
            )
            .await
            .map_err(|e| {
                error!("Failed to download update: {}", e);
                UpdateError::DownloadFailed(e.to_string())
            })?;

        info!("Installing update...");

        // Install the update
        update.install(bytes).map_err(|e| {
            error!("Failed to install update: {}", e);
            UpdateError::InstallFailed(e.to_string())
        })?;

        info!("Update installed successfully, restarting application...");

        // Restart the application
        self.app_handle.restart();
    }

    /// Downloads and installs the update with progress callback.
    ///
    /// The callback receives (downloaded_bytes, total_bytes).
    /// This will restart the application after the update is installed.
    pub async fn download_and_install_with_progress<F>(
        &self,
        on_progress: F,
    ) -> Result<(), UpdateError>
    where
        F: Fn(u64, Option<u64>) + Send + 'static,
    {
        info!("Starting update download and installation with progress...");

        let updater = self
            .app_handle
            .updater_builder()
            .build()
            .map_err(|e| UpdateError::CheckFailed(e.to_string()))?;

        let update = updater
            .check()
            .await
            .map_err(|e| UpdateError::CheckFailed(e.to_string()))?
            .ok_or(UpdateError::NoUpdateAvailable)?;

        info!("Downloading update version {}...", update.version);

        // Download the update with progress
        let mut downloaded: u64 = 0;
        let bytes = update
            .download(
                move |chunk_length, content_length| {
                    downloaded += chunk_length as u64;
                    on_progress(downloaded, content_length.map(|c| c as u64));
                },
                || {
                    info!("Download finished");
                },
            )
            .await
            .map_err(|e| {
                error!("Failed to download update: {}", e);
                UpdateError::DownloadFailed(e.to_string())
            })?;

        info!("Installing update...");

        // Install the update
        update.install(bytes).map_err(|e| {
            error!("Failed to install update: {}", e);
            UpdateError::InstallFailed(e.to_string())
        })?;

        info!("Update installed successfully, restarting application...");

        // Restart the application
        self.app_handle.restart();
    }
}
