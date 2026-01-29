use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use thiserror::Error;
use tokio::sync::RwLock;

use super::{GitService, RepositoryService, TaskService};
use crate::entities::UnitTaskStatus;

/// Default cleanup interval: 1 hour
const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Delidev worktree base directory pattern
const DELIDEV_WORKTREE_DIR: &str = "delidev";

#[derive(Error, Debug)]
pub enum WorktreeCleanupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Git error: {0}")]
    Git(#[from] super::GitError),
}

pub type WorktreeCleanupResult<T> = Result<T, WorktreeCleanupError>;

/// Service for periodically cleaning up orphaned worktrees.
///
/// For each repository, if a worktree directory matches the delidev pattern
/// (e.g., `/tmp/delidev/worktrees/{task_id}`) and it's not connected to any
/// active task, the worktree will be cleaned up.
pub struct WorktreeCleanupService {
    git_service: Arc<GitService>,
    task_service: Arc<TaskService>,
    repository_service: Arc<RepositoryService>,
    /// Cleanup interval in seconds
    interval_secs: u64,
    /// Flag to stop the cleanup loop
    stop_flag: Arc<RwLock<bool>>,
}

impl WorktreeCleanupService {
    /// Creates a new WorktreeCleanupService with default interval
    pub fn new(
        git_service: Arc<GitService>,
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
    ) -> Self {
        Self {
            git_service,
            task_service,
            repository_service,
            interval_secs: DEFAULT_CLEANUP_INTERVAL_SECS,
            stop_flag: Arc::new(RwLock::new(false)),
        }
    }

    /// Creates a new WorktreeCleanupService with custom interval
    pub fn with_interval(mut self, interval_secs: u64) -> Self {
        self.interval_secs = interval_secs;
        self
    }

    /// Starts the periodic cleanup loop.
    /// This spawns a background task that runs cleanup at the configured
    /// interval.
    pub fn start(&self) {
        let git_service = self.git_service.clone();
        let task_service = self.task_service.clone();
        let repository_service = self.repository_service.clone();
        let interval_secs = self.interval_secs;
        let stop_flag = self.stop_flag.clone();

        tokio::spawn(async move {
            tracing::info!(
                "Worktree cleanup service started with interval: {} seconds",
                interval_secs
            );

            // Run cleanup immediately on startup
            tracing::info!("Running initial worktree cleanup on startup...");
            if let Err(e) =
                cleanup_orphaned_worktrees(&git_service, &task_service, &repository_service).await
            {
                tracing::error!("Initial worktree cleanup failed: {}", e);
            }

            loop {
                // Check if we should stop
                if *stop_flag.read().await {
                    tracing::info!("Worktree cleanup service stopped");
                    break;
                }

                // Wait for the interval
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;

                // Check again after sleeping
                if *stop_flag.read().await {
                    tracing::info!("Worktree cleanup service stopped");
                    break;
                }

                // Run cleanup
                if let Err(e) =
                    cleanup_orphaned_worktrees(&git_service, &task_service, &repository_service)
                        .await
                {
                    tracing::error!("Worktree cleanup failed: {}", e);
                }
            }
        });
    }

    /// Stops the periodic cleanup loop
    pub async fn stop(&self) {
        *self.stop_flag.write().await = true;
    }

    /// Runs cleanup immediately (for manual triggering)
    pub async fn run_cleanup(&self) -> WorktreeCleanupResult<CleanupResult> {
        cleanup_orphaned_worktrees(
            &self.git_service,
            &self.task_service,
            &self.repository_service,
        )
        .await
    }
}

/// Result of a cleanup operation
#[derive(Debug, Default)]
pub struct CleanupResult {
    /// Number of worktrees that were cleaned up
    pub cleaned_count: usize,
    /// Paths of worktrees that were cleaned up
    pub cleaned_paths: Vec<PathBuf>,
    /// Errors encountered during cleanup (non-fatal)
    pub errors: Vec<String>,
}

