//! Log broadcasting for real-time execution streaming

#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use coding_agents::NormalizedMessage;
use tokio::sync::broadcast;

/// Capacity for log broadcast channels
const CHANNEL_CAPACITY: usize = 1024;

/// Broadcaster for execution logs
#[derive(Debug)]
pub struct LogBroadcaster {
    /// Map of task_id to broadcast sender
    senders: RwLock<HashMap<String, broadcast::Sender<LogEntry>>>,
}

/// A log entry with metadata
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// The log message
    pub message: NormalizedMessage,
}

impl LogBroadcaster {
    /// Create a new log broadcaster
    pub fn new() -> Self {
        Self {
            senders: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to logs for a specific task
    pub fn subscribe(&self, task_id: &str) -> broadcast::Receiver<LogEntry> {
        let mut senders = self.senders.write().unwrap();

        // Get or create sender for this task
        let sender = senders
            .entry(task_id.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);

        sender.subscribe()
    }

    /// Broadcast a log entry for a task
    pub fn broadcast(&self, task_id: &str, session_id: &str, message: NormalizedMessage) {
        let senders = self.senders.read().unwrap();

        if let Some(sender) = senders.get(task_id) {
            let entry = LogEntry {
                task_id: task_id.to_string(),
                session_id: session_id.to_string(),
                message,
            };

            // Ignore send errors (no subscribers)
            let _ = sender.send(entry);
        }
    }

    /// Check if a task has any subscribers
    pub fn has_subscribers(&self, task_id: &str) -> bool {
        let senders = self.senders.read().unwrap();

        if let Some(sender) = senders.get(task_id) {
            sender.receiver_count() > 0
        } else {
            false
        }
    }

    /// Get the subscriber count for a task
    pub fn subscriber_count(&self, task_id: &str) -> usize {
        let senders = self.senders.read().unwrap();

        senders
            .get(task_id)
            .map(|s| s.receiver_count())
            .unwrap_or(0)
    }

    /// Cleanup channels with no subscribers
    pub fn cleanup_empty_channels(&self) {
        let mut senders = self.senders.write().unwrap();
        senders.retain(|_, sender| sender.receiver_count() > 0);
    }

    /// Get all active task IDs
    pub fn active_tasks(&self) -> Vec<String> {
        let senders = self.senders.read().unwrap();
        senders.keys().cloned().collect()
    }
}

impl Default for LogBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for broadcasting logs to a specific task
pub struct TaskLogHandle {
    broadcaster: Arc<LogBroadcaster>,
    task_id: String,
    session_id: String,
}

impl TaskLogHandle {
    /// Create a new task log handle
    pub fn new(broadcaster: Arc<LogBroadcaster>, task_id: String, session_id: String) -> Self {
        Self {
            broadcaster,
            task_id,
            session_id,
        }
    }

    /// Broadcast a log message
    pub fn broadcast(&self, message: NormalizedMessage) {
        self.broadcaster
            .broadcast(&self.task_id, &self.session_id, message);
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_subscribe_and_broadcast() {
        let broadcaster = LogBroadcaster::new();

        let mut receiver = broadcaster.subscribe("task-1");

        let message = NormalizedMessage::Text {
            content: "Hello, world!".to_string(),
            timestamp: Utc::now(),
        };

        broadcaster.broadcast("task-1", "session-1", message);

        // Use try_recv since broadcast happens synchronously
        let entry = receiver.try_recv().unwrap();
        assert_eq!(entry.task_id, "task-1");
        assert_eq!(entry.session_id, "session-1");
    }

    #[test]
    fn test_multiple_subscribers() {
        let broadcaster = LogBroadcaster::new();

        let mut rx1 = broadcaster.subscribe("task-1");
        let mut rx2 = broadcaster.subscribe("task-1");

        assert_eq!(broadcaster.subscriber_count("task-1"), 2);

        let message = NormalizedMessage::Text {
            content: "Test".to_string(),
            timestamp: Utc::now(),
        };

        broadcaster.broadcast("task-1", "session-1", message);

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_no_cross_task_messages() {
        let broadcaster = LogBroadcaster::new();

        let mut rx1 = broadcaster.subscribe("task-1");
        let _rx2 = broadcaster.subscribe("task-2");

        let message = NormalizedMessage::Text {
            content: "For task-2 only".to_string(),
            timestamp: Utc::now(),
        };

        broadcaster.broadcast("task-2", "session-1", message);

        // rx1 should not receive the message
        assert!(rx1.try_recv().is_err());
    }
}
