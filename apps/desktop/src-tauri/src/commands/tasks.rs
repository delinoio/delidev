use std::sync::Arc;

use tauri::State;

use crate::{
    database::composite_task_status_to_string,
    entities::{
        AIAgentType, AgentTask, BaseRemote, CompositeTask, CompositeTaskStatus, ExecutionLog,
        MergeStrategy, Repository, UnitTask, UnitTaskStatus, VCSProviderType,
    },
    services::{make_branch_name_unique, AppState, ConcurrencyError},
};

/// Gets the primary repository from a repository group.
/// Returns the first repository in the group.
async fn get_primary_repository_from_group(
    state: &AppState,
    repository_group_id: &str,
) -> Result<Repository, String> {
    let group = state
        .repository_group_service
        .get(repository_group_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Repository group not found: {}", repository_group_id))?;

    if group.repository_ids.is_empty() {
        return Err(format!(
            "Repository group {} has no repositories",
            repository_group_id
        ));
    }

    state
        .repository_service
        .get(&group.repository_ids[0])
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Repository not found: {}", group.repository_ids[0]))
}

/// Enriches InProgress tasks with execution status by checking Docker container
/// state. This uses a single Docker API call to get all running containers,
/// then checks each task. For non-InProgress tasks, is_executing is set to
/// false without Docker API calls.
async fn enrich_tasks_with_execution_status(
    tasks: Vec<UnitTask>,
    docker_service: &Option<Arc<crate::services::DockerService>>,
) -> Vec<UnitTask> {
    let Some(docker) = docker_service else {
        return tasks;
    };

    // Get all running container names in a single API call
    let running_containers: Vec<String> = docker
        .list_running_container_names()
        .await
        .unwrap_or_default();

    let mut enriched_tasks = Vec::new();
    for mut task in tasks {
        // Only check container status for tasks marked as InProgress
        if task.status == UnitTaskStatus::InProgress {
            let container_name = format!("delidev-{}", task.id);
            // Check if this task's container is in the running list
            task.is_executing = Some(running_containers.contains(&container_name));
        } else {
            task.is_executing = Some(false);
        }
        enriched_tasks.push(task);
    }
    enriched_tasks
}

/// Sets is_executing to false for all tasks in the list.
/// Used for tasks that are not InProgress and cannot be executing.
fn set_not_executing(mut tasks: Vec<UnitTask>) -> Vec<UnitTask> {
    for task in &mut tasks {
        task.is_executing = Some(false);
    }
    tasks
}

/// Validates that a task_id is safe to use in file paths.
/// Task IDs must be valid UUIDs (alphanumeric with hyphens only).
fn validate_task_id(task_id: &str) -> Result<(), String> {
    // UUID format: 8-4-4-4-12 hex characters with hyphens
    // Example: 550e8400-e29b-41d4-a716-446655440000
    if task_id.is_empty() {
        return Err("Task ID cannot be empty".to_string());
    }

    // Check that all characters are alphanumeric or hyphens
    if !task_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(format!(
            "Task ID '{}' contains invalid characters. Only alphanumeric characters and hyphens \
             are allowed.",
            task_id
        ));
    }

    // Check for path traversal attempts
    if task_id.contains("..") || task_id.starts_with('/') || task_id.starts_with('\\') {
        return Err(format!(
            "Task ID '{}' contains path traversal characters",
            task_id
        ));
    }

    Ok(())
}

/// Validates that a branch name follows Git's rules for valid reference names.
/// See https://git-scm.com/docs/git-check-ref-format for the full specification.
fn validate_git_branch_name(branch_name: &str) -> Result<(), String> {
    let name = branch_name.trim();

    if name.is_empty() {
        return Err("Branch name cannot be empty".to_string());
    }

    // Cannot contain consecutive dots
    if name.contains("..") {
        return Err("Branch name cannot contain '..'".to_string());
    }

    // Cannot contain control characters (ASCII < 32 or 127)
    if name.chars().any(|c| (c as u32) < 32 || c as u32 == 127) {
        return Err("Branch name cannot contain control characters".to_string());
    }

    // Cannot contain space, ~, ^, :, ?, *, [, \
    let forbidden_chars = [' ', '~', '^', ':', '?', '*', '[', '\\'];
    for c in forbidden_chars {
        if name.contains(c) {
            return Err(format!("Branch name cannot contain '{}'", c));
        }
    }

    // Cannot start with a dot
    if name.starts_with('.') {
        return Err("Branch name cannot start with '.'".to_string());
    }

    // Cannot end with a dot
    if name.ends_with('.') {
        return Err("Branch name cannot end with '.'".to_string());
    }

    // Cannot start with a slash
    if name.starts_with('/') {
        return Err("Branch name cannot start with '/'".to_string());
    }

    // Cannot end with a slash
    if name.ends_with('/') {
        return Err("Branch name cannot end with '/'".to_string());
    }

    // Cannot contain consecutive slashes
    if name.contains("//") {
        return Err("Branch name cannot contain '//'".to_string());
    }

    // Cannot end with .lock
    if name.ends_with(".lock") {
        return Err("Branch name cannot end with '.lock'".to_string());
    }

    // Cannot contain @{
    if name.contains("@{") {
        return Err("Branch name cannot contain '@{'".to_string());
    }

    // Cannot be the single character @
    if name == "@" {
        return Err("Branch name cannot be '@'".to_string());
    }

    Ok(())
}

/// Lists unit tasks
#[tauri::command]
pub async fn list_unit_tasks(
    state: State<'_, Arc<AppState>>,
    repository_id: Option<String>,
) -> Result<Vec<UnitTask>, String> {
    let tasks = state
        .task_service
        .list_unit_tasks(repository_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // Enrich tasks with execution status
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };
    Ok(enrich_tasks_with_execution_status(tasks, &docker_service).await)
}

/// Gets a unit task by ID
#[tauri::command]
pub async fn get_unit_task(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<UnitTask>, String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    let task = state
        .task_service
        .get_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?;

    // Enrich task with execution status if it exists
    if let Some(task) = task {
        let docker_service = {
            let guard = state.docker_service.read().await;
            guard.as_ref().cloned()
        };
        let mut tasks = enrich_tasks_with_execution_status(vec![task], &docker_service).await;
        Ok(tasks.pop())
    } else {
        Ok(None)
    }
}

/// Gets an agent task by ID
#[tauri::command]
pub async fn get_agent_task(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<AgentTask>, String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    state
        .task_service
        .get_agent_task(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Generates a title from a prompt by taking the first line, truncated to 80
/// characters. Uses character-aware truncation to avoid panics on multi-byte
/// UTF-8 characters.
fn generate_title_from_prompt(prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or(prompt).trim();
    if first_line.is_empty() {
        return "Untitled Task".to_string();
    }
    // Use char_indices to safely count characters and find byte boundary
    let char_count = first_line.chars().count();
    if char_count <= 80 {
        first_line.to_string()
    } else {
        // Take first 77 characters and add "..."
        let truncated: String = first_line.chars().take(77).collect();
        format!("{}...", truncated)
    }
}

/// Generates a title and branch name from a prompt using the webapp API
/// Returns None if the API call fails (e.g., no license, network error)
async fn try_generate_title_and_branch_from_api(
    license_service: &crate::services::LicenseService,
    prompt: &str,
) -> Option<(String, String)> {
    tracing::info!("Attempting to generate title and branch name via AI");

    // Check license status
    let license_info = license_service.get_license_info().await;
    if license_info.status != crate::entities::LicenseStatus::Active {
        tracing::info!(
            "AI title generation skipped: license not active (status: {:?})",
            license_info.status
        );
        return None;
    }

    // Get the license key for the API call
    let license_key = match license_service.get_license_key().await {
        Some(key) => key,
        None => {
            tracing::warn!("AI title generation skipped: no license key available");
            return None;
        }
    };

    // Call the webapp API
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to build HTTP client for AI title generation: {}", e);
            return None;
        }
    };

    let webapp_url =
        std::env::var("DELIDEV_WEBAPP_URL").unwrap_or_else(|_| "https://deli.dev".to_string());

    tracing::debug!(
        "Calling AI title generation API at {}/api/generate-title-branch",
        webapp_url
    );

    let response = match client
        .post(format!("{}/api/generate-title-branch", webapp_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "prompt": prompt,
            "licenseKey": license_key
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("AI title generation API request failed: {}", e);
            return None;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!(
            "AI title generation API returned error status {}: {}",
            status,
            error_text
        );
        return None;
    }

    let result: GeneratedTaskInfo = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to parse AI title generation response: {}", e);
            return None;
        }
    };

    tracing::info!(
        "AI generated title: '{}', branch: '{}'",
        result.title,
        result.branch_name
    );

    Some((result.title, result.branch_name))
}

