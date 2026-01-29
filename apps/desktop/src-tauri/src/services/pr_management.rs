use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Duration,
};

use tokio::sync::{mpsc, RwLock};

use crate::{
    config::ConfigManager,
    entities::{
        AutoFixReviewFilter, Repository, RepositoryConfig, UnitTask, UnitTaskStatus,
        VCSProviderType,
    },
    services::{
        CICheckStatus, NotificationService, RepositoryGroupService, RepositoryService,
        TaskService, VCSProviderService,
    },
};

/// Auto-fix trigger type
#[derive(Debug, Clone)]
pub enum AutoFixTrigger {
    /// CI failure detected
    CIFailure {
        task_id: String,
        pr_number: u64,
        failed_checks: Vec<String>,
    },
    /// Review comment received
    ReviewComment {
        task_id: String,
        pr_number: u64,
        review_id: u64,
        author: String,
        body: String,
        file_path: Option<String>,
        line: Option<u64>,
    },
}

/// PR monitoring state for a single task
#[derive(Debug, Clone)]
struct PRMonitoringState {
    task_id: String,
    pr_number: u64,
    owner: String,
    repo: String,
    last_ci_status: Option<CICheckStatus>,
    processed_review_ids: HashSet<u64>,
    auto_fix_count: u32,
}

/// Service for managing automated PR operations
/// Monitors PRs for CI failures and review comments, triggering auto-fixes when appropriate
pub struct PRManagementService {
    task_service: Arc<TaskService>,
    repository_service: Arc<RepositoryService>,
    repository_group_service: Arc<RepositoryGroupService>,
    vcs_service: Arc<VCSProviderService>,
    notification_service: Arc<NotificationService>,
    config_manager: Arc<ConfigManager>,
    /// Tracking state for monitored PRs
    monitoring_state: Arc<RwLock<HashMap<String, PRMonitoringState>>>,
    /// Channel to send auto-fix triggers
    auto_fix_tx: Option<mpsc::UnboundedSender<AutoFixTrigger>>,
}

impl PRManagementService {
    pub fn new(
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
        repository_group_service: Arc<RepositoryGroupService>,
        vcs_service: Arc<VCSProviderService>,
        notification_service: Arc<NotificationService>,
        config_manager: Arc<ConfigManager>,
    ) -> Self {
        Self {
            task_service,
            repository_service,
            repository_group_service,
            vcs_service,
            notification_service,
            config_manager,
            monitoring_state: Arc::new(RwLock::new(HashMap::new())),
            auto_fix_tx: None,
        }
    }

    /// Sets the channel for sending auto-fix triggers
    pub fn set_auto_fix_sender(&mut self, tx: mpsc::UnboundedSender<AutoFixTrigger>) {
        self.auto_fix_tx = Some(tx);
    }

