use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use tokio::sync::{mpsc, RwLock};

use super::LicenseService;
use crate::entities::ConcurrencyConfig;

/// Error types for concurrency operations
#[derive(Debug, Clone)]
pub enum ConcurrencyError {
    /// Maximum concurrent sessions limit reached.
    /// The task has been queued for automatic execution when a slot becomes
    /// available.
    LimitReached { current: u32, limit: u32 },
    /// Feature requires a valid license
    LicenseRequired,
}

/// A pending task that couldn't be started due to concurrency limits.
/// Contains only the task ID - the actual task execution is handled by the
/// receiver.
#[derive(Debug, Clone)]
pub struct PendingTask {
    /// The unit task ID
    pub task_id: String,
}

/// RAII guard that automatically unregisters a task when dropped.
/// This ensures task cleanup even if the execution panics.
pub struct TaskGuard {
    task_id: String,
    concurrency_service: Arc<ConcurrencyService>,
    /// Whether the guard has been disarmed (task manually unregistered)
    disarmed: bool,
}

impl TaskGuard {
    /// Creates a new TaskGuard for the given task
    fn new(task_id: String, concurrency_service: Arc<ConcurrencyService>) -> Self {
        Self {
            task_id,
            concurrency_service,
            disarmed: false,
        }
    }

    /// Disarms the guard, preventing automatic unregistration on drop.
    /// Use this if you want to manually control when the task is unregistered.
    #[allow(dead_code)]
    pub fn disarm(&mut self) {
        self.disarmed = true;
    }

    /// Gets the task ID associated with this guard
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            let task_id = self.task_id.clone();
            let service = self.concurrency_service.clone();
            // Spawn a blocking task to handle the async unregister
            // This is safe because drop is called on the same runtime
            tokio::spawn(async move {
                service.unregister_task(&task_id).await;
            });
        }
    }
}

impl std::fmt::Display for ConcurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcurrencyError::LimitReached { current, limit } => {
                write!(
                    f,
                    "Maximum concurrent sessions limit reached ({}/{}). Please wait for a task to \
                     complete or increase the limit.",
                    current, limit
                )
            }
            ConcurrencyError::LicenseRequired => {
                write!(
                    f,
                    "Concurrency limits are a premium feature. Please activate a license to use \
                     this feature."
                )
            }
        }
    }
}

impl std::error::Error for ConcurrencyError {}

/// Service for managing concurrent agent session limits.
/// This is a premium feature that requires a valid license.
///
/// When a task cannot be started due to concurrency limits, it is added to
/// a pending queue. When a running task completes, the service automatically
/// notifies the registered callback channel so that pending tasks can be
/// executed.
pub struct ConcurrencyService {
    /// Set of currently running task IDs
    running_tasks: Arc<RwLock<HashSet<String>>>,
    /// Queue of pending task IDs (FIFO order)
    pending_tasks: Arc<RwLock<VecDeque<PendingTask>>>,
    /// License service for checking premium feature access
    license_service: Arc<LicenseService>,
    /// Channel sender to notify when a slot becomes available
    /// The receiver should attempt to execute the pending task
    slot_available_tx: Arc<RwLock<Option<mpsc::UnboundedSender<PendingTask>>>>,
}