/// Creates a new unit task
#[tauri::command]
pub async fn create_unit_task(
    state: State<'_, Arc<AppState>>,
    repository_group_id: String,
    prompt: String,
    title: Option<String>,
    branch_name: Option<String>,
    agent_type: Option<AIAgentType>,
) -> Result<UnitTask, String> {
    // Get repository group and resolve the primary repository
    let group = state
        .repository_group_service
        .get(&repository_group_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Repository group not found")?;

    if group.repository_ids.is_empty() {
        return Err("Repository group has no repositories".to_string());
    }

    let repo = state
        .repository_service
        .get(&group.repository_ids[0])
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Repository not found")?;

    // Generate title and branch if not provided
    // Try AI generation first (requires valid license), fall back to simple
    // generation
    // Note: AI-generated branch names get a unique suffix to prevent conflicts
    let (task_title, generated_branch) = match (&title, &branch_name) {
        (Some(t), Some(b)) => (t.clone(), Some(b.clone())),
        (Some(t), None) => {
            // Title provided, try to generate branch from API
            let branch = if let Some((_, b)) =
                try_generate_title_and_branch_from_api(&state.license_service, &prompt).await
            {
                // Add unique suffix to AI-generated branch name to prevent conflicts
                Some(make_branch_name_unique(&b))
            } else {
                None
            };
            (t.clone(), branch)
        }
        (None, Some(b)) => {
            // Branch provided, try to generate title from API
            let title = if let Some((t, _)) =
                try_generate_title_and_branch_from_api(&state.license_service, &prompt).await
            {
                t
            } else {
                generate_title_from_prompt(&prompt)
            };
            (title, Some(b.clone()))
        }
        (None, None) => {
            // Neither provided, try to generate both from API
            if let Some((t, b)) =
                try_generate_title_and_branch_from_api(&state.license_service, &prompt).await
            {
                // Add unique suffix to AI-generated branch name to prevent conflicts
                (t, Some(make_branch_name_unique(&b)))
            } else {
                (generate_title_from_prompt(&prompt), None)
            }
        }
    };

    // Create agent task with optional agent type
    let agent_task_id = uuid::Uuid::new_v4().to_string();
    let mut agent_task = AgentTask::new(
        agent_task_id.clone(),
        vec![BaseRemote {
            git_remote_dir_path: repo.local_path.clone(),
            git_branch_name: repo.default_branch.clone(),
        }],
    );

    // Set agent type if provided
    if let Some(at) = agent_type {
        agent_task = agent_task.with_agent_type(at);
    }

    state
        .task_service
        .create_agent_task(&agent_task)
        .await
        .map_err(|e| e.to_string())?;

    // Create unit task with title and optional branch
    let task_id = uuid::Uuid::new_v4().to_string();
    let mut task = UnitTask::new(
        task_id,
        task_title,
        prompt,
        agent_task_id,
        repository_group_id,
    );

    if let Some(branch) = generated_branch {
        task = task.with_branch_name(branch);
    }

    state
        .task_service
        .create_unit_task(&task)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task)
}

/// Updates unit task status
#[tauri::command]
pub async fn update_unit_task_status(
    state: State<'_, Arc<AppState>>,
    id: String,
    status: UnitTaskStatus,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    // Get current task to retrieve old status and title
    let task = state
        .task_service
        .get_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    let old_status = task.status;
    let task_title = task.title.clone();

    // Update status in database
    state
        .task_service
        .update_unit_task_status(&id, status)
        .await
        .map_err(|e| e.to_string())?;

    // Cleanup worktree when transitioning to terminal states (Done or Rejected)
    if status == UnitTaskStatus::Done || status == UnitTaskStatus::Rejected {
        if let Err(e) = cleanup_worktree_for_task(&state, &task).await {
            tracing::warn!("Failed to cleanup worktree for task {}: {}", id, e);
            // Don't fail the status update, just log the warning
        }
    }

    // Send notification if status changed
    state
        .notification_service
        .notify_task_status_change(&id, &task_title, old_status, status);

    // Check if this unit task is part of a composite task and if all nodes are now
    // complete
    if status == UnitTaskStatus::Done {
        if let Ok(Some(composite_task_id)) = state
            .task_service
            .check_and_complete_composite_task(&id)
            .await
        {
            // Emit status change event for the composite task
            state.emit_task_status_change(&composite_task_id, "in_progress", "done");

            // Send desktop notification for composite task completion
            state
                .notification_service
                .notify_composite_task_status_change(
                    &composite_task_id,
                    CompositeTaskStatus::InProgress,
                    CompositeTaskStatus::Done,
                );
        }

        // Trigger execution of dependent tasks whose dependencies are now satisfied
        // This handles the case where a task is manually marked as Done (e.g., after
        // review)
        let exec_service = {
            let guard = state.agent_execution_service.read().await;
            guard.as_ref().cloned()
        };

        if let Some(exec_service) = exec_service {
            let id_clone = id.clone();
            tauri::async_runtime::spawn(async move {
                exec_service.trigger_dependent_tasks(&id_clone).await;
            });
        } else {
            tracing::warn!(
                "Cannot trigger dependent tasks for {}: agent execution service not available",
                id
            );
        }
    }

    Ok(())
}

/// Renames the branch for a unit task
#[tauri::command]
pub async fn rename_unit_task_branch(
    state: State<'_, Arc<AppState>>,
    id: String,
    branch_name: String,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    // Validate branch name follows Git's rules
    validate_git_branch_name(&branch_name)?;

    // Get the task to verify it exists
    let task = state
        .task_service
        .get_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Only allow renaming for tasks that are in progress and don't have an existing
    // branch (i.e., before execution starts)
    if task.status != UnitTaskStatus::InProgress {
        return Err("Branch can only be renamed before the task starts executing".to_string());
    }

    // Check if a git worktree/branch has already been created (indicates execution
    // has started) base_commit is set when the worktree is created during
    // execution
    if task.base_commit.is_some() {
        return Err("Cannot rename branch after execution has started".to_string());
    }

    // Update the branch name in database
    state
        .task_service
        .update_unit_task_branch_name(&id, branch_name.trim())
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!("Branch name updated for task {}: {}", id, branch_name);
    Ok(())
}

/// Requests changes for a unit task by appending feedback to the prompt and
/// setting status back to in_progress
#[tauri::command]
pub async fn request_unit_task_changes(
    state: State<'_, Arc<AppState>>,
    id: String,
    feedback: String,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    // Get the task to verify it exists
    let task = state
        .task_service
        .get_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Only allow requesting changes for tasks that are in review
    if task.status != UnitTaskStatus::InReview {
        return Err("Can only request changes for tasks that are in review".to_string());
    }

    // Append feedback to the existing prompt
    let new_prompt = format!(
        "{}\n\n---\n\n## Feedback from review\n\n{}",
        task.prompt.trim(),
        feedback.trim()
    );

    // Update the prompt in database
    state
        .task_service
        .update_unit_task_prompt(&id, &new_prompt)
        .await
        .map_err(|e| e.to_string())?;

    // Update status back to in_progress
    let old_status = task.status;
    state
        .task_service
        .update_unit_task_status(&id, UnitTaskStatus::InProgress)
        .await
        .map_err(|e| e.to_string())?;

    // Send notification
    state.notification_service.notify_task_status_change(
        &id,
        &task.title,
        old_status,
        UnitTaskStatus::InProgress,
    );

    tracing::info!("Changes requested for task {}", id);
    Ok(())
}

/// Deletes a unit task
#[tauri::command]
pub async fn delete_unit_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    // Check if task exists
    let task = state
        .task_service
        .get_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Check if task is referenced by a composite task
    let is_in_composite = state
        .task_service
        .is_unit_task_in_composite(&id)
        .await
        .map_err(|e| e.to_string())?;

    if is_in_composite {
        return Err("Cannot delete a unit task that is part of a composite task".to_string());
    }

    // Cleanup resources (worktree, container)
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    if let Some(docker_service) = docker_service {
        let container_name = format!("delidev-{}", id);
        if let Err(e) = docker_service.stop_container(&container_name).await {
            tracing::warn!("Failed to stop container {}: {}", container_name, e);
        }
        if let Err(e) = docker_service.remove_container(&container_name).await {
            tracing::warn!("Failed to remove container {}: {}", container_name, e);
        }
    }

    if let Ok(repo) = get_primary_repository_from_group(&state, &task.repository_group_id).await {
        let repo_path = std::path::PathBuf::from(&repo.local_path);
        let base_tmp = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", id));

        if worktree_path.exists() {
            let _ = state
                .git_service
                .remove_worktree(&repo_path, &worktree_path);
        }
    }

    // Delete from database
    state
        .task_service
        .delete_unit_task(&id)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!("Unit task deleted: {}", id);
    Ok(())
}