    /// Creates a channel for receiving auto-fix triggers
    pub fn create_auto_fix_channel(
        &mut self,
    ) -> mpsc::UnboundedReceiver<AutoFixTrigger> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.auto_fix_tx = Some(tx);
        rx
    }

    /// Starts monitoring a PR for a unit task
    pub async fn start_monitoring(&self, task_id: &str) -> Result<(), String> {
        let task = self
            .task_service
            .get_unit_task(task_id)
            .await
            .map_err(|e| format!("Failed to get task: {}", e))?
            .ok_or_else(|| format!("Task not found: {}", task_id))?;

        // Only monitor tasks with open PRs
        if task.status != UnitTaskStatus::PrOpen {
            return Err("Task does not have an open PR".to_string());
        }

        let pr_url = task
            .linked_pr_url
            .as_ref()
            .ok_or("Task has no linked PR URL")?;

        // Parse PR URL to extract owner, repo, and PR number
        let (owner, repo, pr_number) = parse_github_pr_url(pr_url)
            .ok_or_else(|| format!("Failed to parse PR URL: {}", pr_url))?;

        let state = PRMonitoringState {
            task_id: task_id.to_string(),
            pr_number,
            owner,
            repo,
            last_ci_status: None,
            processed_review_ids: HashSet::new(),
            auto_fix_count: task.auto_fix_task_ids.len() as u32,
        };

        let mut monitoring = self.monitoring_state.write().await;
        monitoring.insert(task_id.to_string(), state);

        tracing::info!(task_id = %task_id, pr_number = %pr_number, "Started PR monitoring");

        Ok(())
    }

    /// Stops monitoring a PR
    pub async fn stop_monitoring(&self, task_id: &str) {
        let mut monitoring = self.monitoring_state.write().await;
        if monitoring.remove(task_id).is_some() {
            tracing::info!(task_id = %task_id, "Stopped PR monitoring");
        }
    }

    /// Performs a single poll cycle for all monitored PRs
    /// Returns the number of auto-fix triggers generated
    pub async fn poll_once(&self) -> Result<usize, String> {
        let monitoring = self.monitoring_state.read().await;
        let task_ids: Vec<String> = monitoring.keys().cloned().collect();
        drop(monitoring);

        let mut trigger_count = 0;

        for task_id in task_ids {
            match self.check_task_pr(&task_id).await {
                Ok(count) => trigger_count += count,
                Err(e) => {
                    tracing::warn!(task_id = %task_id, error = %e, "Failed to check task PR");
                }
            }
        }

        Ok(trigger_count)
    }

    /// Checks a single task's PR for CI failures and review comments
    async fn check_task_pr(&self, task_id: &str) -> Result<usize, String> {
        let monitoring = self.monitoring_state.read().await;
        let state = monitoring
            .get(task_id)
            .ok_or_else(|| format!("Task {} not being monitored", task_id))?
            .clone();
        drop(monitoring);

        // Get the task and repository info
        let task = self
            .task_service
            .get_unit_task(task_id)
            .await
            .map_err(|e| format!("Failed to get task: {}", e))?
            .ok_or_else(|| format!("Task not found: {}", task_id))?;

        // Task is no longer in PrOpen status, stop monitoring
        if task.status != UnitTaskStatus::PrOpen {
            self.stop_monitoring(task_id).await;
            return Ok(0);
        }

        // Get repository to check VCS provider type
        let repo_group = self
            .repository_group_service
            .get(&task.repository_group_id)
            .await
            .map_err(|e| format!("Failed to get repo group: {}", e))?
            .ok_or_else(|| format!("Repository group not found: {}", task.repository_group_id))?;

        // For now, we only support single-repo groups
        let repo_id = repo_group
            .repository_ids
            .first()
            .ok_or("Repository group has no repositories")?;

        let repository = self
            .repository_service
            .get(repo_id)
            .await
            .map_err(|e| format!("Failed to get repository: {}", e))?
            .ok_or_else(|| format!("Repository not found: {}", repo_id))?;

        // Only GitHub is supported for now
        if repository.vcs_provider_type != VCSProviderType::GitHub {
            return Err(format!(
                "VCS provider {:?} not supported for PR management",
                repository.vcs_provider_type
            ));
        }

        // Get repository config for automation settings
        let repo_config = ConfigManager::load_repository_config(Path::new(&repository.local_path))
            .unwrap_or_default();

        // Get credentials
        let creds = self
            .config_manager
            .load_credentials()
            .map_err(|e| format!("Failed to get VCS credentials: {}", e))?;

        let github_creds = creds
            .github
            .as_ref()
            .ok_or("GitHub credentials not configured")?;

        let mut trigger_count = 0;

        // Check CI status if auto-fix is enabled
        if repo_config.automation.auto_fix_ci_failures {
            let ci_triggers = self
                .check_ci_status(
                    &state,
                    &task,
                    github_creds,
                    &repo_config,
                )
                .await?;
            trigger_count += ci_triggers;
        }

        // Check review comments if auto-fix is enabled
        if repo_config.automation.auto_fix_review_comments {
            let review_triggers = self
                .check_review_comments(
                    &state,
                    &task,
                    github_creds,
                    &repository,
                    &repo_config,
                )
                .await?;
            trigger_count += review_triggers;
        }

        Ok(trigger_count)
    }

    /// Checks CI status and triggers auto-fix if failures are detected
    async fn check_ci_status(
        &self,
        state: &PRMonitoringState,
        task: &UnitTask,
        creds: &crate::entities::GitHubCredentials,
        repo_config: &RepositoryConfig,
    ) -> Result<usize, String> {
        // Check if we've exceeded the max auto-fix attempts
        if state.auto_fix_count >= repo_config.automation.max_auto_fix_attempts {
            tracing::debug!(
                task_id = %state.task_id,
                "Max auto-fix attempts reached for CI failures"
            );
            return Ok(0);
        }

        let ci_status = self
            .vcs_service
            .get_github_pr_checks(creds, &state.owner, &state.repo, state.pr_number)
            .await
            .map_err(|e| format!("Failed to get CI status: {}", e))?;

        // Only trigger if status changed from non-failure to failure
        let should_trigger = ci_status.overall_status == CICheckStatus::Failure
            && state.last_ci_status != Some(CICheckStatus::Failure);

        if should_trigger {
            // Update the monitoring state
            {
                let mut monitoring = self.monitoring_state.write().await;
                if let Some(s) = monitoring.get_mut(&state.task_id) {
                    s.last_ci_status = Some(ci_status.overall_status);
                    s.auto_fix_count += 1;
                }
            }

            let failed_checks: Vec<String> = ci_status
                .check_runs
                .iter()
                .filter(|r| r.status == CICheckStatus::Failure)
                .map(|r| r.name.clone())
                .collect();

            // Send notification
            self.notification_service.notify_ci_failure(
                &task.id,
                &task.title,
            );

            // Send auto-fix trigger
            if let Some(tx) = &self.auto_fix_tx {
                let trigger = AutoFixTrigger::CIFailure {
                    task_id: task.id.clone(),
                    pr_number: state.pr_number,
                    failed_checks,
                };
                if let Err(e) = tx.send(trigger) {
                    tracing::error!(error = %e, "Failed to send CI failure auto-fix trigger");
                }
            }

            tracing::info!(
                task_id = %state.task_id,
                pr_number = %state.pr_number,
                "CI failure detected, triggering auto-fix"
            );

            return Ok(1);
        }

        // Update last CI status
        {
            let mut monitoring = self.monitoring_state.write().await;
            if let Some(s) = monitoring.get_mut(&state.task_id) {
                s.last_ci_status = Some(ci_status.overall_status);
            }
        }

        Ok(0)
    }

    /// Checks review comments and triggers auto-fix for new comments
    async fn check_review_comments(
        &self,
        state: &PRMonitoringState,
        task: &UnitTask,
        creds: &crate::entities::GitHubCredentials,
        _repository: &Repository,
        repo_config: &RepositoryConfig,
    ) -> Result<usize, String> {
        // Check if we've exceeded the max auto-fix attempts
        if state.auto_fix_count >= repo_config.automation.max_auto_fix_attempts {
            tracing::debug!(
                task_id = %state.task_id,
                "Max auto-fix attempts reached for review comments"
            );
            return Ok(0);
        }

        let reviews = self
            .vcs_service
            .get_github_pr_reviews(creds, &state.owner, &state.repo, state.pr_number)
            .await
            .map_err(|e| format!("Failed to get reviews: {}", e))?;

        let mut trigger_count = 0;
        let mut new_processed_ids: HashSet<u64> = state.processed_review_ids.clone();

        for review in reviews {
            // Skip already processed reviews
            if state.processed_review_ids.contains(&review.id) {
                continue;
            }

            // Check author permissions based on filter setting
            let should_process = match repo_config.automation.auto_fix_review_comments_filter {
                AutoFixReviewFilter::WriteAccessOnly => {
                    // Check if author has write access
                    self.vcs_service
                        .has_github_write_access(creds, &state.owner, &state.repo, &review.author)
                        .await
                        .unwrap_or(false)
                }
                AutoFixReviewFilter::All => true,
            };

            if !should_process {
                new_processed_ids.insert(review.id);
                continue;
            }

            // Process review body if present (for top-level review comments)
            if let Some(body) = &review.body {
                if !body.trim().is_empty() && (state.auto_fix_count + trigger_count as u32) < repo_config.automation.max_auto_fix_attempts {
                    if let Some(tx) = &self.auto_fix_tx {
                        let trigger = AutoFixTrigger::ReviewComment {
                            task_id: task.id.clone(),
                            pr_number: state.pr_number,
                            review_id: review.id,
                            author: review.author.clone(),
                            body: body.clone(),
                            file_path: None,
                            line: None,
                        };
                        if let Err(e) = tx.send(trigger) {
                            tracing::error!(error = %e, "Failed to send review comment auto-fix trigger");
                        } else {
                            trigger_count += 1;
                        }
                    }
                }
            }

            // Process inline comments
            for comment in &review.comments {
                if (state.auto_fix_count + trigger_count as u32) >= repo_config.automation.max_auto_fix_attempts {
                    break;
                }

                if let Some(tx) = &self.auto_fix_tx {
                    let trigger = AutoFixTrigger::ReviewComment {
                        task_id: task.id.clone(),
                        pr_number: state.pr_number,
                        review_id: comment.id,
                        author: comment.author.clone(),
                        body: comment.body.clone(),
                        file_path: comment.path.clone(),
                        line: comment.line,
                    };
                    if let Err(e) = tx.send(trigger) {
                        tracing::error!(error = %e, "Failed to send review comment auto-fix trigger");
                    } else {
                        trigger_count += 1;
                    }
                }
            }

            new_processed_ids.insert(review.id);
        }

        // Update the monitoring state with new processed IDs
        if trigger_count > 0 {
            {
                let mut monitoring = self.monitoring_state.write().await;
                if let Some(s) = monitoring.get_mut(&state.task_id) {
                    s.processed_review_ids = new_processed_ids;
                    s.auto_fix_count += trigger_count as u32;
                }
            }

            // Send notification
            self.notification_service.notify_review_comments(
                &task.id,
                &task.title,
                trigger_count,
            );

            tracing::info!(
                task_id = %state.task_id,
                pr_number = %state.pr_number,
                count = %trigger_count,
                "Review comments detected, triggering auto-fix"
            );
        }

        Ok(trigger_count)
    }

    /// Starts the background polling loop
    /// Returns a handle to stop the loop
    pub fn start_polling_loop(
        self: Arc<Self>,
        poll_interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);

            loop {
                interval.tick().await;

                match self.poll_once().await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::debug!(triggers = %count, "PR poll cycle completed");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "PR poll cycle failed");
                    }
                }
            }
        })
    }

    /// Gets the list of currently monitored task IDs
    pub async fn get_monitored_tasks(&self) -> Vec<String> {
        let monitoring = self.monitoring_state.read().await;
        monitoring.keys().cloned().collect()
    }
}

