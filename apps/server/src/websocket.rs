//! WebSocket handler for real-time subscriptions

use std::collections::HashMap;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use rpc_protocol::{
    method_names, ExecutionLogNotification, JsonRpcRequest, JsonRpcResponse,
    SubscribeExecutionLogsRequest,
};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::{log_broadcaster::LogEntry, state::AppState};

/// Handle WebSocket upgrade
pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Map of subscribed task IDs to their receivers
    let mut subscriptions: HashMap<String, broadcast::Receiver<LogEntry>> = HashMap::new();

    // Channel for outgoing messages
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(100);

    // Task to send messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    loop {
        tokio::select! {
            // Handle incoming messages
            msg_result = receiver.next() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_message(&text, &state, &mut subscriptions, &tx).await {
                            warn!("Error handling WebSocket message: {}", e);
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // Stream ended
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Process subscription messages
        for (_task_id, sub_rx) in subscriptions.iter_mut() {
            while let Ok(entry) = sub_rx.try_recv() {
                let notification = ExecutionLogNotification {
                    task_id: entry.task_id.clone(),
                    session_id: entry.session_id.clone(),
                    message: entry.message.clone(),
                };

                if let Ok(json) = serde_json::to_string(&notification) {
                    let _ = tx.send(Message::Text(json.into())).await;
                }
            }
        }
    }

    // Clean up
    send_task.abort();
}

/// Handle an incoming WebSocket message
async fn handle_message(
    text: &str,
    state: &AppState,
    subscriptions: &mut HashMap<String, broadcast::Receiver<LogEntry>>,
    tx: &tokio::sync::mpsc::Sender<Message>,
) -> Result<(), String> {
    let request: JsonRpcRequest =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON-RPC request: {}", e))?;

    match request.method.as_str() {
        method_names::SUBSCRIBE_EXECUTION_LOGS => {
            let params: SubscribeExecutionLogsRequest =
                serde_json::from_value(request.params.clone())
                    .map_err(|e| format!("Invalid params: {}", e))?;

            // Subscribe to the task's log stream
            let receiver = state.log_broadcaster.subscribe(&params.task_id);
            subscriptions.insert(params.task_id.clone(), receiver);

            // Send success response
            let response = JsonRpcResponse::success(
                request.id,
                serde_json::json!({ "subscribed": true, "taskId": params.task_id }),
            );
            let json = serde_json::to_string(&response).unwrap();
            let _ = tx.send(Message::Text(json.into())).await;

            info!(task_id = %params.task_id, "Client subscribed to execution logs");
        }

        method_names::UNSUBSCRIBE_EXECUTION_LOGS => {
            let params: SubscribeExecutionLogsRequest =
                serde_json::from_value(request.params.clone())
                    .map_err(|e| format!("Invalid params: {}", e))?;

            subscriptions.remove(&params.task_id);

            let response = JsonRpcResponse::success(
                request.id,
                serde_json::json!({ "unsubscribed": true, "taskId": params.task_id }),
            );
            let json = serde_json::to_string(&response).unwrap();
            let _ = tx.send(Message::Text(json.into())).await;

            info!(task_id = %params.task_id, "Client unsubscribed from execution logs");
        }

        method => {
            let response = JsonRpcResponse::error(
                request.id,
                rpc_protocol::JsonRpcError::method_not_found(method),
            );
            let json = serde_json::to_string(&response).unwrap();
            let _ = tx.send(Message::Text(json.into())).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // WebSocket tests would require more complex setup with actual connections
    // These are placeholder tests

    #[test]
    fn test_message_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":"1","method":"subscribeExecutionLogs","params":{"taskId":"task-1"}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "subscribeExecutionLogs");
    }
}
