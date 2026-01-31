//! Worker registry for managing worker servers.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// Worker status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    /// Worker is available for tasks.
    Idle,
    /// Worker is executing a task.
    Busy,
    /// Worker has missed heartbeats.
    Unhealthy,
}

/// Registered worker information.
#[derive(Debug, Clone)]
pub struct RegisteredWorker {
    /// Worker ID.
    pub id: Uuid,
    /// Worker name.
    pub name: String,
    /// Worker endpoint URL.
    pub endpoint_url: String,
    /// Current status.
    pub status: WorkerStatus,
    /// Last heartbeat timestamp.
    pub last_heartbeat: DateTime<Utc>,
    /// Currently assigned task ID.
    pub current_task_id: Option<Uuid>,
    /// Registration timestamp.
    pub registered_at: DateTime<Utc>,
}

impl RegisteredWorker {
    /// Creates a new registered worker.
    pub fn new(name: impl Into<String>, endpoint_url: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            endpoint_url: endpoint_url.into(),
            status: WorkerStatus::Idle,
            last_heartbeat: now,
            current_task_id: None,
            registered_at: now,
        }
    }

    /// Returns true if the worker is healthy (heartbeat within threshold).
    pub fn is_healthy(&self, heartbeat_timeout: Duration) -> bool {
        Utc::now() - self.last_heartbeat < heartbeat_timeout
    }

    /// Returns true if the worker is available for a task.
    pub fn is_available(&self, heartbeat_timeout: Duration) -> bool {
        self.status == WorkerStatus::Idle && self.is_healthy(heartbeat_timeout)
    }
}

/// Worker registry for tracking active workers.
#[derive(Debug)]
pub struct WorkerRegistry {
    /// Registered workers by ID.
    workers: HashMap<Uuid, RegisteredWorker>,
    /// Heartbeat timeout duration (default: 90 seconds).
    heartbeat_timeout: Duration,
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerRegistry {
    /// Creates a new worker registry.
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            heartbeat_timeout: Duration::seconds(90),
        }
    }

    /// Sets the heartbeat timeout.
    pub fn with_heartbeat_timeout(mut self, timeout: Duration) -> Self {
        self.heartbeat_timeout = timeout;
        self
    }

    /// Registers a new worker.
    pub fn register(&mut self, name: impl Into<String>, endpoint_url: impl Into<String>) -> Uuid {
        let worker = RegisteredWorker::new(name, endpoint_url);
        let id = worker.id;
        self.workers.insert(id, worker);
        tracing::info!(worker_id = %id, "Worker registered");
        id
    }

    /// Unregisters a worker.
    pub fn unregister(&mut self, worker_id: Uuid) -> Option<RegisteredWorker> {
        let worker = self.workers.remove(&worker_id);
        if worker.is_some() {
            tracing::info!(worker_id = %worker_id, "Worker unregistered");
        }
        worker
    }

    /// Updates a worker's heartbeat.
    pub fn heartbeat(
        &mut self,
        worker_id: Uuid,
        status: WorkerStatus,
        current_task_id: Option<Uuid>,
    ) -> bool {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.last_heartbeat = Utc::now();
            worker.status = status;
            worker.current_task_id = current_task_id;
            true
        } else {
            false
        }
    }

    /// Gets a worker by ID.
    pub fn get(&self, worker_id: Uuid) -> Option<&RegisteredWorker> {
        self.workers.get(&worker_id)
    }

    /// Gets a mutable reference to a worker by ID.
    pub fn get_mut(&mut self, worker_id: Uuid) -> Option<&mut RegisteredWorker> {
        self.workers.get_mut(&worker_id)
    }

    /// Finds an available worker for a task.
    pub fn find_available(&self) -> Option<&RegisteredWorker> {
        self.workers
            .values()
            .find(|w| w.is_available(self.heartbeat_timeout))
    }

    /// Assigns a task to a worker.
    pub fn assign_task(&mut self, worker_id: Uuid, task_id: Uuid) -> bool {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            if worker.is_available(self.heartbeat_timeout) {
                worker.status = WorkerStatus::Busy;
                worker.current_task_id = Some(task_id);
                tracing::info!(worker_id = %worker_id, task_id = %task_id, "Task assigned to worker");
                return true;
            }
        }
        false
    }

    /// Marks a worker as idle after completing a task.
    pub fn complete_task(&mut self, worker_id: Uuid) -> bool {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.status = WorkerStatus::Idle;
            worker.current_task_id = None;
            return true;
        }
        false
    }

    /// Returns all workers.
    pub fn all_workers(&self) -> impl Iterator<Item = &RegisteredWorker> {
        self.workers.values()
    }

    /// Checks all workers and marks unhealthy ones.
    pub fn check_health(&mut self) {
        for worker in self.workers.values_mut() {
            if !worker.is_healthy(self.heartbeat_timeout) && worker.status != WorkerStatus::Unhealthy
            {
                tracing::warn!(
                    worker_id = %worker.id,
                    last_heartbeat = %worker.last_heartbeat,
                    "Worker marked as unhealthy"
                );
                worker.status = WorkerStatus::Unhealthy;
            }
        }
    }

    /// Returns the number of registered workers.
    pub fn count(&self) -> usize {
        self.workers.len()
    }

    /// Returns the number of available workers.
    pub fn available_count(&self) -> usize {
        self.workers
            .values()
            .filter(|w| w.is_available(self.heartbeat_timeout))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_registration() {
        let mut registry = WorkerRegistry::new();
        let id = registry.register("worker-1", "http://localhost:54872");

        assert_eq!(registry.count(), 1);
        let worker = registry.get(id).unwrap();
        assert_eq!(worker.name, "worker-1");
        assert_eq!(worker.status, WorkerStatus::Idle);
    }

    #[test]
    fn test_worker_unregistration() {
        let mut registry = WorkerRegistry::new();
        let id = registry.register("worker-1", "http://localhost:54872");

        assert_eq!(registry.count(), 1);
        registry.unregister(id);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_task_assignment() {
        let mut registry = WorkerRegistry::new();
        let worker_id = registry.register("worker-1", "http://localhost:54872");
        let task_id = Uuid::new_v4();

        assert!(registry.assign_task(worker_id, task_id));

        let worker = registry.get(worker_id).unwrap();
        assert_eq!(worker.status, WorkerStatus::Busy);
        assert_eq!(worker.current_task_id, Some(task_id));
    }

    #[test]
    fn test_find_available() {
        let mut registry = WorkerRegistry::new();
        let id1 = registry.register("worker-1", "http://localhost:54872");
        let _id2 = registry.register("worker-2", "http://localhost:54873");

        // Both should be available initially
        assert!(registry.find_available().is_some());

        // Assign task to first worker
        registry.assign_task(id1, Uuid::new_v4());

        // Second worker should still be available
        let available = registry.find_available().unwrap();
        assert_ne!(available.id, id1);
    }
}