/// Lists composite tasks
#[tauri::command]
pub async fn list_composite_tasks(
    state: State<'_, Arc<AppState>>,
    repository_id: Option<String>,
) -> Result<Vec<CompositeTask>, String> {
    state
        .task_service
        .list_composite_tasks(repository_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Gets a composite task by ID
#[tauri::command]
pub async fn get_composite_task(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<CompositeTask>, String> {
    state
        .task_service
        .get_composite_task(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Creates a new composite task
#[tauri::command]
pub async fn create_composite_task(
    state: State<'_, Arc<AppState>>,
    repository_group_id: String,
    prompt: String,
    title: Option<String>,
    planning_agent_type: Option<AIAgentType>,
    execution_agent_type: Option<AIAgentType>,
) -> Result<CompositeTask, String> {
    // Get repository group and resolve the primary repository
    let group = state
        .repository_group_service
        .get(&repository_group_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Repository group not found")?;

    if group.repository_ids.is_empty() {
        return Err("Repository group has no repositories".to_string());
    }

    let repo = state
        .repository_service
        .get(&group.repository_ids[0])
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Repository not found")?;

    // Generate title if not provided
    // Try AI generation first (requires valid license), fall back to simple
    // generation
    let task_title = match &title {
        Some(t) => t.clone(),
        None => {
            // Try to generate title from API
            if let Some((t, _)) =
                try_generate_title_and_branch_from_api(&state.license_service, &prompt).await
            {
                t
            } else {
                generate_title_from_prompt(&prompt)
            }
        }
    };

    // Create planning agent task with optional agent type
    let planning_task_id = uuid::Uuid::new_v4().to_string();
    let mut planning_task = AgentTask::new(
        planning_task_id.clone(),
        vec![BaseRemote {
            git_remote_dir_path: repo.local_path.clone(),
            git_branch_name: repo.default_branch.clone(),
        }],
    );

    // Set planning agent type if provided
    if let Some(at) = planning_agent_type {
        planning_task = planning_task.with_agent_type(at);
    }

    state
        .task_service
        .create_agent_task(&planning_task)
        .await
        .map_err(|e| e.to_string())?;

    // Create composite task with title
    let task_id = uuid::Uuid::new_v4().to_string();
    let mut task = CompositeTask::new(
        task_id,
        task_title,
        prompt,
        planning_task_id,
        repository_group_id,
    );

    // Set execution agent type if provided
    if let Some(at) = execution_agent_type {
        task = task.with_execution_agent_type(at);
    }

    state
        .task_service
        .create_composite_task(&task)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task)
}

/// Updates composite task status
#[tauri::command]
pub async fn update_composite_task_status(
    state: State<'_, Arc<AppState>>,
    id: String,
    status: CompositeTaskStatus,
) -> Result<(), String> {
    // Get the current task to determine old status
    let task = state
        .task_service
        .get_composite_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    let old_status = composite_task_status_to_string(task.status);
    let new_status = composite_task_status_to_string(status);

    state
        .task_service
        .update_composite_task_status(&id, status)
        .await
        .map_err(|e| e.to_string())?;

    // Emit status change event so frontend can update UI
    state.emit_task_status_change(&id, old_status, new_status);

    // Send desktop notification for composite task status change
    state
        .notification_service
        .notify_composite_task_status_change(&id, task.status, status);

    Ok(())
}

/// Deletes a composite task and all its unit tasks
#[tauri::command]
pub async fn delete_composite_task(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&id)?;

    // Check if composite task exists
    let task = state
        .task_service
        .get_composite_task(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Get all unit task IDs belonging to this composite task
    let unit_task_ids = state
        .task_service
        .get_composite_task_unit_task_ids(&id)
        .await
        .map_err(|e| e.to_string())?;

    // Cleanup resources (worktree, container) for each unit task
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    // Get repository for worktree cleanup
    let repo = get_primary_repository_from_group(&state, &task.repository_group_id)
        .await
        .ok();

    // Delete each unit task with resource cleanup
    for unit_task_id in &unit_task_ids {
        // Stop and remove Docker container
        if let Some(ref docker_service) = docker_service {
            let container_name = format!("delidev-{}", unit_task_id);
            if let Err(e) = docker_service.stop_container(&container_name).await {
                tracing::warn!(
                    "Failed to stop container {} for unit task {}: {}",
                    container_name,
                    unit_task_id,
                    e
                );
            }
            if let Err(e) = docker_service.remove_container(&container_name).await {
                tracing::warn!(
                    "Failed to remove container {} for unit task {}: {}",
                    container_name,
                    unit_task_id,
                    e
                );
            }
        }

        // Remove git worktree
        if let Some(ref repo) = repo {
            let repo_path = std::path::PathBuf::from(&repo.local_path);
            let base_tmp = std::env::temp_dir()
                .canonicalize()
                .unwrap_or_else(|_| std::env::temp_dir());
            let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", unit_task_id));

            if worktree_path.exists() {
                if let Err(e) = state
                    .git_service
                    .remove_worktree(&repo_path, &worktree_path)
                {
                    tracing::warn!(
                        "Failed to remove worktree for unit task {}: {}",
                        unit_task_id,
                        e
                    );
                }
            }
        }

        // Delete unit task from database
        if let Err(e) = state.task_service.delete_unit_task(unit_task_id).await {
            tracing::warn!("Failed to delete unit task {}: {}", unit_task_id, e);
        }
    }

    // Remove planning worktree for the composite task
    if let Some(ref repo) = repo {
        let repo_path = std::path::PathBuf::from(&repo.local_path);
        let base_tmp = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        let planning_worktree_path = base_tmp.join(format!("delidev/planning/{}", id));

        if planning_worktree_path.exists() {
            if let Err(e) = state
                .git_service
                .remove_worktree(&repo_path, &planning_worktree_path)
            {
                tracing::warn!(
                    "Failed to remove planning worktree for composite task {}: {}",
                    id,
                    e
                );
            }
        }
    }

    // Delete the composite task (cascade deletes composite_task_nodes)
    state
        .task_service
        .delete_composite_task(&id)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Composite task deleted: {} (with {} unit tasks)",
        id,
        unit_task_ids.len()
    );
    Ok(())
}

/// Gets tasks grouped by status (for Kanban view)
/// If workspace_id is provided, only returns tasks belonging to that workspace
#[tauri::command]
pub async fn get_tasks_by_status(
    state: State<'_, Arc<AppState>>,
    workspace_id: Option<String>,
) -> Result<TasksByStatus, String> {
    // Get Docker service for checking execution status
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    // Get repository group IDs for the workspace (if provided)
    let workspace_group_ids: Option<std::collections::HashSet<String>> =
        if let Some(ws_id) = &workspace_id {
            let groups = state
                .repository_group_service
                .list_by_workspace(ws_id)
                .await
                .map_err(|e| e.to_string())?;
            Some(groups.into_iter().map(|g| g.id).collect())
        } else {
            None
        };

    // Helper closure to filter tasks by workspace
    let filter_unit_tasks = |tasks: Vec<UnitTask>| -> Vec<UnitTask> {
        if let Some(ref group_ids) = workspace_group_ids {
            tasks
                .into_iter()
                .filter(|t| group_ids.contains(&t.repository_group_id))
                .collect()
        } else {
            tasks
        }
    };

    let filter_composite_tasks = |tasks: Vec<CompositeTask>| -> Vec<CompositeTask> {
        if let Some(ref group_ids) = workspace_group_ids {
            tasks
                .into_iter()
                .filter(|t| group_ids.contains(&t.repository_group_id))
                .collect()
        } else {
            tasks
        }
    };

    // Get blocked unit task IDs (unit tasks that belong to a composite task
    // and have unmet dependencies)
    let blocked_unit_task_ids = state
        .task_service
        .get_blocked_unit_task_ids()
        .await
        .map_err(|e| e.to_string())?;

    // Get unit tasks by status
    // Only InProgress tasks need Docker status checking - others are set to
    // is_executing=false
    let unit_in_progress = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::InProgress)
        .await
        .map_err(|e| e.to_string())?;
    // Filter out blocked unit tasks from in_progress list
    let unit_in_progress: Vec<UnitTask> = unit_in_progress
        .into_iter()
        .filter(|task| !blocked_unit_task_ids.contains(&task.id))
        .collect();
    let unit_in_progress = filter_unit_tasks(unit_in_progress);
    let unit_in_progress =
        enrich_tasks_with_execution_status(unit_in_progress, &docker_service).await;

    let in_review = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::InReview)
        .await
        .map_err(|e| e.to_string())?;
    let in_review = set_not_executing(filter_unit_tasks(in_review));

    let approved = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::Approved)
        .await
        .map_err(|e| e.to_string())?;
    let approved = set_not_executing(filter_unit_tasks(approved));

    let pr_open = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::PrOpen)
        .await
        .map_err(|e| e.to_string())?;
    let pr_open = set_not_executing(filter_unit_tasks(pr_open));

    let unit_done = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::Done)
        .await
        .map_err(|e| e.to_string())?;
    let unit_done = set_not_executing(filter_unit_tasks(unit_done));

    let unit_rejected = state
        .task_service
        .get_unit_tasks_by_status(UnitTaskStatus::Rejected)
        .await
        .map_err(|e| e.to_string())?;
    let unit_rejected = set_not_executing(filter_unit_tasks(unit_rejected));

    // Get composite tasks by status
    // Planning and PendingApproval are considered "in_progress" for the kanban view
    let composite_planning = state
        .task_service
        .get_composite_tasks_by_status(CompositeTaskStatus::Planning)
        .await
        .map_err(|e| e.to_string())?;
    let composite_planning = filter_composite_tasks(composite_planning);

    let composite_pending_approval = state
        .task_service
        .get_composite_tasks_by_status(CompositeTaskStatus::PendingApproval)
        .await
        .map_err(|e| e.to_string())?;
    let composite_pending_approval = filter_composite_tasks(composite_pending_approval);

    let composite_in_progress = state
        .task_service
        .get_composite_tasks_by_status(CompositeTaskStatus::InProgress)
        .await
        .map_err(|e| e.to_string())?;
    let composite_in_progress = filter_composite_tasks(composite_in_progress);

    let composite_done = state
        .task_service
        .get_composite_tasks_by_status(CompositeTaskStatus::Done)
        .await
        .map_err(|e| e.to_string())?;
    let composite_done = filter_composite_tasks(composite_done);

    let composite_rejected = state
        .task_service
        .get_composite_tasks_by_status(CompositeTaskStatus::Rejected)
        .await
        .map_err(|e| e.to_string())?;
    let composite_rejected = filter_composite_tasks(composite_rejected);

    // Combine planning and in_progress composite tasks into in_progress
    // PendingApproval is grouped into in_review
    let mut all_composite_in_progress = composite_planning;
    all_composite_in_progress.extend(composite_in_progress);

    Ok(TasksByStatus {
        in_progress: unit_in_progress,
        in_review,
        approved,
        pr_open,
        done: unit_done,
        rejected: unit_rejected,
        composite_in_progress: all_composite_in_progress,
        composite_in_review: composite_pending_approval,
        composite_done,
        composite_rejected,
    })
}

#[derive(serde::Serialize)]
pub struct TasksByStatus {
    pub in_progress: Vec<UnitTask>,
    pub in_review: Vec<UnitTask>,
    pub approved: Vec<UnitTask>,
    pub pr_open: Vec<UnitTask>,
    pub done: Vec<UnitTask>,
    pub rejected: Vec<UnitTask>,
    pub composite_in_progress: Vec<CompositeTask>,
    pub composite_in_review: Vec<CompositeTask>,
    pub composite_done: Vec<CompositeTask>,
    pub composite_rejected: Vec<CompositeTask>,
}

