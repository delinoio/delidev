//! Embedded Worker for Single Process Mode
//!
//! This module provides an embedded worker that executes tasks locally
//! without requiring a separate worker process or network communication.
//!
//! In single-process mode, task execution is delegated to the desktop app's
//! AgentExecutionService, which handles Docker sandbox creation and agent
//! invocation.

use std::sync::Arc;

use task_store::TaskStore;
use tokio::sync::RwLock;

use super::SingleProcessError;

/// Embedded worker for single-process mode
///
/// This worker executes tasks locally using the desktop app's services.
/// It provides the same functionality as the standalone worker but
/// communicates via direct function calls instead of JSON-RPC.
pub struct EmbeddedWorker {
    /// Reference to the task store
    store: Arc<dyn TaskStore>,

    /// Currently executing tasks
    active_tasks: Arc<RwLock<Vec<String>>>,
}

impl EmbeddedWorker {
    /// Creates a new embedded worker
    pub async fn new(store: Arc<dyn TaskStore>) -> Result<Self, SingleProcessError> {
        tracing::info!("Initializing embedded worker for single-process mode");

        Ok(Self {
            store,
            active_tasks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Returns the task store
    pub fn store(&self) -> &Arc<dyn TaskStore> {
        &self.store
    }

    /// Checks if a task is currently executing
    pub async fn is_task_executing(&self, task_id: &str) -> bool {
        let tasks = self.active_tasks.read().await;
        tasks.contains(&task_id.to_string())
    }

    /// Registers a task as executing
    ///
    /// This is called by the desktop app's AgentExecutionService when
    /// it starts executing a task.
    pub async fn register_executing_task(&self, task_id: &str) {
        let mut tasks = self.active_tasks.write().await;
        if !tasks.contains(&task_id.to_string()) {
            tasks.push(task_id.to_string());
            tracing::info!(task_id = %task_id, "Registered executing task");
        }
    }

    /// Unregisters a task as executing
    ///
    /// This is called by the desktop app's AgentExecutionService when
    /// a task completes or is stopped.
    pub async fn unregister_executing_task(&self, task_id: &str) {
        let mut tasks = self.active_tasks.write().await;
        tasks.retain(|id| id != task_id);
        tracing::info!(task_id = %task_id, "Unregistered executing task");
    }

    /// Gets the list of currently executing tasks
    pub async fn get_executing_tasks(&self) -> Vec<String> {
        let tasks = self.active_tasks.read().await;
        tasks.clone()
    }

    /// Gets the current load (number of executing tasks)
    pub async fn get_current_load(&self) -> u32 {
        let tasks = self.active_tasks.read().await;
        tasks.len() as u32
    }

    /// Checks if the worker has capacity for more tasks
    ///
    /// In single-process mode, the capacity is determined by the
    /// concurrency settings in the global config.
    pub async fn has_capacity(&self, max_concurrent: u32) -> bool {
        self.get_current_load().await < max_concurrent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use task_store::MemoryStore;

    #[tokio::test]
    async fn test_embedded_worker_creation() {
        let store = Arc::new(MemoryStore::new());
        let worker = EmbeddedWorker::new(store).await.unwrap();

        assert_eq!(worker.get_current_load().await, 0);
        assert!(worker.has_capacity(1).await);
    }

    #[tokio::test]
    async fn test_task_registration() {
        let store = Arc::new(MemoryStore::new());
        let worker = EmbeddedWorker::new(store).await.unwrap();

        // Register a task
        worker.register_executing_task("task-1").await;
        assert!(worker.is_task_executing("task-1").await);
        assert_eq!(worker.get_current_load().await, 1);

        // Register another task
        worker.register_executing_task("task-2").await;
        assert!(worker.is_task_executing("task-2").await);
        assert_eq!(worker.get_current_load().await, 2);

        // Unregister first task
        worker.unregister_executing_task("task-1").await;
        assert!(!worker.is_task_executing("task-1").await);
        assert_eq!(worker.get_current_load().await, 1);
    }

    #[tokio::test]
    async fn test_capacity_check() {
        let store = Arc::new(MemoryStore::new());
        let worker = EmbeddedWorker::new(store).await.unwrap();

        assert!(worker.has_capacity(2).await);

        worker.register_executing_task("task-1").await;
        assert!(worker.has_capacity(2).await);

        worker.register_executing_task("task-2").await;
        assert!(!worker.has_capacity(2).await);

        worker.unregister_executing_task("task-1").await;
        assert!(worker.has_capacity(2).await);
    }
}
