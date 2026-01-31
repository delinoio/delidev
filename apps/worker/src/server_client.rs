//! Client for communication with the main server

#![allow(dead_code)]

use coding_agents::NormalizedMessage;
use rpc_protocol::{
    JsonRpcRequest, JsonRpcResponse, RegisterWorkerRequest, RegisterWorkerResponse,
    ReportTaskCompleteRequest, SendExecutionLogRequest, SuccessResponse, WorkerCapacity,
    WorkerHeartbeatRequest, WorkerHeartbeatResponse, WorkerLoad,
};
use thiserror::Error;
use tracing::debug;

/// Client for communication with the main server
pub struct MainServerClient {
    /// Server URL
    server_url: String,
    /// HTTP client
    http_client: reqwest::Client,
    /// Request counter for JSON-RPC IDs
    request_id: std::sync::atomic::AtomicU64,
}

impl MainServerClient {
    /// Create a new main server client
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            http_client: reqwest::Client::new(),
            request_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Generate the next request ID
    fn next_id(&self) -> String {
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        id.to_string()
    }

    /// Make an RPC call to the server
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, ClientError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(rpc_protocol::RequestId::String(self.next_id())),
            method: method.to_string(),
            params,
        };

        debug!(method = %method, "Making RPC call");

        let response = self
            .http_client
            .post(format!("{}/rpc", self.server_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Server returned status {}",
                response.status()
            )));
        }

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| ClientError::Deserialization(e.to_string()))?;

        match rpc_response.result {
            Some(result) => serde_json::from_value(result)
                .map_err(|e| ClientError::Deserialization(e.to_string())),
            None => {
                if let Some(error) = rpc_response.error {
                    Err(ClientError::RpcError {
                        code: error.code,
                        message: error.message,
                    })
                } else {
                    Err(ClientError::ServerError(
                        "Response missing both result and error".to_string(),
                    ))
                }
            }
        }
    }

    /// Register this worker with the main server
    pub async fn register_worker(
        &self,
        worker_id: &str,
        address: &str,
        capacity: WorkerCapacity,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        let request = RegisterWorkerRequest {
            worker_id: worker_id.to_string(),
            address: address.to_string(),
            capacity,
        };

        self.call("registerWorker", serde_json::to_value(request).unwrap())
            .await
    }

    /// Send heartbeat to the main server
    pub async fn heartbeat(
        &self,
        worker_id: &str,
        load: WorkerLoad,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        let request = WorkerHeartbeatRequest {
            worker_id: worker_id.to_string(),
            current_load: load,
        };

        self.call("workerHeartbeat", serde_json::to_value(request).unwrap())
            .await
    }

    /// Report task completion to the main server
    pub async fn report_task_complete(
        &self,
        task_id: &str,
        success: bool,
        summary: Option<String>,
        exit_code: Option<i32>,
    ) -> Result<SuccessResponse, ClientError> {
        let request = ReportTaskCompleteRequest {
            task_id: task_id.to_string(),
            success,
            summary,
            exit_code,
        };

        self.call("reportTaskComplete", serde_json::to_value(request).unwrap())
            .await
    }

    /// Send execution log to the main server
    pub async fn send_execution_log(
        &self,
        task_id: &str,
        session_id: &str,
        message: NormalizedMessage,
    ) -> Result<SuccessResponse, ClientError> {
        let request = SendExecutionLogRequest {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            message,
        };

        self.call("sendExecutionLog", serde_json::to_value(request).unwrap())
            .await
    }

    /// Check server health
    pub async fn health_check(&self) -> Result<(), ClientError> {
        let response = self
            .http_client
            .get(format!("{}/health", self.server_url))
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ClientError::ServerError(format!(
                "Health check failed with status {}",
                response.status()
            )))
        }
    }
}

impl Clone for MainServerClient {
    fn clone(&self) -> Self {
        Self {
            server_url: self.server_url.clone(),
            http_client: self.http_client.clone(),
            request_id: std::sync::atomic::AtomicU64::new(
                self.request_id.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

/// Client errors
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("RPC error (code {code}): {message}")]
    RpcError { code: i32, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MainServerClient::new("http://localhost:54871");
        assert_eq!(client.server_url, "http://localhost:54871");
    }

    #[test]
    fn test_next_id() {
        let client = MainServerClient::new("http://localhost:54871");
        assert_eq!(client.next_id(), "1");
        assert_eq!(client.next_id(), "2");
        assert_eq!(client.next_id(), "3");
    }
}
