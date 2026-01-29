use std::{process::Command, sync::Arc};

use tauri::State;
use tracing::info;
use uuid::Uuid;

use crate::services::AppState;

/// Checks if Docker is available
#[tauri::command]
pub async fn check_docker(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    // Try to initialize docker service if needed
    Ok(state.try_init_docker_service().await)
}

/// Gets Docker version
#[tauri::command]
pub async fn get_docker_version(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    // Try to initialize docker service if needed
    if !state.try_init_docker_service().await {
        return Err("Docker/Podman is not available".to_string());
    }

    let docker_guard = state.docker_service.read().await;
    match docker_guard.as_ref() {
        Some(docker) => docker.version().await.map_err(|e| e.to_string()),
        None => Err("Docker/Podman is not available".to_string()),
    }
}

/// Gets application info
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
    }
}

#[derive(serde::Serialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
}

/// Opens a file in the configured external editor
///
/// Uses the editor's diff feature (via VSCode-compatible `--diff` syntax) to
/// show old vs new version when both base and head commits are provided. Falls
/// back to opening the file normally if only the file path is provided.
///
/// Note: This assumes all supported editors (VSCode, Cursor, Windsurf, VSCode
/// Insiders, VSCodium) support the `--diff` CLI flag with the same syntax.
#[tauri::command]
pub async fn open_in_editor(
    state: State<'_, Arc<AppState>>,
    file_path: String,
    repo_path: Option<String>,
    base_commit: Option<String>,
    head_commit: Option<String>,
) -> Result<(), String> {
    let config = state.global_config.read().await;
    let editor_type = config.editor.editor_type;
    let editor_cmd = editor_type.command();

    info!(
        "Opening file in editor: {} with command: {}",
        file_path, editor_cmd
    );

    // If we have both base and head commits, use the diff command
    if let (Some(repo), Some(base), Some(head)) = (repo_path, base_commit, head_commit) {
        // Use VSCode-compatible diff interface with git show
        // Format: <editor-cmd> --diff <left-file> <right-file>
        // We use git show to get the content at specific commits

        // Create temp files for the diff with unique IDs to prevent collisions
        let temp_dir = std::env::temp_dir();
        let unique_id = Uuid::new_v4();
        let base_temp = temp_dir.join(format!(
            "delidev_base_{}_{}.tmp",
            unique_id,
            sanitize_filename(&file_path)
        ));
        let head_temp = temp_dir.join(format!(
            "delidev_head_{}_{}.tmp",
            unique_id,
            sanitize_filename(&file_path)
        ));

        // Get file content at base commit
        let base_content = Command::new("git")
            .args(["-C", &repo, "show", &format!("{}:{}", base, file_path)])
            .output()
            .map_err(|e| format!("Failed to get base file content: {}", e))?;

        if !base_content.status.success() {
            // File might not exist at base commit (new file)
            std::fs::write(&base_temp, "").map_err(|e| e.to_string())?;
        } else {
            std::fs::write(&base_temp, &base_content.stdout).map_err(|e| e.to_string())?;
        }

        // Get file content at head commit
        let head_content = Command::new("git")
            .args(["-C", &repo, "show", &format!("{}:{}", head, file_path)])
            .output()
            .map_err(|e| format!("Failed to get head file content: {}", e))?;

        if !head_content.status.success() {
            // File might not exist at head commit (deleted file)
            std::fs::write(&head_temp, "").map_err(|e| e.to_string())?;
        } else {
            std::fs::write(&head_temp, &head_content.stdout).map_err(|e| e.to_string())?;
        }

        // Open diff in editor
        let status = Command::new(editor_cmd)
            .args([
                "--diff",
                base_temp.to_str().ok_or("Invalid path")?,
                head_temp.to_str().ok_or("Invalid path")?,
            ])
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to open editor: {}. Is {} installed?",
                    e,
                    editor_type.display_name()
                )
            })?;

        info!("Opened diff in editor with PID: {:?}", status.id());
    } else {
        // Just open the file directly
        let status = Command::new(editor_cmd)
            .arg(&file_path)
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to open editor: {}. Is {} installed?",
                    e,
                    editor_type.display_name()
                )
            })?;

        info!("Opened file in editor with PID: {:?}", status.id());
    }

    Ok(())
}

/// Sanitizes a filename by replacing path separators with underscores
fn sanitize_filename(path: &str) -> String {
    path.replace(['/', '\\'], "_")
}
