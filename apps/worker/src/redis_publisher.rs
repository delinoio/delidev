//! Redis publisher for execution logs
//!
//! This module enables workers to publish execution logs directly to Redis,
//! allowing for efficient distributed event streaming to connected clients.

#![allow(dead_code)]

use coding_agents::NormalizedMessage;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Channel prefix for task logs (must match server's redis_broadcaster)
const CHANNEL_PREFIX: &str = "delidev:task:";
const CHANNEL_SUFFIX: &str = ":logs";

/// A log entry for Redis PubSub (matches server's RedisLogEntry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisLogEntry {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// The log message
    pub message: NormalizedMessage,
}

impl RedisLogEntry {
    /// Create a new Redis log entry
    pub fn new(task_id: String, session_id: String, message: NormalizedMessage) -> Self {
        Self {
            task_id,
            session_id,
            message,
        }
    }

    /// Get the Redis channel name for this entry
    pub fn channel(&self) -> String {
        format!("{}{}{}", CHANNEL_PREFIX, self.task_id, CHANNEL_SUFFIX)
    }
}

/// Get the Redis channel name for a task
pub fn task_channel(task_id: &str) -> String {
    format!("{}{}{}", CHANNEL_PREFIX, task_id, CHANNEL_SUFFIX)
}

/// Redis publisher for execution logs
#[derive(Clone)]
pub struct RedisPublisher {
    /// Redis connection manager
    conn: Arc<RwLock<Option<ConnectionManager>>>,
    /// Redis URL
    redis_url: String,
}

impl RedisPublisher {
    /// Create a new Redis publisher
    pub fn new(redis_url: &str) -> Self {
        Self {
            conn: Arc::new(RwLock::new(None)),
            redis_url: redis_url.to_string(),
        }
    }

    /// Connect to Redis
    pub async fn connect(&self) -> Result<(), RedisPublisherError> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| RedisPublisherError::Connection(e.to_string()))?;

        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| RedisPublisherError::Connection(e.to_string()))?;

        *self.conn.write().await = Some(manager);
        info!("Redis publisher connected");
        Ok(())
    }

    /// Check if connected to Redis
    pub async fn is_connected(&self) -> bool {
        self.conn.read().await.is_some()
    }

    /// Publish an execution log to Redis
    pub async fn publish(
        &self,
        task_id: &str,
        session_id: &str,
        message: NormalizedMessage,
    ) -> Result<(), RedisPublisherError> {
        let entry = RedisLogEntry::new(task_id.to_string(), session_id.to_string(), message);
        let channel = entry.channel();

        let payload = serde_json::to_string(&entry)
            .map_err(|e| RedisPublisherError::Serialization(e.to_string()))?;

        let mut conn_guard = self.conn.write().await;
        let conn = conn_guard
            .as_mut()
            .ok_or(RedisPublisherError::NotConnected)?;

        redis::cmd("PUBLISH")
            .arg(&channel)
            .arg(&payload)
            .query_async::<i64>(conn)
            .await
            .map_err(|e| RedisPublisherError::Publish(e.to_string()))?;

        debug!(channel = %channel, "Published log to Redis");
        Ok(())
    }

    /// Publish a batch of execution logs
    pub async fn publish_batch(
        &self,
        task_id: &str,
        session_id: &str,
        messages: Vec<NormalizedMessage>,
    ) -> Result<(), RedisPublisherError> {
        for message in messages {
            self.publish(task_id, session_id, message).await?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for RedisPublisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisPublisher")
            .field("redis_url", &self.redis_url)
            .finish()
    }
}

/// Redis publisher errors
#[derive(Debug, thiserror::Error)]
pub enum RedisPublisherError {
    #[error("Redis connection error: {0}")]
    Connection(String),

    #[error("Redis not connected")]
    NotConnected,

    #[error("Failed to publish: {0}")]
    Publish(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_name() {
        let channel = task_channel("task-123");
        assert_eq!(channel, "delidev:task:task-123:logs");
    }

    #[test]
    fn test_redis_log_entry_channel() {
        use chrono::Utc;

        let entry = RedisLogEntry::new(
            "task-456".to_string(),
            "session-1".to_string(),
            NormalizedMessage::Text {
                content: "Hello".to_string(),
                timestamp: Utc::now(),
            },
        );

        assert_eq!(entry.channel(), "delidev:task:task-456:logs");
    }
}
