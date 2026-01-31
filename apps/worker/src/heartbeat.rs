//! Heartbeat service for worker health reporting

use std::{sync::Arc, time::Duration};

use rpc_protocol::WorkerLoad;
use sysinfo::System;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::{config::WorkerConfig, executor::TaskExecutor, server_client::MainServerClient};

/// Heartbeat service for periodic health reporting
pub struct HeartbeatService {
    /// Worker configuration
    config: Arc<WorkerConfig>,
    /// Server client
    client: Arc<MainServerClient>,
    /// Task executor for getting running task count
    executor: Arc<TaskExecutor>,
    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(
        config: Arc<WorkerConfig>,
        client: Arc<MainServerClient>,
        executor: Arc<TaskExecutor>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            config,
            client,
            executor,
            shutdown_rx,
        }
    }

    /// Run the heartbeat service
    pub async fn run(mut self) {
        let interval = Duration::from_secs(self.config.heartbeat_interval_secs);
        let worker_id = self.config.worker_id();

        info!(
            worker_id = %worker_id,
            interval_secs = %self.config.heartbeat_interval_secs,
            "Starting heartbeat service"
        );

        let mut interval_timer = tokio::time::interval(interval);
        let mut system = System::new();

        loop {
            tokio::select! {
                _ = interval_timer.tick() => {
                    // Refresh system info
                    system.refresh_cpu_all();
                    system.refresh_memory();

                    // Get current load
                    let load = self.get_current_load(&system).await;

                    debug!(
                        worker_id = %worker_id,
                        running_tasks = %load.running_tasks,
                        cpu_usage = %load.cpu_usage,
                        memory_usage = %load.memory_usage,
                        "Sending heartbeat"
                    );

                    // Send heartbeat
                    match self.client.heartbeat(worker_id, load).await {
                        Ok(response) => {
                            if !response.accepted {
                                warn!("Heartbeat was not accepted by server");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to send heartbeat");
                        }
                    }
                }

                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Heartbeat service shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Get current worker load
    async fn get_current_load(&self, system: &System) -> WorkerLoad {
        let running_tasks = self.executor.running_task_count().await as u32;

        // Calculate CPU usage (average across all cores)
        let cpu_usage = system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / system.cpus().len() as f32;

        // Calculate memory usage
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage = if total_memory > 0 {
            ((used_memory as f64 / total_memory as f64) * 100.0) as u8
        } else {
            0
        };

        WorkerLoad {
            running_tasks,
            cpu_usage: cpu_usage as u8,
            memory_usage,
        }
    }
}

/// Get system capacity information
pub fn get_system_capacity() -> rpc_protocol::WorkerCapacity {
    let system = System::new_all();

    rpc_protocol::WorkerCapacity {
        max_concurrent_tasks: num_cpus::get() as u32,
        available_memory: system.available_memory(),
        available_cpus: num_cpus::get() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_capacity() {
        let capacity = get_system_capacity();
        assert!(capacity.max_concurrent_tasks > 0);
        assert!(capacity.available_cpus > 0);
    }
}
