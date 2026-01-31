//! Worker registration and management

#![allow(dead_code)]

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use rpc_protocol::{WorkerCapacity, WorkerLoad};

/// Information about a registered worker
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Worker ID
    pub id: String,

    /// Worker address
    pub address: String,

    /// Last heartbeat timestamp
    pub last_heartbeat: Instant,

    /// Worker capacity
    pub capacity: WorkerCapacity,

    /// Current load
    pub current_load: WorkerLoad,

    /// Currently assigned task IDs
    pub current_tasks: Vec<String>,
}

impl WorkerInfo {
    /// Check if the worker has capacity for more tasks
    pub fn has_capacity(&self) -> bool {
        self.current_load.running_tasks < self.capacity.max_concurrent_tasks
    }

    /// Get the available task slots
    pub fn available_slots(&self) -> u32 {
        self.capacity
            .max_concurrent_tasks
            .saturating_sub(self.current_load.running_tasks)
    }
}

/// Registry for managing workers
#[derive(Debug)]
pub struct WorkerRegistry {
    /// Registered workers
    workers: HashMap<String, WorkerInfo>,

    /// Task to worker assignments
    task_assignments: HashMap<String, String>,

    /// Heartbeat timeout duration
    heartbeat_timeout: Duration,
}

impl WorkerRegistry {
    /// Create a new worker registry
    pub fn new(heartbeat_timeout_secs: u64) -> Self {
        Self {
            workers: HashMap::new(),
            task_assignments: HashMap::new(),
            heartbeat_timeout: Duration::from_secs(heartbeat_timeout_secs),
        }
    }

    /// Register a new worker
    pub fn register(
        &mut self,
        worker_id: String,
        address: String,
        capacity: WorkerCapacity,
    ) -> &WorkerInfo {
        let worker = WorkerInfo {
            id: worker_id.clone(),
            address,
            last_heartbeat: Instant::now(),
            capacity,
            current_load: WorkerLoad {
                running_tasks: 0,
                cpu_usage: 0,
                memory_usage: 0,
            },
            current_tasks: Vec::new(),
        };

        self.workers.insert(worker_id.clone(), worker);
        self.workers.get(&worker_id).unwrap()
    }

    /// Update worker heartbeat
    pub fn heartbeat(&mut self, worker_id: &str, load: WorkerLoad) -> Result<(), RegistryError> {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.last_heartbeat = Instant::now();
            worker.current_load = load;
            Ok(())
        } else {
            Err(RegistryError::WorkerNotFound)
        }
    }

    /// Unregister a worker
    pub fn unregister(&mut self, worker_id: &str) {
        // Remove worker
        self.workers.remove(worker_id);

        // Remove task assignments for this worker
        self.task_assignments.retain(|_, wid| wid != worker_id);
    }

    /// Get a worker by ID
    pub fn get_worker(&self, worker_id: &str) -> Option<&WorkerInfo> {
        self.workers.get(worker_id)
    }

    /// List all workers
    pub fn list_workers(&self) -> Vec<&WorkerInfo> {
        self.workers.values().collect()
    }

    /// List only healthy workers (those with recent heartbeats)
    pub fn list_healthy_workers(&self) -> Vec<&WorkerInfo> {
        let now = Instant::now();
        self.workers
            .values()
            .filter(|w| now.duration_since(w.last_heartbeat) < self.heartbeat_timeout)
            .collect()
    }

    /// Select the best worker for a task
    pub fn select_worker_for_task(&self) -> Option<&WorkerInfo> {
        let now = Instant::now();

        self.workers
            .values()
            .filter(|w| {
                w.has_capacity() && now.duration_since(w.last_heartbeat) < self.heartbeat_timeout
            })
            .min_by_key(|w| w.current_load.running_tasks)
    }

    /// Assign a task to a worker
    pub fn assign_task(&mut self, task_id: &str, worker_id: &str) -> Result<(), RegistryError> {
        if !self.workers.contains_key(worker_id) {
            return Err(RegistryError::WorkerNotFound);
        }

        // Record assignment
        self.task_assignments
            .insert(task_id.to_string(), worker_id.to_string());

        // Update worker's task list
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.current_tasks.push(task_id.to_string());
            worker.current_load.running_tasks = worker.current_tasks.len() as u32;
        }

        Ok(())
    }

    /// Remove a task assignment
    pub fn remove_task_assignment(&mut self, task_id: &str) {
        if let Some(worker_id) = self.task_assignments.remove(task_id) {
            if let Some(worker) = self.workers.get_mut(&worker_id) {
                worker.current_tasks.retain(|id| id != task_id);
                worker.current_load.running_tasks = worker.current_tasks.len() as u32;
            }
        }
    }

    /// Get the worker assigned to a task
    pub fn get_task_worker(&self, task_id: &str) -> Option<&WorkerInfo> {
        self.task_assignments
            .get(task_id)
            .and_then(|worker_id| self.workers.get(worker_id))
    }

    /// Clean up stale workers (those that have timed out)
    pub fn cleanup_stale_workers(&mut self) -> Vec<String> {
        let now = Instant::now();
        let stale_workers: Vec<String> = self
            .workers
            .iter()
            .filter(|(_, w)| now.duration_since(w.last_heartbeat) >= self.heartbeat_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for worker_id in &stale_workers {
            self.unregister(worker_id);
        }

        stale_workers
    }

    /// Get the number of registered workers
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Get the number of healthy workers
    pub fn healthy_worker_count(&self) -> usize {
        self.list_healthy_workers().len()
    }
}