/// Starts execution of a unit task
#[tauri::command]
pub async fn start_task_execution(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    // Try to initialize Docker service if needed
    if !state.try_init_docker_service().await {
        return Err(
            "Docker/Podman is not available. Please install and start your container runtime to \
             execute tasks."
                .to_string(),
        );
    }

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        guard
            .as_ref()
            .ok_or("Execution service not available")?
            .clone()
    };

    // Get the task
    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Get agent task from database (contains ai_agent_type set at creation time)
    let agent_task = state
        .task_service
        .get_agent_task(&task.agent_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Agent task not found")?;

    // Atomically check concurrency limits and register task (premium feature)
    // The TaskGuard ensures automatic cleanup even if execution panics
    let task_guard = {
        let global_config = state.global_config.read().await;
        match state
            .concurrency_service
            .try_start_task(&global_config.concurrency, &task_id)
            .await
        {
            Ok(guard) => guard,
            Err(ConcurrencyError::LimitReached { current, limit }) => {
                // Add to pending queue instead of returning error
                state.concurrency_service.add_pending_task(&task_id).await;
                tracing::info!(
                    "Task {} queued for execution (concurrency limit: {}/{})",
                    task_id,
                    current,
                    limit
                );
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        }
    };

    // Execute in background with TaskGuard for panic safety
    let task_clone = task.clone();
    let task_service = state.task_service.clone();
    tauri::async_runtime::spawn(async move {
        // TaskGuard is moved into this closure - it will auto-unregister on drop
        let _guard = task_guard;

        if let Err(e) = exec_service
            .execute_unit_task(&task_clone, &agent_task)
            .await
        {
            tracing::error!("Task execution failed: {}", e);
        }

        // After execution completes, check if task is Done and trigger dependent tasks
        // This handles the case where the task had no changes and went directly to Done
        if let Ok(Some(updated_task)) = task_service.get_unit_task(&task_clone.id).await {
            if updated_task.status == UnitTaskStatus::Done {
                exec_service.trigger_dependent_tasks(&task_clone.id).await;
            }
        }

        // TaskGuard drops here, automatically unregistering the task
    });

    Ok(())
}

/// Gets execution logs for a session
#[tauri::command]
pub async fn get_execution_logs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<ExecutionLog>, String> {
    let guard = state.agent_execution_service.read().await;
    let exec_service = guard.as_ref().ok_or("Execution service not available")?;

    Ok(exec_service.get_logs(&session_id).await)
}

/// Gets all execution logs
#[tauri::command]
pub async fn get_all_execution_logs(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ExecutionLog>, String> {
    let guard = state.agent_execution_service.read().await;
    let exec_service = guard.as_ref().ok_or("Execution service not available")?;

    Ok(exec_service.get_all_logs().await)
}

/// Gets agent stream messages for a session
#[tauri::command]
pub async fn get_stream_messages(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<crate::entities::AgentStreamMessageEntry>, String> {
    state
        .task_service
        .get_stream_messages(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets historical execution logs for a session from the database
#[tauri::command]
pub async fn get_historical_execution_logs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<ExecutionLog>, String> {
    state
        .task_service
        .get_execution_logs(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Cleans up resources for a task (worktree, container)
#[tauri::command]
pub async fn cleanup_task(state: State<'_, Arc<AppState>>, task_id: String) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    let guard = state.agent_execution_service.read().await;
    let exec_service = guard.as_ref().ok_or("Execution service not available")?;

    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    exec_service
        .cleanup_task(&task)
        .await
        .map_err(|e| e.to_string())
}

/// Checks if Docker is available, trying to initialize if not already done
#[tauri::command]
pub async fn is_docker_available(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    // Try to initialize docker service if needed
    Ok(state.try_init_docker_service().await)
}

/// Checks if a task is currently executing (i.e., its Docker container is
/// running)
#[tauri::command]
pub async fn is_task_executing(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<bool, String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    // Get Docker service
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    match docker_service {
        Some(docker_service) => {
            // Check if the container for this task is running
            let container_name = format!("delidev-{}", task_id);
            Ok(docker_service.is_container_running(&container_name).await)
        }
        None => Ok(false),
    }
}

/// Creates a PR for a task and returns the PR URL
/// This function uses an AI coding agent to create the PR, which allows for
/// more intelligent PR descriptions and handles the git operations.
#[tauri::command]
pub async fn create_pr_for_task(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<String, String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    tracing::info!("Creating PR for task: {}", task_id);

    // 1. Get the task
    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // 2. Get repository info from repository group
    let repo = get_primary_repository_from_group(&state, &task.repository_group_id).await?;

    // 3. Get VCS credentials
    let credentials = state.credentials.read().await;

    // 4. Resolve worktree path
    let base_tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task_id));

    if !worktree_path.exists() {
        return Err(format!(
            "Worktree not found for task '{}' at '{}'. The task may have been cleaned up.",
            task_id,
            worktree_path.display()
        ));
    }

    // 5. Get branch name from worktree
    let branch_name = state
        .git_service
        .current_branch(&worktree_path)
        .map_err(|e| format!("Failed to get current branch: {}", e))?;

    // 6. Handle PR creation based on VCS provider
    match repo.vcs_provider_type {
        VCSProviderType::GitHub => {
            let github_creds = credentials
                .github
                .as_ref()
                .ok_or("GitHub credentials not configured")?;

            // Parse owner/repo from remote URL
            let (owner, repo_name) =
                parse_github_url(&repo.remote_url).ok_or("Failed to parse GitHub URL")?;

            // Check if PR already exists for this branch
            if let Ok(Some(existing_pr)) = state
                .vcs_service
                .find_github_pr_by_branch(github_creds, &owner, &repo_name, &branch_name)
                .await
            {
                tracing::info!(
                    "Found existing PR for branch {}: {}",
                    branch_name,
                    existing_pr.url
                );

                // Update task with existing PR URL
                state
                    .task_service
                    .update_unit_task_pr_url(&task_id, &existing_pr.url)
                    .await
                    .map_err(|e| format!("Failed to update task PR URL: {}", e))?;

                // Update task status to PrOpen
                state
                    .task_service
                    .update_unit_task_status(&task_id, UnitTaskStatus::PrOpen)
                    .await
                    .map_err(|e| format!("Failed to update task status: {}", e))?;

                // Cleanup worktree
                let repo_path = std::path::PathBuf::from(&repo.local_path);
                if let Err(e) = state
                    .git_service
                    .remove_worktree(&repo_path, &worktree_path)
                {
                    tracing::warn!("Failed to cleanup worktree for task {}: {}", task_id, e);
                }

                return Ok(existing_pr.url);
            }

            // No existing PR found - use AI agent to create one
            tracing::info!("No existing PR found, using AI agent to create PR");

            // Get execution service
            let exec_service = {
                let guard = state.agent_execution_service.read().await;
                guard
                    .as_ref()
                    .ok_or("Execution service not available")?
                    .clone()
            };

            // Create an AgentTask for PR creation
            let agent_task_id = uuid::Uuid::new_v4().to_string();
            let agent_task = AgentTask::new(
                agent_task_id.clone(),
                vec![BaseRemote {
                    git_remote_dir_path: repo.local_path.clone(),
                    git_branch_name: repo.default_branch.clone(),
                }],
            );

            // Save the agent task to database
            state
                .task_service
                .create_agent_task(&agent_task)
                .await
                .map_err(|e| format!("Failed to create agent task: {}", e))?;

            // Build prompt for PR creation - keep it simple and let the agent determine
            // title/body from the diff
            let pr_prompt = format!(
                "Create a PR. Commit all the changes and push the branch if required.\n\nBranch: \
                 {}\nBase branch: {}\n\nDetermine the PR title and body from the diff. The PR \
                 body should include a summary of changes and end with '---\\n\\nGenerated by \
                 DeliDev'.",
                branch_name, repo.default_branch
            );

            // Execute the agent task
            let result = exec_service
                .execute_agent_task(&agent_task, &worktree_path, &pr_prompt, &task_id)
                .await
                .map_err(|e| format!("Failed to execute agent task: {}", e))?;

            if !result.success {
                return Err(format!(
                    "AI agent failed to create PR: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                ));
            }

            // Try to find the PR that was created by checking again
            let pr_url = if let Ok(Some(created_pr)) = state
                .vcs_service
                .find_github_pr_by_branch(github_creds, &owner, &repo_name, &branch_name)
                .await
            {
                created_pr.url
            } else {
                // Fallback: try to extract PR URL from agent output
                extract_pr_url_from_output(&result.output)
                    .ok_or("Could not find PR URL in agent output")?
            };

            // Update task with PR URL
            state
                .task_service
                .update_unit_task_pr_url(&task_id, &pr_url)
                .await
                .map_err(|e| format!("Failed to update task PR URL: {}", e))?;

            // Update task status to PrOpen (PR created, awaiting merge)
            state
                .task_service
                .update_unit_task_status(&task_id, UnitTaskStatus::PrOpen)
                .await
                .map_err(|e| format!("Failed to update task status: {}", e))?;

            // Cleanup worktree after PR is created (branch is pushed, worktree no longer
            // needed)
            let repo_path = std::path::PathBuf::from(&repo.local_path);
            if let Err(e) = state
                .git_service
                .remove_worktree(&repo_path, &worktree_path)
            {
                tracing::warn!("Failed to cleanup worktree for task {}: {}", task_id, e);
                // Don't fail the PR creation, just log the warning
            } else {
                tracing::info!("Cleaned up worktree for task {} after PR creation", task_id);
            }

            tracing::info!("PR created successfully: {}", pr_url);
            Ok(pr_url)
        }
        VCSProviderType::GitLab => Err("GitLab PR creation not yet implemented".to_string()),
        VCSProviderType::Bitbucket => Err("Bitbucket PR creation not yet implemented".to_string()),
    }
}

/// Extracts a GitHub PR URL from agent output text
fn extract_pr_url_from_output(output: &str) -> Option<String> {
    // Look for GitHub PR URL patterns
    let patterns = [r"https://github\.com/[^/]+/[^/]+/pull/\d+"];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(m) = re.find(output) {
                return Some(m.as_str().to_string());
            }
        }
    }

    None
}

/// Commits worktree changes to the main repository's current branch
#[tauri::command]
pub async fn commit_to_repository(
    state: State<'_, Arc<AppState>>,
    task_id: String,
    merge_strategy: Option<MergeStrategy>,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    let strategy = merge_strategy.unwrap_or_default();
    tracing::info!(
        "Committing changes for task: {} with strategy: {:?}",
        task_id,
        strategy
    );

    // 1. Get the task
    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // 2. Get repository info from repository group
    let repo = get_primary_repository_from_group(&state, &task.repository_group_id).await?;

    // 3. Resolve worktree path
    let base_tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task_id));

    if !worktree_path.exists() {
        return Err(format!(
            "Worktree not found for task '{}'. The task may have been cleaned up.",
            task_id
        ));
    }

    // 4. Get the task's branch name from worktree
    let task_branch = state
        .git_service
        .current_branch(&worktree_path)
        .map_err(|e| format!("Failed to get current branch: {}", e))?;

    // 5. Get the main repo path
    let repo_path = std::path::PathBuf::from(&repo.local_path);

    // 6. Merge the task branch into the current branch of the main repo using the
    //    selected strategy
    match strategy {
        MergeStrategy::Merge => {
            state
                .git_service
                .merge_branch(&repo_path, &task_branch)
                .map_err(|e| format!("Failed to merge branch: {}", e))?;
        }
        MergeStrategy::Squash => {
            state
                .git_service
                .squash_merge_branch(&repo_path, &task_branch)
                .map_err(|e| format!("Failed to squash merge branch: {}", e))?;
        }
        MergeStrategy::Rebase => {
            state
                .git_service
                .rebase_merge_branch(&repo_path, &task_branch)
                .map_err(|e| format!("Failed to rebase merge branch: {}", e))?;
        }
    }

    // 7. Update task status to Done
    state
        .task_service
        .update_unit_task_status(&task_id, UnitTaskStatus::Done)
        .await
        .map_err(|e| format!("Failed to update task status: {}", e))?;

    // 8. Cleanup worktree
    let _ = state
        .git_service
        .remove_worktree(&repo_path, &worktree_path);

    tracing::info!(
        "Changes committed successfully for task: {} using {:?} strategy",
        task_id,
        strategy
    );
    Ok(())
}

/// Parses GitHub URL to extract owner and repo name
fn parse_github_url(url: &str) -> Option<(String, String)> {
    // Handle HTTPS: https://github.com/owner/repo or https://github.com/owner/repo.git
    if url.starts_with("https://github.com/") {
        let path = url.strip_prefix("https://github.com/")?;
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    // Handle SSH: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.strip_prefix("git@github.com:")?;
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

/// Helper function to cleanup worktree for a task
async fn cleanup_worktree_for_task(
    state: &State<'_, Arc<AppState>>,
    task: &UnitTask,
) -> Result<(), String> {
    // Get repository info from repository group
    let repo = get_primary_repository_from_group(state, &task.repository_group_id).await?;

    let repo_path = std::path::PathBuf::from(&repo.local_path);
    let base_tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task.id));

    // Remove worktree if it exists
    if worktree_path.exists() {
        state
            .git_service
            .remove_worktree(&repo_path, &worktree_path)
            .map_err(|e| format!("Failed to remove worktree: {}", e))?;
        tracing::info!(
            "Cleaned up worktree for task {}: {:?}",
            task.id,
            worktree_path
        );
    }

    // Also cleanup any leftover Docker container
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    if let Some(docker_service) = docker_service {
        let container_name = format!("delidev-{}", task.id);
        if let Err(e) = docker_service.stop_container(&container_name).await {
            tracing::warn!("Failed to stop container {}: {}", container_name, e);
        }
        if let Err(e) = docker_service.remove_container(&container_name).await {
            tracing::warn!("Failed to remove container {}: {}", container_name, e);
        }
    }

    Ok(())
}

/// Gets the git diff for a task's worktree or from main repo branch
#[tauri::command]
pub async fn get_task_diff(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<Option<String>, String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    // Get the task
    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Get repository info from repository group
    let repo = get_primary_repository_from_group(&state, &task.repository_group_id).await?;

    // Resolve worktree path
    let base_tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task_id));

    // Try worktree first
    if worktree_path.exists() {
        match state.git_service.get_diff(&worktree_path) {
            Ok(diff) => {
                if !diff.is_empty() {
                    return Ok(Some(diff));
                }
                // If diff is empty, try get_diff_from_base
                match state
                    .git_service
                    .get_diff_from_base(&worktree_path, &repo.default_branch)
                {
                    Ok(diff) if !diff.is_empty() => return Ok(Some(diff)),
                    Ok(_) => {
                        // Empty diff from base - fall through to main repo
                        // fallback
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get diff from worktree base: {}", e);
                        // Fall through to main repo fallback
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get diff from worktree: {}", e);
            }
        }
    }

    // Worktree doesn't exist - try to get diff from main repo using branch name and
    // base_commit
    let branch_name = match &task.branch_name {
        Some(name) => name,
        None => return Ok(None),
    };

    let repo_path = std::path::PathBuf::from(&repo.local_path);

    // Use stored base_commit and end_commit if available for accurate diff
    // This shows only the changes made by this specific task
    if let (Some(ref base_commit), Some(ref end_commit)) = (&task.base_commit, &task.end_commit) {
        match state
            .git_service
            .get_diff_between_commits(&repo_path, base_commit, end_commit)
        {
            Ok(diff) if !diff.is_empty() => return Ok(Some(diff)),
            Ok(_) => return Ok(None),
            Err(e) => {
                tracing::warn!(
                    "Failed to get diff between commits {} and {}: {}",
                    base_commit,
                    end_commit,
                    e
                );
                // Fall through to base_commit...HEAD fallback
            }
        }
    }

    // Fallback: Use base_commit to current HEAD (for tasks still in progress or
    // missing end_commit)
    if let Some(ref base_commit) = task.base_commit {
        match state
            .git_service
            .get_diff_from_commit(&repo_path, base_commit, branch_name)
        {
            Ok(diff) if !diff.is_empty() => return Ok(Some(diff)),
            Ok(_) => return Ok(None),
            Err(e) => {
                tracing::warn!("Failed to get diff from base commit {}: {}", base_commit, e);
                // Fall through to branch-based diff as fallback
            }
        }
    }

    // Final fallback: Use branch-based diff (may include unrelated changes if
    // default branch advanced)
    match state
        .git_service
        .get_diff_between_branches(&repo_path, branch_name, &repo.default_branch)
    {
        Ok(diff) if !diff.is_empty() => Ok(Some(diff)),
        Ok(_) => Ok(None),
        Err(e) => {
            tracing::warn!("Failed to get diff from main repo: {}", e);
            Ok(None)
        }
    }
}

/// Stops the execution of a running task
#[tauri::command]
pub async fn stop_task_execution(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<(), String> {
    // Validate task_id to prevent path traversal
    validate_task_id(&task_id)?;

    tracing::info!("Stopping execution for task: {}", task_id);

    // Get the task
    let task = state
        .task_service
        .get_unit_task(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;

    // Check if the task is in progress
    if task.status != UnitTaskStatus::InProgress {
        return Err("Task is not in progress".to_string());
    }

    // Get Docker service
    let docker_service = {
        let guard = state.docker_service.read().await;
        guard.as_ref().cloned()
    };

    if let Some(docker_service) = docker_service {
        // Stop and remove the container
        let container_name = format!("delidev-{}", task_id);
        if let Err(e) = docker_service.stop_container(&container_name).await {
            tracing::warn!("Failed to stop container {}: {}", container_name, e);
        }
        if let Err(e) = docker_service.remove_container(&container_name).await {
            tracing::warn!("Failed to remove container {}: {}", container_name, e);
        }
        tracing::info!("Container stopped: {}", container_name);
    }

    // Cleanup worktree
    if let Ok(repo) = get_primary_repository_from_group(&state, &task.repository_group_id).await {
        let repo_path = std::path::PathBuf::from(&repo.local_path);
        // Resolve symlinks in /tmp (macOS uses /tmp -> /private/tmp, which Podman
        // doesn't resolve)
        let base_tmp = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        let worktree_path = base_tmp.join(format!("delidev/worktrees/{}", task_id));

        if worktree_path.exists() {
            let _ = state
                .git_service
                .remove_worktree(&repo_path, &worktree_path);
            tracing::info!("Worktree removed: {:?}", worktree_path);
        }
    }

    // Update task status to indicate it was stopped
    // Keep it as InProgress so user can restart it
    tracing::info!("Task {} execution stopped", task_id);

    Ok(())
}

/// Starts the planning phase for a composite task
/// This creates an agent session that generates a PLAN-{randomString}.yaml file
#[tauri::command]
pub async fn start_composite_task_planning(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
) -> Result<String, String> {
    tracing::info!(
        "Starting planning for composite task: {}",
        composite_task_id
    );

    // Get the composite task
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Verify task is in Planning state
    if composite_task.status != CompositeTaskStatus::Planning {
        return Err("Composite task is not in planning state".to_string());
    }

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        guard
            .as_ref()
            .ok_or("Execution service not available")?
            .clone()
    };

    // Create planning service
    let planning_service = crate::services::CompositePlanningService::new(
        state.task_service.clone(),
        state.repository_service.clone(),
        state.repository_group_service.clone(),
        state.git_service.clone(),
    );

    // Clone for status update
    let task_service = state.task_service.clone();

    // Start planning
    let plan_filename = planning_service
        .start_planning(&composite_task, &exec_service)
        .await
        .map_err(|e| e.to_string())?;

    // Check if auto-approval is enabled
    let should_auto_approve = {
        // Get repository from repository group
        let repo =
            get_primary_repository_from_group(&state, &composite_task.repository_group_id).await?;

        // Load repository config
        let repo_path = std::path::PathBuf::from(&repo.local_path);
        let repo_config = match crate::config::ConfigManager::load_repository_config(&repo_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    repo_path = %repo_path.display(),
                    "Failed to load repository config, using default configuration"
                );
                Default::default()
            }
        };

        // Get global config
        let global_config = state.global_config.read().await;

        // Check effective auto-approve setting
        repo_config.effective_composite_task_auto_approve(&global_config)
    };

    if should_auto_approve {
        tracing::info!(
            "Auto-approval enabled for composite task: {}",
            composite_task_id
        );

        // First update status to PendingApproval (required for approve_plan)
        task_service
            .update_composite_task_status(&composite_task_id, CompositeTaskStatus::PendingApproval)
            .await
            .map_err(|e| e.to_string())?;

        // Re-fetch with updated status
        let composite_task = state
            .task_service
            .get_composite_task(&composite_task_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Composite task not found after status update")?;

        // Approve the plan (creates UnitTasks and CompositeTaskNodes)
        planning_service
            .approve_plan(&composite_task)
            .await
            .map_err(|e| e.to_string())?;

        // Emit status change event so frontend can update UI
        state.emit_task_status_change(&composite_task_id, "planning", "in_progress");

        // Send desktop notification for planning completion with auto-approval
        state
            .notification_service
            .notify_composite_task_status_change(
                &composite_task_id,
                CompositeTaskStatus::Planning,
                CompositeTaskStatus::InProgress,
            );

        tracing::info!(
            "Planning completed and auto-approved for composite task: {}, plan file: {}",
            composite_task_id,
            plan_filename
        );

        // Start execution of tasks
        start_composite_task_execution(&state, &composite_task_id).await?;
    } else {
        // Update status to PendingApproval
        task_service
            .update_composite_task_status(&composite_task_id, CompositeTaskStatus::PendingApproval)
            .await
            .map_err(|e| e.to_string())?;

        // Emit status change event so frontend can update UI
        state.emit_task_status_change(&composite_task_id, "planning", "pending_approval");

        // Send desktop notification for planning completion
        state
            .notification_service
            .notify_composite_task_status_change(
                &composite_task_id,
                CompositeTaskStatus::Planning,
                CompositeTaskStatus::PendingApproval,
            );

        tracing::info!(
            "Planning completed for composite task: {}, plan file: {}",
            composite_task_id,
            plan_filename
        );
    }

    Ok(plan_filename)
}

/// Helper function to start execution of composite task nodes
async fn start_composite_task_execution(
    state: &State<'_, Arc<AppState>>,
    composite_task_id: &str,
) -> Result<(), String> {
    // Try to initialize Docker service if needed for execution
    if !state.try_init_docker_service().await {
        tracing::warn!("Docker/Podman is not available. Tasks will need to be started manually.");
        return Ok(());
    }

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        match guard.as_ref() {
            Some(service) => service.clone(),
            None => {
                tracing::warn!(
                    "Execution service not available. Tasks will need to be started manually."
                );
                return Ok(());
            }
        }
    };

    // Re-fetch the composite task to get the newly created nodes
    let composite_task = state
        .task_service
        .get_composite_task(composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Find executable nodes (nodes with no dependencies can start immediately)
    let executable_nodes = composite_task.executable_nodes(&[]);

    tracing::info!(
        "Starting {} executable nodes for composite task: {}",
        executable_nodes.len(),
        composite_task_id
    );

    // Start execution of each executable node
    for node in executable_nodes {
        // Get the UnitTask for this node
        let unit_task = match state.task_service.get_unit_task(&node.unit_task_id).await {
            Ok(Some(task)) => task,
            Ok(None) => {
                tracing::error!("UnitTask not found for node: {}", node.id);
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to get UnitTask for node {}: {}", node.id, e);
                continue;
            }
        };

        // Get agent task
        let agent_task = match state
            .task_service
            .get_agent_task(&unit_task.agent_task_id)
            .await
        {
            Ok(Some(task)) => task,
            Ok(None) => {
                tracing::error!("Agent task not found for unit task: {}", unit_task.id);
                continue;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get agent task for unit task {}: {}",
                    unit_task.id,
                    e
                );
                continue;
            }
        };

        // Atomically check concurrency limits and register task (premium feature)
        let task_guard = {
            let global_config = state.global_config.read().await;
            match state
                .concurrency_service
                .try_start_task(&global_config.concurrency, &unit_task.id)
                .await
            {
                Ok(guard) => guard,
                Err(ConcurrencyError::LimitReached { current, limit }) => {
                    // Add to pending queue instead of skipping
                    state
                        .concurrency_service
                        .add_pending_task(&unit_task.id)
                        .await;
                    tracing::info!(
                        "Node {} task {} queued for execution (concurrency limit: {}/{})",
                        node.id,
                        unit_task.id,
                        current,
                        limit
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        "Cannot start node {} due to concurrency error: {}",
                        node.id,
                        e
                    );
                    continue;
                }
            }
        };

        // Start execution in background with TaskGuard for panic safety
        let exec_service_clone = exec_service.clone();
        let unit_task_clone = unit_task.clone();
        let agent_task_clone = agent_task.clone();

        tauri::async_runtime::spawn(async move {
            // TaskGuard is moved into this closure - it will auto-unregister on drop
            let _guard = task_guard;

            if let Err(e) = exec_service_clone
                .execute_unit_task(&unit_task_clone, &agent_task_clone)
                .await
            {
                tracing::error!(
                    "Task execution failed for node {}: {}",
                    unit_task_clone.id,
                    e
                );
            }
            // TaskGuard drops here, automatically unregistering the task
        });

        tracing::info!("Started execution of unit task: {}", unit_task.id);
    }

    Ok(())
}

/// Gets the plan content for a composite task
/// Returns stored content from database if available, otherwise reads from file
#[tauri::command]
pub async fn get_composite_task_plan(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
) -> Result<Option<String>, String> {
    // Get the composite task
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Check if plan file path is set
    if composite_task.plan_file_path.is_none() {
        return Ok(None);
    }

    // Create planning service to get plan content (handles DB caching and file
    // reading)
    let planning_service = crate::services::CompositePlanningService::new(
        state.task_service.clone(),
        state.repository_service.clone(),
        state.repository_group_service.clone(),
        state.git_service.clone(),
    );

    match planning_service.get_plan_content(&composite_task).await {
        Ok(content) => Ok(Some(content)),
        Err(e) => {
            tracing::warn!("Failed to get plan content: {}", e);
            Ok(None)
        }
    }
}

/// Approves the plan for a composite task and starts execution
#[tauri::command]
pub async fn approve_composite_task_plan(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
) -> Result<(), String> {
    tracing::info!("Approving plan for composite task: {}", composite_task_id);

    // Get the composite task
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Create planning service
    let planning_service = crate::services::CompositePlanningService::new(
        state.task_service.clone(),
        state.repository_service.clone(),
        state.repository_group_service.clone(),
        state.git_service.clone(),
    );

    // Approve the plan (creates UnitTasks and CompositeTaskNodes)
    // Note: approve_plan internally updates status from pending_approval to
    // in_progress
    planning_service
        .approve_plan(&composite_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit status change event so frontend can update UI
    state.emit_task_status_change(&composite_task_id, "pending_approval", "in_progress");

    // Send desktop notification for plan approval
    state
        .notification_service
        .notify_composite_task_status_change(
            &composite_task_id,
            CompositeTaskStatus::PendingApproval,
            CompositeTaskStatus::InProgress,
        );

    tracing::info!("Plan approved for composite task: {}", composite_task_id);

    // Try to initialize Docker service if needed for execution
    if !state.try_init_docker_service().await {
        tracing::warn!("Docker/Podman is not available. Tasks will need to be started manually.");
        return Ok(());
    }

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        match guard.as_ref() {
            Some(service) => service.clone(),
            None => {
                tracing::warn!(
                    "Execution service not available. Tasks will need to be started manually."
                );
                return Ok(());
            }
        }
    };

    // Re-fetch the composite task to get the newly created nodes
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found after approval")?;

    // Find executable nodes (nodes with no dependencies can start immediately)
    let executable_nodes = composite_task.executable_nodes(&[]);

    tracing::info!(
        "Starting {} executable nodes for composite task: {}",
        executable_nodes.len(),
        composite_task_id
    );

    // Start execution of each executable node
    for node in executable_nodes {
        // Get the UnitTask for this node
        let unit_task = match state.task_service.get_unit_task(&node.unit_task_id).await {
            Ok(Some(task)) => task,
            Ok(None) => {
                tracing::error!("UnitTask not found for node: {}", node.id);
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to get UnitTask for node {}: {}", node.id, e);
                continue;
            }
        };

        // Get agent task
        let agent_task = match state
            .task_service
            .get_agent_task(&unit_task.agent_task_id)
            .await
        {
            Ok(Some(task)) => task,
            Ok(None) => {
                tracing::error!("Agent task not found for unit task: {}", unit_task.id);
                continue;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get agent task for unit task {}: {}",
                    unit_task.id,
                    e
                );
                continue;
            }
        };

        // Atomically check concurrency limits and register task (premium feature)
        let task_guard = {
            let global_config = state.global_config.read().await;
            match state
                .concurrency_service
                .try_start_task(&global_config.concurrency, &unit_task.id)
                .await
            {
                Ok(guard) => guard,
                Err(ConcurrencyError::LimitReached { current, limit }) => {
                    // Add to pending queue instead of skipping
                    state
                        .concurrency_service
                        .add_pending_task(&unit_task.id)
                        .await;
                    tracing::info!(
                        "Node {} task {} queued for execution (concurrency limit: {}/{})",
                        node.id,
                        unit_task.id,
                        current,
                        limit
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        "Cannot start node {} due to concurrency error: {}",
                        node.id,
                        e
                    );
                    continue;
                }
            }
        };

        // Start execution in background with TaskGuard for panic safety
        let exec_service_clone = exec_service.clone();
        let unit_task_clone = unit_task.clone();
        let agent_task_clone = agent_task.clone();

        tauri::async_runtime::spawn(async move {
            // TaskGuard is moved into this closure - it will auto-unregister on drop
            let _guard = task_guard;

            if let Err(e) = exec_service_clone
                .execute_unit_task(&unit_task_clone, &agent_task_clone)
                .await
            {
                tracing::error!(
                    "Task execution failed for node {}: {}",
                    unit_task_clone.id,
                    e
                );
            }
            // TaskGuard drops here, automatically unregistering the task
        });

        tracing::info!("Started execution of unit task: {}", unit_task.id);
    }

    Ok(())
}

/// Rejects the plan for a composite task
#[tauri::command]
pub async fn reject_composite_task_plan(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
) -> Result<(), String> {
    tracing::info!("Rejecting plan for composite task: {}", composite_task_id);

    // Get the composite task
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Create planning service
    let planning_service = crate::services::CompositePlanningService::new(
        state.task_service.clone(),
        state.repository_service.clone(),
        state.repository_group_service.clone(),
        state.git_service.clone(),
    );

    // Store the previous status for event emission and notification
    let old_composite_status = composite_task.status;

    // Reject the plan
    planning_service
        .reject_plan(&composite_task)
        .await
        .map_err(|e| e.to_string())?;

    // Emit status change event so frontend can update UI
    state.emit_task_status_change(
        &composite_task_id,
        composite_task_status_to_string(old_composite_status),
        "rejected",
    );

    // Send desktop notification for plan rejection
    state
        .notification_service
        .notify_composite_task_status_change(
            &composite_task_id,
            old_composite_status,
            CompositeTaskStatus::Rejected,
        );

    tracing::info!("Plan rejected for composite task: {}", composite_task_id);

    Ok(())
}

/// Updates the plan for a composite task based on user feedback
/// This runs a new Claude session with the current plan and user's update
/// request
#[tauri::command]
pub async fn update_composite_task_plan(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
    update_request: String,
) -> Result<String, String> {
    tracing::info!(
        "Updating plan for composite task: {} with request: {}",
        composite_task_id,
        update_request
    );

    // Get the composite task
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Verify task is in PendingApproval state
    if composite_task.status != CompositeTaskStatus::PendingApproval {
        return Err("Composite task is not in pending approval state".to_string());
    }

    // Update status to Planning before starting the update
    state
        .task_service
        .update_composite_task_status(&composite_task_id, CompositeTaskStatus::Planning)
        .await
        .map_err(|e| e.to_string())?;

    // Emit status change event so frontend can update UI
    state.emit_task_status_change(&composite_task_id, "pending_approval", "planning");

    // Send desktop notification for status change
    state
        .notification_service
        .notify_composite_task_status_change(
            &composite_task_id,
            CompositeTaskStatus::PendingApproval,
            CompositeTaskStatus::Planning,
        );

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        guard
            .as_ref()
            .ok_or("Execution service not available")?
            .clone()
    };

    // Create planning service
    let planning_service = crate::services::CompositePlanningService::new(
        state.task_service.clone(),
        state.repository_service.clone(),
        state.repository_group_service.clone(),
        state.git_service.clone(),
    );

    // Update the plan
    let plan_filename = planning_service
        .update_plan(&composite_task, &update_request, &exec_service)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Plan updated for composite task: {}, new plan file: {}",
        composite_task_id,
        plan_filename
    );

    Ok(plan_filename)
}

