//! Agent session API endpoints.

use std::sync::Arc;

use axum::{extract::State, Json};
use rpc_protocol::{requests::*, responses::*};
use task_store::TaskStore;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Gets the output log for an agent session.
pub async fn get_log<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetLogRequest>,
) -> ServerResult<Json<GetLogResponse>> {
    let session_id: Uuid = request
        .session_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid session_id".to_string()))?;

    let session = state
        .store
        .get_agent_session(session_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Session not found".to_string()))?;

    Ok(Json(GetLogResponse {
        log: session.output_log.unwrap_or_default(),
    }))
}

/// Stops a running agent session.
pub async fn stop_session<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<StopSessionRequest>,
) -> ServerResult<Json<StopSessionResponse>> {
    let session_id: Uuid = request
        .session_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid session_id".to_string()))?;

    let mut session = state
        .store
        .get_agent_session(session_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Session not found".to_string()))?;

    // Mark session as completed
    session.completed_at = Some(chrono::Utc::now());
    state.store.update_agent_session(session).await?;

    tracing::info!(session_id = %session_id, "Session stopped");

    // TODO: Actually stop the running agent process via worker

    Ok(Json(StopSessionResponse {}))
}

/// Submits a response to a TTY input request.
pub async fn submit_tty_input<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<SubmitTtyInputRequest>,
) -> ServerResult<Json<SubmitTtyInputResponse>> {
    let request_id: Uuid = request
        .request_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid request_id".to_string()))?;

    let mut tty_request = state
        .store
        .get_tty_input_request(request_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("TTY input request not found".to_string()))?;

    // Update the request with the response
    tty_request.response = Some(request.response);
    tty_request.status = entities::TtyInputStatus::Responded;
    tty_request.responded_at = Some(chrono::Utc::now());
    state.store.update_tty_input_request(tty_request).await?;

    tracing::info!(request_id = %request_id, "TTY input submitted");

    // TODO: Relay response to worker running the agent

    Ok(Json(SubmitTtyInputResponse {}))
}