/// Registry errors
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Worker not found")]
    WorkerNotFound,

    #[error("Worker has no capacity")]
    NoCapacity,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capacity() -> WorkerCapacity {
        WorkerCapacity {
            max_concurrent_tasks: 4,
            available_memory: 8 * 1024 * 1024 * 1024,
            available_cpus: 8,
        }
    }

    #[test]
    fn test_register_worker() {
        let mut registry = WorkerRegistry::new(60);

        let worker = registry.register(
            "worker-1".to_string(),
            "127.0.0.1:9000".to_string(),
            test_capacity(),
        );

        assert_eq!(worker.id, "worker-1");
        assert_eq!(registry.worker_count(), 1);
    }

    #[test]
    fn test_select_worker_for_task() {
        let mut registry = WorkerRegistry::new(60);

        registry.register(
            "worker-1".to_string(),
            "127.0.0.1:9000".to_string(),
            test_capacity(),
        );
        registry.register(
            "worker-2".to_string(),
            "127.0.0.1:9001".to_string(),
            test_capacity(),
        );

        // Assign some tasks to worker-1
        registry.assign_task("task-1", "worker-1").unwrap();
        registry.assign_task("task-2", "worker-1").unwrap();

        // Worker-2 should be selected (fewer tasks)
        let selected = registry.select_worker_for_task().unwrap();
        assert_eq!(selected.id, "worker-2");
    }

    #[test]
    fn test_heartbeat() {
        let mut registry = WorkerRegistry::new(60);

        registry.register(
            "worker-1".to_string(),
            "127.0.0.1:9000".to_string(),
            test_capacity(),
        );

        let new_load = WorkerLoad {
            running_tasks: 2,
            cpu_usage: 50,
            memory_usage: 40,
        };

        registry.heartbeat("worker-1", new_load.clone()).unwrap();

        let worker = registry.get_worker("worker-1").unwrap();
        assert_eq!(worker.current_load.running_tasks, 2);
        assert_eq!(worker.current_load.cpu_usage, 50);
    }

    #[test]
    fn test_task_assignment() {
        let mut registry = WorkerRegistry::new(60);

        registry.register(
            "worker-1".to_string(),
            "127.0.0.1:9000".to_string(),
            test_capacity(),
        );

        registry.assign_task("task-1", "worker-1").unwrap();

        let worker = registry.get_task_worker("task-1").unwrap();
        assert_eq!(worker.id, "worker-1");
        assert!(worker.current_tasks.contains(&"task-1".to_string()));

        registry.remove_task_assignment("task-1");

        let worker = registry.get_worker("worker-1").unwrap();
        assert!(!worker.current_tasks.contains(&"task-1".to_string()));
    }
}
