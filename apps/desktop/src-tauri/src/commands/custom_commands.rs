use std::{path::PathBuf, sync::Arc};

use tauri::State;

use crate::{
    entities::{AIAgentType, CommandFramework, CustomCommand},
    services::{AppState, CustomCommandService},
};

/// Lists all custom commands for a repository
#[tauri::command]
pub async fn list_custom_commands(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
) -> Result<Vec<CustomCommand>, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    let repo_path = PathBuf::from(&repo.local_path);
    let service = CustomCommandService::new();

    service
        .discover_commands(&repo_path)
        .map_err(|e| e.to_string())
}

/// Gets a specific custom command by name
#[tauri::command]
pub async fn get_custom_command(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
    command_name: String,
) -> Result<CustomCommand, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    let repo_path = PathBuf::from(&repo.local_path);
    let service = CustomCommandService::new();

    service
        .get_command(&repo_path, &command_name)
        .map_err(|e| e.to_string())
}

/// Lists custom commands filtered by agent type
#[tauri::command]
pub async fn list_custom_commands_by_agent(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
    agent_type: AIAgentType,
) -> Result<Vec<CustomCommand>, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    let repo_path = PathBuf::from(&repo.local_path);
    let service = CustomCommandService::new();

    service
        .list_commands_by_agent(&repo_path, agent_type)
        .map_err(|e| e.to_string())
}

/// Lists custom commands filtered by framework
#[tauri::command]
pub async fn list_custom_commands_by_framework(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
    framework: CommandFramework,
) -> Result<Vec<CustomCommand>, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    let repo_path = PathBuf::from(&repo.local_path);
    let service = CustomCommandService::new();

    service
        .list_commands_by_framework(&repo_path, framework)
        .map_err(|e| e.to_string())
}

/// Renders a custom command template with arguments
#[tauri::command]
pub async fn render_custom_command(
    state: State<'_, Arc<AppState>>,
    repository_id: String,
    command_name: String,
    arguments: String,
) -> Result<String, String> {
    let repo = state
        .repository_service
        .get(&repository_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found".to_string())?;

    let repo_path = PathBuf::from(&repo.local_path);
    let service = CustomCommandService::new();

    let command = service
        .get_command(&repo_path, &command_name)
        .map_err(|e| e.to_string())?;

    Ok(command.render(&arguments))
}