impl ConcurrencyService {
    /// Creates a new ConcurrencyService
    pub fn new(license_service: Arc<LicenseService>) -> Self {
        Self {
            running_tasks: Arc::new(RwLock::new(HashSet::new())),
            pending_tasks: Arc::new(RwLock::new(VecDeque::new())),
            license_service,
            slot_available_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Sets the channel for notifying when a slot becomes available.
    /// The receiver will receive pending tasks that should be executed.
    pub async fn set_slot_available_channel(&self, tx: mpsc::UnboundedSender<PendingTask>) {
        *self.slot_available_tx.write().await = Some(tx);
    }

    /// Adds a task to the pending queue.
    /// The task will be automatically executed when a slot becomes available.
    pub async fn add_pending_task(&self, task_id: &str) {
        let mut pending = self.pending_tasks.write().await;
        // Avoid duplicate entries
        if !pending.iter().any(|t| t.task_id == task_id) {
            pending.push_back(PendingTask {
                task_id: task_id.to_string(),
            });
            tracing::info!(
                "Added task {} to pending queue, queue size: {}",
                task_id,
                pending.len()
            );
        }
    }

    /// Removes a task from the pending queue (e.g., if it was cancelled).
    pub async fn remove_pending_task(&self, task_id: &str) {
        let mut pending = self.pending_tasks.write().await;
        pending.retain(|t| t.task_id != task_id);
        tracing::debug!("Removed task {} from pending queue", task_id);
    }

    /// Gets the number of pending tasks.
    pub async fn pending_count(&self) -> u32 {
        self.pending_tasks.read().await.len() as u32
    }

    /// Gets the list of pending task IDs.
    pub async fn pending_task_ids(&self) -> Vec<String> {
        self.pending_tasks
            .read()
            .await
            .iter()
            .map(|t| t.task_id.clone())
            .collect()
    }

    /// Atomically checks if a new task can be started and registers it if
    /// allowed. Returns a TaskGuard on success that will automatically
    /// unregister the task when dropped.
    ///
    /// This function validates:
    /// 1. If max_concurrent_sessions is None (unlimited), always allows
    /// 2. If max_concurrent_sessions is set, requires a valid license
    /// 3. If license is valid, checks current count against limit
    ///
    /// This method is atomic - it holds the write lock during both the check
    /// and insert, preventing race conditions where multiple tasks could
    /// bypass the limit.
    pub async fn try_start_task(
        self: &Arc<Self>,
        config: &ConcurrencyConfig,
        task_id: &str,
    ) -> Result<TaskGuard, ConcurrencyError> {
        // If no limit is set, always allow - but still register for tracking
        let limit = match config.max_concurrent_sessions {
            Some(limit) => limit,
            None => {
                // No limit, register and return guard
                let mut running = self.running_tasks.write().await;
                running.insert(task_id.to_string());
                tracing::debug!(
                    "Registered running task {} (unlimited mode), total running: {}",
                    task_id,
                    running.len()
                );
                return Ok(TaskGuard::new(task_id.to_string(), Arc::clone(self)));
            }
        };

        // Concurrency limits require a valid license
        if !self.license_service.is_license_valid().await {
            return Err(ConcurrencyError::LicenseRequired);
        }

        // Atomically check and register while holding the write lock
        let mut running = self.running_tasks.write().await;
        let current_count = running.len() as u32;

        if current_count >= limit {
            return Err(ConcurrencyError::LimitReached {
                current: current_count,
                limit,
            });
        }

        // Register the task while still holding the lock
        running.insert(task_id.to_string());
        tracing::debug!(
            "Registered running task {}, total running: {}/{}",
            task_id,
            running.len(),
            limit
        );

        Ok(TaskGuard::new(task_id.to_string(), Arc::clone(self)))
    }

    /// Checks if a new task can be started given the concurrency limit.
    /// Returns Ok(()) if the task can proceed, or an error if the limit is
    /// reached.
    ///
    /// DEPRECATED: Use try_start_task instead to avoid race conditions.
    /// This method is kept for backward compatibility but should not be used
    /// in new code.
    #[deprecated(note = "Use try_start_task instead to avoid race conditions")]
    pub async fn can_start_task(&self, config: &ConcurrencyConfig) -> Result<(), ConcurrencyError> {
        // If no limit is set, always allow
        let limit = match config.max_concurrent_sessions {
            Some(limit) => limit,
            None => return Ok(()),
        };

        // Concurrency limits require a valid license
        if !self.license_service.is_license_valid().await {
            return Err(ConcurrencyError::LicenseRequired);
        }

        // Check current count against limit
        let running = self.running_tasks.read().await;
        let current_count = running.len() as u32;

        if current_count >= limit {
            return Err(ConcurrencyError::LimitReached {
                current: current_count,
                limit,
            });
        }

        Ok(())
    }

    /// Registers a task as running.
    /// Should be called when task execution starts.
    ///
    /// DEPRECATED: Use try_start_task instead to avoid race conditions.
    #[deprecated(note = "Use try_start_task instead to avoid race conditions")]
    pub async fn register_task(&self, task_id: &str) {
        let mut running = self.running_tasks.write().await;
        running.insert(task_id.to_string());
        tracing::debug!(
            "Registered running task {}, total running: {}",
            task_id,
            running.len()
        );
    }

    /// Unregisters a task as running.
    /// Should be called when task execution completes (success or failure).
    ///
    /// If there are pending tasks and a slot is now available, notifies the
    /// registered callback channel to trigger execution of the next pending
    /// task.
    pub async fn unregister_task(&self, task_id: &str) {
        let mut running = self.running_tasks.write().await;
        running.remove(task_id);
        let running_count = running.len();
        tracing::debug!(
            "Unregistered task {}, total running: {}",
            task_id,
            running_count
        );
        // Release the lock before processing pending tasks
        drop(running);

        // Check if there are pending tasks to execute
        self.try_execute_pending_task().await;
    }

    /// Attempts to pop and notify for a pending task if there's capacity.
    /// This is called after a task is unregistered to potentially start a
    /// pending task.
    async fn try_execute_pending_task(&self) {
        // Pop a pending task if available
        let pending_task = {
            let mut pending = self.pending_tasks.write().await;
            pending.pop_front()
        };

        if let Some(pending_task) = pending_task {
            // Notify via channel
            let tx_guard = self.slot_available_tx.read().await;
            if let Some(tx) = tx_guard.as_ref() {
                if tx.send(pending_task.clone()).is_ok() {
                    tracing::info!("Notified to execute pending task: {}", pending_task.task_id);
                } else {
                    // Channel closed, put the task back
                    tracing::warn!(
                        "Failed to notify pending task execution, channel closed. Re-queuing \
                         task: {}",
                        pending_task.task_id
                    );
                    let mut pending = self.pending_tasks.write().await;
                    pending.push_front(pending_task);
                }
            } else {
                // No callback registered, put the task back
                tracing::warn!(
                    "No slot available callback registered. Re-queuing task: {}",
                    pending_task.task_id
                );
                let mut pending = self.pending_tasks.write().await;
                pending.push_front(pending_task);
            }
        }
    }

    /// Gets the current number of running tasks.
    pub async fn running_count(&self) -> u32 {
        self.running_tasks.read().await.len() as u32
    }

    /// Gets the set of currently running task IDs.
    pub async fn running_tasks(&self) -> HashSet<String> {
        self.running_tasks.read().await.clone()
    }

    /// Clears all running tasks and pending queue.
    /// This should only be used for cleanup purposes (e.g., on app restart).
    pub async fn clear(&self) {
        let mut running = self.running_tasks.write().await;
        running.clear();
        let mut pending = self.pending_tasks.write().await;
        pending.clear();
        tracing::debug!("Cleared all running and pending tasks");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;

    fn create_test_license_service() -> Arc<LicenseService> {
        // Create a mock config manager for testing
        // In reality, we'd need to properly mock this
        let config_manager =
            Arc::new(ConfigManager::new().expect("Failed to create config manager for test"));
        Arc::new(LicenseService::new(config_manager))
    }

    fn create_test_service() -> Arc<ConcurrencyService> {
        let license_service = create_test_license_service();
        Arc::new(ConcurrencyService::new(license_service))
    }

    #[tokio::test]
    async fn test_try_start_task_unlimited() {
        let service = create_test_service();

        let config = ConcurrencyConfig {
            max_concurrent_sessions: None,
        };

        // Should always succeed with no limit
        let guard1 = service.try_start_task(&config, "task1").await;
        assert!(guard1.is_ok());
        assert_eq!(service.running_count().await, 1);

        let guard2 = service.try_start_task(&config, "task2").await;
        assert!(guard2.is_ok());
        assert_eq!(service.running_count().await, 2);

        let guard3 = service.try_start_task(&config, "task3").await;
        assert!(guard3.is_ok());
        assert_eq!(service.running_count().await, 3);

        // Keep guards alive to prevent auto-unregister
        drop(guard1);
        drop(guard2);
        drop(guard3);
    }

    #[tokio::test]
    async fn test_task_guard_auto_unregister() {
        let service = create_test_service();

        let config = ConcurrencyConfig {
            max_concurrent_sessions: None,
        };

        {
            let guard = service.try_start_task(&config, "task1").await.unwrap();
            assert_eq!(service.running_count().await, 1);
            assert_eq!(guard.task_id(), "task1");
            // Guard will be dropped here
        }

        // Give tokio a chance to process the spawned unregister task
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Task should be automatically unregistered
        assert_eq!(service.running_count().await, 0);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_unlimited_concurrency() {
        let license_service = create_test_license_service();
        let service = ConcurrencyService::new(license_service);

        let config = ConcurrencyConfig {
            max_concurrent_sessions: None,
        };

        // Should always succeed with no limit
        assert!(service.can_start_task(&config).await.is_ok());

        // Register some tasks
        service.register_task("task1").await;
        service.register_task("task2").await;
        service.register_task("task3").await;

        // Still should succeed
        assert!(service.can_start_task(&config).await.is_ok());
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_register_and_unregister() {
        let license_service = create_test_license_service();
        let service = ConcurrencyService::new(license_service);

        assert_eq!(service.running_count().await, 0);

        service.register_task("task1").await;
        assert_eq!(service.running_count().await, 1);

        service.register_task("task2").await;
        assert_eq!(service.running_count().await, 2);

        service.unregister_task("task1").await;
        assert_eq!(service.running_count().await, 1);

        service.unregister_task("task2").await;
        assert_eq!(service.running_count().await, 0);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_clear() {
        let license_service = create_test_license_service();
        let service = ConcurrencyService::new(license_service);

        service.register_task("task1").await;
        service.register_task("task2").await;
        assert_eq!(service.running_count().await, 2);

        service.clear().await;
        assert_eq!(service.running_count().await, 0);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_running_tasks() {
        let license_service = create_test_license_service();
        let service = ConcurrencyService::new(license_service);

        service.register_task("task1").await;
        service.register_task("task2").await;

        let running = service.running_tasks().await;
        assert!(running.contains("task1"));
        assert!(running.contains("task2"));
        assert_eq!(running.len(), 2);
    }

    #[tokio::test]
    async fn test_manual_unregister() {
        let service = create_test_service();

        let config = ConcurrencyConfig {
            max_concurrent_sessions: None,
        };

        let guard = service.try_start_task(&config, "task1").await.unwrap();
        assert_eq!(service.running_count().await, 1);

        // Manually unregister
        service.unregister_task(guard.task_id()).await;
        assert_eq!(service.running_count().await, 0);

        // Guard drop will try to unregister again, but that's ok (no-op)
        drop(guard);
    }
}