/// Parses a GitHub PR URL to extract owner, repo, and PR number
/// Supports formats like:
/// - https://github.com/owner/repo/pull/123
/// - https://github.com/owner/repo/pull/123/files
fn parse_github_pr_url(url: &str) -> Option<(String, String, u64)> {
    // Use a simple approach without regex for clarity
    let url = url.trim_end_matches('/');

    // Find the "/pull/" part
    let pull_idx = url.find("/pull/")?;
    let before_pull = &url[..pull_idx];
    let after_pull = &url[pull_idx + 6..]; // Skip "/pull/"

    // Extract PR number (first numeric segment after /pull/)
    let pr_number_str = after_pull.split('/').next()?;
    let pr_number: u64 = pr_number_str.parse().ok()?;

    // Extract owner and repo from before /pull/
    // Remove "https://github.com/" prefix
    let path = before_pull
        .strip_prefix("https://github.com/")
        .or_else(|| before_pull.strip_prefix("http://github.com/"))?;

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        Some((owner, repo, pr_number))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_github_pr_url {
        use super::*;

        #[test]
        fn test_parses_standard_pr_url() {
            let result = parse_github_pr_url("https://github.com/owner/repo/pull/123");
            assert_eq!(result, Some(("owner".to_string(), "repo".to_string(), 123)));
        }

        #[test]
        fn test_parses_pr_url_with_files_suffix() {
            let result = parse_github_pr_url("https://github.com/owner/repo/pull/456/files");
            assert_eq!(result, Some(("owner".to_string(), "repo".to_string(), 456)));
        }

        #[test]
        fn test_parses_pr_url_with_trailing_slash() {
            let result = parse_github_pr_url("https://github.com/owner/repo/pull/789/");
            assert_eq!(result, Some(("owner".to_string(), "repo".to_string(), 789)));
        }

        #[test]
        fn test_returns_none_for_invalid_url() {
            assert_eq!(parse_github_pr_url("not a url"), None);
            assert_eq!(parse_github_pr_url("https://github.com/owner/repo"), None);
            assert_eq!(parse_github_pr_url("https://github.com/owner/repo/issues/123"), None);
        }

        #[test]
        fn test_returns_none_for_non_numeric_pr_number() {
            assert_eq!(parse_github_pr_url("https://github.com/owner/repo/pull/abc"), None);
        }
    }
}
