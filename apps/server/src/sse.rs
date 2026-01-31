//! Server-Sent Events (SSE) handler for real-time log streaming
//!
//! This module provides an SSE endpoint for clients to receive execution logs
//! in real-time. It supports both Redis-backed streaming (for distributed mode)
//! and in-memory streaming (for single-process mode).

use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use futures_util::StreamExt;
use rpc_protocol::ExecutionLogNotification;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};

use crate::state::AppState;

/// SSE endpoint for subscribing to execution logs
///
/// GET /events/{task_id}
///
/// This endpoint streams execution logs for a specific task using SSE.
/// Clients can use the standard EventSource API to connect.
pub async fn handle_sse(
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    info!(task_id = %task_id, "SSE client connected");

    // Check if Redis is configured and connected
    if let Some(ref redis) = state.redis_broadcaster {
        if redis.is_connected().await {
            // Use Redis subscription for distributed mode
            return create_redis_sse_stream(redis.clone(), task_id)
                .await
                .into_response();
        }
    }

    // Fall back to in-memory broadcast
    create_memory_sse_stream(state, task_id).into_response()
}

/// Create an SSE stream backed by Redis PubSub
async fn create_redis_sse_stream(
    redis: crate::redis_broadcaster::RedisBroadcaster,
    task_id: String,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        match redis.subscribe(&task_id).await {
            Ok(mut subscription) => {
                if let Err(e) = subscription.subscribe().await {
                    error!("Failed to subscribe to Redis channel: {}", e);
                    return;
                }

                debug!(task_id = %task_id, "Streaming from Redis");

                loop {
                    match subscription.recv().await {
                        Some(entry) => {
                            let notification = ExecutionLogNotification {
                                task_id: entry.task_id.clone(),
                                session_id: entry.session_id.clone(),
                                message: entry.message.clone(),
                            };

                            match serde_json::to_string(&notification) {
                                Ok(json) => {
                                    yield Ok::<_, Infallible>(Event::default().data(json).event("log"));
                                }
                                Err(e) => {
                                    warn!("Failed to serialize notification: {}", e);
                                }
                            }
                        }
                        None => {
                            // Stream ended
                            debug!(task_id = %task_id, "Redis stream ended");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to create Redis subscription: {}", e);
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Create an SSE stream backed by in-memory broadcast
fn create_memory_sse_stream(state: AppState, task_id: String) -> impl IntoResponse {
    let receiver = state.log_broadcaster.subscribe(&task_id);
    let broadcast_stream = BroadcastStream::new(receiver);

    let stream = async_stream::stream! {
        let mut stream = broadcast_stream;

        debug!(task_id = %task_id, "Streaming from in-memory broadcaster");

        while let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    let notification = ExecutionLogNotification {
                        task_id: entry.task_id.clone(),
                        session_id: entry.session_id.clone(),
                        message: entry.message.clone(),
                    };

                    match serde_json::to_string(&notification) {
                        Ok(json) => {
                            yield Ok::<_, Infallible>(Event::default().data(json).event("log"));
                        }
                        Err(e) => {
                            warn!("Failed to serialize notification: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Broadcast receiver lagged: {}", e);
                }
            }
        }

        debug!(task_id = %task_id, "In-memory stream ended");
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_format() {
        // Just verify the event types are correctly defined
        let event = Event::default().data("test").event("log");
        // Event doesn't implement Debug or PartialEq, so just verify it compiles
        let _ = event;
    }
}
