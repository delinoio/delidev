//! Redis-backed log broadcasting for distributed event streaming
//!
//! This module provides Redis PubSub integration for distributing execution logs
//! across multiple workers and clients. It extends the in-memory LogBroadcaster
//! to support distributed scenarios.

#![allow(dead_code)]

use std::sync::Arc;

use coding_agents::NormalizedMessage;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Channel prefix for task logs
const CHANNEL_PREFIX: &str = "delidev:task:";
const CHANNEL_SUFFIX: &str = ":logs";

/// A log entry for Redis PubSub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisLogEntry {
    /// Task ID
    pub task_id: String,
    /// Session ID
    pub session_id: String,
    /// The log message (serialized as JSON)
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

/// Redis broadcaster for distributed log streaming
#[derive(Clone)]
pub struct RedisBroadcaster {
    /// Redis connection manager for publishing
    conn: Arc<RwLock<Option<ConnectionManager>>>,
    /// Redis URL for connecting
    redis_url: String,
}

impl RedisBroadcaster {
    /// Create a new Redis broadcaster
    pub fn new(redis_url: &str) -> Self {
        Self {
            conn: Arc::new(RwLock::new(None)),
            redis_url: redis_url.to_string(),
        }
    }

    /// Initialize the Redis connection
    pub async fn connect(&self) -> Result<(), RedisError> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| RedisError::Connection(e.to_string()))?;

        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| RedisError::Connection(e.to_string()))?;

        *self.conn.write().await = Some(manager);
        info!("Redis broadcaster connected");
        Ok(())
    }

    /// Check if connected to Redis
    pub async fn is_connected(&self) -> bool {
        self.conn.read().await.is_some()
    }

    /// Publish a log entry to Redis
    pub async fn publish(
        &self,
        task_id: &str,
        session_id: &str,
        message: NormalizedMessage,
    ) -> Result<(), RedisError> {
        let entry = RedisLogEntry::new(task_id.to_string(), session_id.to_string(), message);
        let channel = entry.channel();

        let payload =
            serde_json::to_string(&entry).map_err(|e| RedisError::Serialization(e.to_string()))?;

        let mut conn_guard = self.conn.write().await;
        let conn = conn_guard
            .as_mut()
            .ok_or_else(|| RedisError::NotConnected)?;

        redis::cmd("PUBLISH")
            .arg(&channel)
            .arg(&payload)
            .query_async::<i64>(conn)
            .await
            .map_err(|e| RedisError::Publish(e.to_string()))?;

        debug!(channel = %channel, "Published log entry to Redis");
        Ok(())
    }

    /// Subscribe to a task's log stream
    /// Returns an async stream of log entries
    pub async fn subscribe(&self, task_id: &str) -> Result<RedisSubscription, RedisError> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| RedisError::Connection(e.to_string()))?;

        let pubsub = client
            .get_async_pubsub()
            .await
            .map_err(|e| RedisError::Connection(e.to_string()))?;

        let channel = task_channel(task_id);

        Ok(RedisSubscription::new(pubsub, channel))
    }
}

impl std::fmt::Debug for RedisBroadcaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisBroadcaster")
            .field("redis_url", &self.redis_url)
            .finish()
    }
}

/// A subscription to a Redis channel
pub struct RedisSubscription {
    pubsub: redis::aio::PubSub,
    channel: String,
    subscribed: bool,
}

impl RedisSubscription {
    /// Create a new subscription
    fn new(pubsub: redis::aio::PubSub, channel: String) -> Self {
        Self {
            pubsub,
            channel,
            subscribed: false,
        }
    }

    /// Subscribe to the channel
    pub async fn subscribe(&mut self) -> Result<(), RedisError> {
        if !self.subscribed {
            self.pubsub
                .subscribe(&self.channel)
                .await
                .map_err(|e| RedisError::Subscribe(e.to_string()))?;
            self.subscribed = true;
            info!(channel = %self.channel, "Subscribed to Redis channel");
        }
        Ok(())
    }

    /// Receive the next log entry
    pub async fn recv(&mut self) -> Option<RedisLogEntry> {
        use futures_util::StreamExt;

        let msg = self.pubsub.on_message().next().await?;

        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to get message payload: {}", e);
                return None;
            }
        };

        match serde_json::from_str::<RedisLogEntry>(&payload) {
            Ok(entry) => Some(entry),
            Err(e) => {
                warn!("Failed to deserialize log entry: {}", e);
                None
            }
        }
    }
}

/// Redis broadcaster errors
#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("Redis connection error: {0}")]
    Connection(String),

    #[error("Redis not connected")]
    NotConnected,

    #[error("Failed to publish: {0}")]
    Publish(String),

    #[error("Failed to subscribe: {0}")]
    Subscribe(String),

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
    fn test_redis_log_entry_serialization() {
        use chrono::Utc;

        let entry = RedisLogEntry::new(
            "task-1".to_string(),
            "session-1".to_string(),
            NormalizedMessage::Text {
                content: "Hello".to_string(),
                timestamp: Utc::now(),
            },
        );

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RedisLogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.task_id, "task-1");
        assert_eq!(deserialized.session_id, "session-1");
    }
}