/// Cleans up orphaned worktrees that are not connected to any active task.
///
/// This function:
/// 1. Gets all registered repositories
/// 2. Scans `/tmp/delidev/worktrees/` for worktree directories
/// 3. For each worktree, checks if there's an active task using it
/// 4. If no active task, removes the worktree
async fn cleanup_orphaned_worktrees(
    git_service: &GitService,
    task_service: &TaskService,
    repository_service: &RepositoryService,
) -> WorktreeCleanupResult<CleanupResult> {
    tracing::info!("Starting worktree cleanup...");

    let mut result = CleanupResult::default();

    // Get the base worktree directory
    // Resolve symlinks in /tmp (macOS uses /tmp -> /private/tmp)
    let base_tmp = PathBuf::from("/tmp")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let worktrees_dir = base_tmp.join(DELIDEV_WORKTREE_DIR).join("worktrees");

    // Check if the worktrees directory exists
    if !worktrees_dir.exists() {
        tracing::debug!("Worktrees directory does not exist: {:?}", worktrees_dir);
        return Ok(result);
    }

    // Get all active task IDs (tasks that are not Done or Rejected)
    let active_task_ids = get_active_task_ids(task_service).await?;
    tracing::debug!("Active task IDs: {:?}", active_task_ids);

    // Get all repositories for worktree removal
    let repositories = repository_service
        .list()
        .await
        .map_err(|e| WorktreeCleanupError::Database(e.to_string()))?;

    // Build a map of repository paths
    let repo_paths: Vec<PathBuf> = repositories
        .iter()
        .map(|r| PathBuf::from(&r.local_path))
        .collect();

    // Scan the worktrees directory
    let entries = match std::fs::read_dir(&worktrees_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("Failed to read worktrees directory: {}", e);
            return Ok(result);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Extract the task ID from the directory name
        let task_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        tracing::debug!("Checking worktree for task: {}", task_id);

        // Check if this task ID is in the active tasks set
        if active_task_ids.contains(&task_id) {
            tracing::debug!(
                "Worktree {} is connected to an active task, skipping",
                task_id
            );
            continue;
        }

        // This worktree is orphaned, try to clean it up
        tracing::info!("Cleaning up orphaned worktree: {:?}", path);

        // Try to find the parent repository and clean up properly
        let mut cleaned = false;
        for repo_path in &repo_paths {
            match git_service.remove_worktree(repo_path, &path) {
                Ok(()) => {
                    tracing::info!("Successfully cleaned up worktree: {:?}", path);
                    result.cleaned_count += 1;
                    result.cleaned_paths.push(path.clone());
                    cleaned = true;
                    break;
                }
                Err(e) => {
                    tracing::debug!("Failed to remove worktree from repo {:?}: {}", repo_path, e);
                }
            }
        }

        // If we couldn't clean up via git, try to just remove the directory
        if !cleaned && path.exists() {
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {
                    tracing::info!("Forcefully removed orphaned worktree directory: {:?}", path);
                    result.cleaned_count += 1;
                    result.cleaned_paths.push(path.clone());
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to remove worktree directory {:?}: {}", path, e);
                    tracing::warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }
    }

    // Also clean up planning worktrees
    let planning_dir = base_tmp.join(DELIDEV_WORKTREE_DIR).join("planning");
    if planning_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&planning_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Planning worktrees use composite task IDs
                let composite_task_id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // Check if this composite task is still in planning state
                let is_active_planning =
                    is_composite_task_planning(task_service, &composite_task_id).await;

                if is_active_planning {
                    tracing::debug!(
                        "Planning worktree {} is still active, skipping",
                        composite_task_id
                    );
                    continue;
                }

                // This planning worktree is orphaned
                tracing::info!("Cleaning up orphaned planning worktree: {:?}", path);

                // The planning branch name follows this pattern
                let planning_branch_name = format!("delidev/planning/{}", composite_task_id);

                // Try to remove via git first, then force remove
                let mut cleaned = false;
                for repo_path in &repo_paths {
                    if git_service.remove_worktree(repo_path, &path).is_ok() {
                        result.cleaned_count += 1;
                        result.cleaned_paths.push(path.clone());
                        cleaned = true;

                        // Also delete the planning branch
                        if let Err(e) = git_service.delete_branch(repo_path, &planning_branch_name)
                        {
                            tracing::warn!(
                                "Failed to delete planning branch '{}': {}",
                                planning_branch_name,
                                e
                            );
                        } else {
                            tracing::info!(
                                "Deleted orphaned planning branch: {}",
                                planning_branch_name
                            );
                        }
                        break;
                    }
                }

                if !cleaned && path.exists() {
                    match std::fs::remove_dir_all(&path) {
                        Ok(()) => {
                            result.cleaned_count += 1;
                            result.cleaned_paths.push(path.clone());

                            // Try to delete the planning branch from any repository
                            for repo_path in &repo_paths {
                                if git_service
                                    .delete_branch(repo_path, &planning_branch_name)
                                    .is_ok()
                                {
                                    tracing::info!(
                                        "Deleted orphaned planning branch: {}",
                                        planning_branch_name
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to remove planning worktree {:?}: {}", path, e);
                            tracing::warn!("{}", error_msg);
                            result.errors.push(error_msg);
                        }
                    }
                }
            }
        }
    }

    tracing::info!(
        "Worktree cleanup completed. Cleaned {} worktrees.",
        result.cleaned_count
    );

    Ok(result)
}

/// Gets the set of active task IDs (tasks that are InProgress or InReview)
async fn get_active_task_ids(task_service: &TaskService) -> WorktreeCleanupResult<HashSet<String>> {
    let mut active_ids = HashSet::new();

    // Get tasks in InProgress status
    let in_progress_tasks = task_service
        .get_unit_tasks_by_status(UnitTaskStatus::InProgress)
        .await
        .map_err(|e| WorktreeCleanupError::Database(e.to_string()))?;

    for task in in_progress_tasks {
        active_ids.insert(task.id);
    }

    // Get tasks in InReview status
    let in_review_tasks = task_service
        .get_unit_tasks_by_status(UnitTaskStatus::InReview)
        .await
        .map_err(|e| WorktreeCleanupError::Database(e.to_string()))?;

    for task in in_review_tasks {
        active_ids.insert(task.id);
    }

    // Get tasks in PrOpen status (these still need worktrees for updates)
    let pr_open_tasks = task_service
        .get_unit_tasks_by_status(UnitTaskStatus::PrOpen)
        .await
        .map_err(|e| WorktreeCleanupError::Database(e.to_string()))?;

    for task in pr_open_tasks {
        active_ids.insert(task.id);
    }

    Ok(active_ids)
}

/// Checks if a composite task is still in planning state
async fn is_composite_task_planning(task_service: &TaskService, composite_task_id: &str) -> bool {
    match task_service.get_composite_task(composite_task_id).await {
        Ok(Some(task)) => task.status == crate::entities::CompositeTaskStatus::Planning,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_result_default() {
        let result = CleanupResult::default();
        assert_eq!(result.cleaned_count, 0);
        assert!(result.cleaned_paths.is_empty());
        assert!(result.errors.is_empty());
    }
}