/// Executes the next available nodes in a composite task
/// Returns the list of UnitTask IDs that were started
#[tauri::command]
pub async fn execute_composite_task_nodes(
    state: State<'_, Arc<AppState>>,
    composite_task_id: String,
) -> Result<Vec<String>, String> {
    tracing::info!("Executing nodes for composite task: {}", composite_task_id);

    // Get the composite task with nodes
    let composite_task = state
        .task_service
        .get_composite_task(&composite_task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Composite task not found")?;

    // Verify task is in InProgress state
    if composite_task.status != CompositeTaskStatus::InProgress {
        return Err("Composite task is not in progress".to_string());
    }

    // Get execution service
    let exec_service = {
        let guard = state.agent_execution_service.read().await;
        guard
            .as_ref()
            .ok_or("Execution service not available")?
            .clone()
    };

    // Find completed nodes by checking UnitTask status
    let mut completed_node_ids: Vec<String> = Vec::new();
    for node in &composite_task.nodes {
        if let Ok(Some(unit_task)) = state.task_service.get_unit_task(&node.unit_task_id).await {
            if unit_task.status == UnitTaskStatus::Done {
                completed_node_ids.push(node.id.clone());
            }
        }
    }

    // Find executable nodes (dependencies satisfied, not yet completed/in progress)
    let executable_nodes = composite_task.executable_nodes(&completed_node_ids);

    let mut started_task_ids: Vec<String> = Vec::new();

    for node in executable_nodes {
        // Get the UnitTask for this node
        let unit_task = state
            .task_service
            .get_unit_task(&node.unit_task_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or(format!("UnitTask not found for node: {}", node.id))?;

        // Only start if task is not already in progress or done
        if unit_task.status != UnitTaskStatus::InProgress
            && unit_task.status != UnitTaskStatus::Done
        {
            // Atomically check concurrency limits and register task (premium feature)
            let task_guard = {
                let global_config = state.global_config.read().await;
                match state
                    .concurrency_service
                    .try_start_task(&global_config.concurrency, &unit_task.id)
                    .await
                {
                    Ok(guard) => guard,
                    Err(ConcurrencyError::LimitReached { current, limit }) => {
                        // Add to pending queue instead of skipping
                        state
                            .concurrency_service
                            .add_pending_task(&unit_task.id)
                            .await;
                        tracing::info!(
                            "Task {} queued for execution (concurrency limit: {}/{})",
                            unit_task.id,
                            current,
                            limit
                        );
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Cannot start task {} due to concurrency error: {}",
                            unit_task.id,
                            e
                        );
                        continue;
                    }
                }
            };

            // Get agent task
            let agent_task = state
                .task_service
                .get_agent_task(&unit_task.agent_task_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("Agent task not found")?;

            // Start execution in background with TaskGuard for panic safety
            let exec_service_clone = exec_service.clone();
            let unit_task_clone = unit_task.clone();
            let agent_task_clone = agent_task.clone();

            tauri::async_runtime::spawn(async move {
                // TaskGuard is moved into this closure - it will auto-unregister on drop
                let _guard = task_guard;

                if let Err(e) = exec_service_clone
                    .execute_unit_task(&unit_task_clone, &agent_task_clone)
                    .await
                {
                    tracing::error!(
                        "Task execution failed for node {}: {}",
                        unit_task_clone.id,
                        e
                    );
                }
                // TaskGuard drops here, automatically unregistering the task
            });

            started_task_ids.push(unit_task.id.clone());
        }
    }

    // Check if all nodes are completed
    if completed_node_ids.len() == composite_task.nodes.len() && !composite_task.nodes.is_empty() {
        // All nodes completed, update composite task status to Done
        state
            .task_service
            .update_composite_task_status(&composite_task_id, CompositeTaskStatus::Done)
            .await
            .map_err(|e| e.to_string())?;

        // Emit status change event so frontend can update UI
        state.emit_task_status_change(&composite_task_id, "in_progress", "done");

        // Send desktop notification for composite task completion
        state
            .notification_service
            .notify_composite_task_status_change(
                &composite_task_id,
                CompositeTaskStatus::InProgress,
                CompositeTaskStatus::Done,
            );
    }

    tracing::info!(
        "Started {} tasks for composite task: {}",
        started_task_ids.len(),
        composite_task_id
    );

    Ok(started_task_ids)
}

/// Response from the title/branch generation endpoint
#[derive(serde::Deserialize, serde::Serialize)]
pub struct GeneratedTaskInfo {
    pub title: String,
    #[serde(rename = "branchName")]
    pub branch_name: String,
}

/// Token usage information
#[derive(serde::Serialize)]
pub struct TokenUsage {
    /// Total cost in USD
    pub total_cost_usd: Option<f64>,
    /// Total duration in milliseconds
    pub total_duration_ms: Option<f64>,
}

/// Gets token usage for a unit task (includes main task and all auto-fix tasks)
#[tauri::command]
pub async fn get_unit_task_token_usage(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<TokenUsage, String> {
    let (total_cost_usd, total_duration_ms) = state
        .task_service
        .get_unit_task_token_usage(&task_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TokenUsage {
        total_cost_usd,
        total_duration_ms,
    })
}

/// Gets token usage for a composite task (includes planning task and all unit
/// tasks)
#[tauri::command]
pub async fn get_composite_task_token_usage(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<TokenUsage, String> {
    let (total_cost_usd, total_duration_ms) = state
        .task_service
        .get_composite_task_token_usage(&task_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TokenUsage {
        total_cost_usd,
        total_duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let result = parse_github_url("https://github.com/owner/repo");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let result = parse_github_url("https://github.com/owner/repo.git");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let result = parse_github_url("git@github.com:owner/repo.git");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_url_ssh_without_git() {
        let result = parse_github_url("git@github.com:owner/repo");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert_eq!(parse_github_url("not-a-url"), None);
        assert_eq!(parse_github_url("https://gitlab.com/owner/repo"), None);
        assert_eq!(parse_github_url("git@gitlab.com:owner/repo"), None);
    }

    #[test]
    fn test_parse_github_url_with_extra_path() {
        // URL with extra path segments should still work (extracts first two segments)
        let result = parse_github_url("https://github.com/owner/repo/tree/main");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_task_id("abc-123").is_ok());
        assert!(validate_task_id("simple").is_ok());
    }

    #[test]
    fn test_validate_task_id_empty() {
        assert!(validate_task_id("").is_err());
    }

    #[test]
    fn test_validate_task_id_path_traversal() {
        assert!(validate_task_id("../etc/passwd").is_err());
        assert!(validate_task_id("..").is_err());
        assert!(validate_task_id("/etc/passwd").is_err());
        assert!(validate_task_id("\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_task_id_special_chars() {
        assert!(validate_task_id("task;rm -rf /").is_err());
        assert!(validate_task_id("task$(whoami)").is_err());
        assert!(validate_task_id("task`id`").is_err());
        assert!(validate_task_id("task|cat /etc/passwd").is_err());
    }

    #[test]
    fn test_validate_git_branch_name_valid() {
        assert!(validate_git_branch_name("feature/add-login").is_ok());
        assert!(validate_git_branch_name("fix/bug-123").is_ok());
        assert!(validate_git_branch_name("main").is_ok());
        assert!(validate_git_branch_name("release-1.0.0").is_ok());
        assert!(validate_git_branch_name("user/feature").is_ok());
    }

    #[test]
    fn test_validate_git_branch_name_empty() {
        assert!(validate_git_branch_name("").is_err());
        assert!(validate_git_branch_name("   ").is_err());
    }

    #[test]
    fn test_validate_git_branch_name_consecutive_dots() {
        assert!(validate_git_branch_name("feature..test").is_err());
        assert!(validate_git_branch_name("a..b..c").is_err());
    }

    #[test]
    fn test_validate_git_branch_name_forbidden_chars() {
        assert!(validate_git_branch_name("feature branch").is_err()); // space
        assert!(validate_git_branch_name("feature~test").is_err()); // tilde
        assert!(validate_git_branch_name("feature^test").is_err()); // caret
        assert!(validate_git_branch_name("feature:test").is_err()); // colon
        assert!(validate_git_branch_name("feature?test").is_err()); // question mark
        assert!(validate_git_branch_name("feature*test").is_err()); // asterisk
        assert!(validate_git_branch_name("feature[test").is_err()); // bracket
        assert!(validate_git_branch_name("feature\\test").is_err()); // backslash
    }

    #[test]
    fn test_validate_git_branch_name_dot_rules() {
        assert!(validate_git_branch_name(".hidden").is_err()); // starts with dot
        assert!(validate_git_branch_name("feature.").is_err()); // ends with dot
        assert!(validate_git_branch_name("feature.lock").is_err()); // ends with
                                                                    // .lock
    }

    #[test]
    fn test_validate_git_branch_name_slash_rules() {
        assert!(validate_git_branch_name("/feature").is_err()); // starts with slash
        assert!(validate_git_branch_name("feature/").is_err()); // ends with slash
        assert!(validate_git_branch_name("feature//test").is_err()); // consecutive slashes
    }

    #[test]
    fn test_validate_git_branch_name_at_rules() {
        assert!(validate_git_branch_name("@").is_err()); // single @
        assert!(validate_git_branch_name("feature@{test}").is_err()); // contains @{
    }

    #[test]
    fn test_validate_git_branch_name_trims_whitespace() {
        assert!(validate_git_branch_name("  feature/test  ").is_ok());
    }

    #[test]
    fn test_extract_pr_url_basic() {
        let output = "Creating PR...\nhttps://github.com/owner/repo/pull/123\nDone!";
        assert_eq!(
            extract_pr_url_from_output(output),
            Some("https://github.com/owner/repo/pull/123".to_string())
        );
    }

    #[test]
    fn test_extract_pr_url_with_surrounding_text() {
        let output =
            "PR created successfully: https://github.com/delinoio/delidev/pull/42 - please review";
        assert_eq!(
            extract_pr_url_from_output(output),
            Some("https://github.com/delinoio/delidev/pull/42".to_string())
        );
    }

    #[test]
    fn test_extract_pr_url_multiline() {
        let output = r#"
            Running gh pr create...

            https://github.com/org/project/pull/999

            PR has been created.
        "#;
        assert_eq!(
            extract_pr_url_from_output(output),
            Some("https://github.com/org/project/pull/999".to_string())
        );
    }

    #[test]
    fn test_extract_pr_url_no_match() {
        let output = "No PR URL in this output";
        assert_eq!(extract_pr_url_from_output(output), None);
    }

    #[test]
    fn test_extract_pr_url_empty_output() {
        assert_eq!(extract_pr_url_from_output(""), None);
    }

    #[test]
    fn test_extract_pr_url_invalid_format() {
        // Should not match invalid PR URLs
        assert_eq!(
            extract_pr_url_from_output("https://github.com/owner/repo/pulls"),
            None
        );
        assert_eq!(
            extract_pr_url_from_output("https://github.com/owner/repo/pull/"),
            None
        );
        assert_eq!(
            extract_pr_url_from_output("https://gitlab.com/owner/repo/pull/123"),
            None
        );
    }

    #[test]
    fn test_extract_pr_url_first_match() {
        // Should return the first match if there are multiple URLs
        let output = "First: https://github.com/owner/repo/pull/1\nSecond: https://github.com/owner/repo/pull/2";
        assert_eq!(
            extract_pr_url_from_output(output),
            Some("https://github.com/owner/repo/pull/1".to_string())
        );
    }

    #[test]
    fn test_generate_title_from_prompt_short() {
        let prompt = "Add user authentication";
        assert_eq!(
            generate_title_from_prompt(prompt),
            "Add user authentication"
        );
    }

    #[test]
    fn test_generate_title_from_prompt_multiline() {
        let prompt =
            "Add user authentication\nThis is a detailed description of what we need to do.";
        assert_eq!(
            generate_title_from_prompt(prompt),
            "Add user authentication"
        );
    }

    #[test]
    fn test_generate_title_from_prompt_long() {
        let prompt = "This is a very long prompt that exceeds eighty characters and should be \
                      truncated appropriately";
        let title = generate_title_from_prompt(prompt);
        // Title should be 77 chars + "..." = 80 chars for ASCII
        assert_eq!(title.chars().count(), 80);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_generate_title_from_prompt_exactly_80() {
        let prompt = "A".repeat(80);
        assert_eq!(generate_title_from_prompt(&prompt), prompt);
    }

    #[test]
    fn test_generate_title_from_prompt_empty() {
        assert_eq!(generate_title_from_prompt(""), "Untitled Task");
    }

    #[test]
    fn test_generate_title_from_prompt_whitespace_only() {
        assert_eq!(generate_title_from_prompt("   "), "Untitled Task");
        assert_eq!(generate_title_from_prompt("\t\n"), "Untitled Task");
    }

    #[test]
    fn test_generate_title_from_prompt_unicode_short() {
        // Test with emojis and non-ASCII characters
        let prompt = "Add 日本語 support with emoji 🎉";
        assert_eq!(
            generate_title_from_prompt(prompt),
            "Add 日本語 support with emoji 🎉"
        );
    }

    #[test]
    fn test_generate_title_from_prompt_unicode_long() {
        // Create a prompt with multi-byte UTF-8 characters that exceeds 80 chars
        // Each emoji is 4 bytes but counts as 1 character
        let prompt = "🎉".repeat(100);
        let title = generate_title_from_prompt(&prompt);
        // Should be 77 emojis + "..." = 80 characters
        assert_eq!(title.chars().count(), 80);
        assert!(title.ends_with("..."));
        // Verify no panic occurred and we have valid UTF-8
        assert!(title.is_char_boundary(0));
    }

    #[test]
    fn test_generate_title_from_prompt_mixed_unicode_truncation() {
        // Mix of ASCII and multi-byte characters near the 80 char boundary
        let prompt = "Hello 世界! ".repeat(20); // This will exceed 80 chars
        let title = generate_title_from_prompt(&prompt);
        assert_eq!(title.chars().count(), 80);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_generate_title_from_prompt_exactly_81_chars() {
        // Boundary case: exactly 81 characters should be truncated
        let prompt = "A".repeat(81);
        let title = generate_title_from_prompt(&prompt);
        assert_eq!(title.chars().count(), 80);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_generate_title_from_prompt_trims_whitespace() {
        let prompt = "  Add feature  ";
        assert_eq!(generate_title_from_prompt(prompt), "Add feature");
    }

    #[test]
    fn test_make_branch_name_unique_format() {
        let branch = make_branch_name_unique("feature/add-login");
        // Should have format: original-xxxxxxxx (8 char suffix)
        assert!(branch.starts_with("feature/add-login-"));
        assert_eq!(branch.len(), "feature/add-login-".len() + 8);
    }

    #[test]
    fn test_make_branch_name_unique_generates_different_suffixes() {
        let branch1 = make_branch_name_unique("feature/test");
        let branch2 = make_branch_name_unique("feature/test");
        // Should generate different unique suffixes
        assert_ne!(branch1, branch2);
    }

    #[test]
    fn test_make_branch_name_unique_preserves_original() {
        let original = "fix/critical-bug";
        let unique = make_branch_name_unique(original);
        assert!(unique.starts_with("fix/critical-bug-"));
    }
}
