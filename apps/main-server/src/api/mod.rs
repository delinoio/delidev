//! API endpoints.

pub mod auth;
pub mod repository;
pub mod secrets;
pub mod session;
pub mod task;
pub mod todo;
pub mod worker;
pub mod workspace;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use task_store::TaskStore;

use crate::state::AppState;

/// Creates the API router with all endpoints.
pub fn create_router<S: TaskStore + 'static>() -> Router<Arc<AppState<S>>> {
    Router::new()
        // Task management endpoints
        .route("/api/task/create-unit", post(task::create_unit_task))
        .route("/api/task/create-composite", post(task::create_composite_task))
        .route("/api/task/get", post(task::get_task))
        .route("/api/task/list", post(task::list_tasks))
        .route("/api/task/update-status", post(task::update_task_status))
        .route("/api/task/delete", post(task::delete_task))
        .route("/api/task/retry", post(task::retry_task))
        .route("/api/task/approve", post(task::approve_task))
        .route("/api/task/reject", post(task::reject_task))
        .route("/api/task/request-changes", post(task::request_changes))
        // Session endpoints
        .route("/api/session/get-log", post(session::get_log))
        .route("/api/session/stop", post(session::stop_session))
        .route("/api/session/submit-tty-input", post(session::submit_tty_input))
        // Repository endpoints
        .route("/api/repository/add", post(repository::add_repository))
        .route("/api/repository/list", post(repository::list_repositories))
        .route("/api/repository/get", post(repository::get_repository))
        .route("/api/repository/remove", post(repository::remove_repository))
        .route("/api/repository-group/create", post(repository::create_repository_group))
        .route("/api/repository-group/list", post(repository::list_repository_groups))
        .route("/api/repository-group/update", post(repository::update_repository_group))
        .route("/api/repository-group/delete", post(repository::delete_repository_group))
        // Workspace endpoints
        .route("/api/workspace/create", post(workspace::create_workspace))
        .route("/api/workspace/list", post(workspace::list_workspaces))
        .route("/api/workspace/get", post(workspace::get_workspace))
        .route("/api/workspace/update", post(workspace::update_workspace))
        .route("/api/workspace/delete", post(workspace::delete_workspace))
        // Todo endpoints
        .route("/api/todo/list", post(todo::list_todo_items))
        .route("/api/todo/get", post(todo::get_todo_item))
        .route("/api/todo/update-status", post(todo::update_todo_status))
        .route("/api/todo/dismiss", post(todo::dismiss_todo))
        // Secrets endpoints
        .route("/api/secrets/send", post(secrets::send_secrets))
        .route("/api/secrets/clear", post(secrets::clear_secrets))
        // Auth endpoints
        .route("/api/auth/get-login-url", post(auth::get_login_url))
        .route("/api/auth/callback", get(auth::handle_callback))
        .route("/api/auth/refresh", post(auth::refresh_token))
        .route("/api/auth/me", get(auth::get_current_user))
        .route("/api/auth/logout", post(auth::logout))
        // Worker endpoints (internal)
        .route("/api/worker/register", post(worker::register_worker))
        .route("/api/worker/heartbeat", post(worker::heartbeat))
        .route("/api/worker/unregister", post(worker::unregister_worker))
        .route("/api/worker/get-task", post(worker::get_next_task))
        .route("/api/worker/report-status", post(worker::report_task_status))
        .route("/api/worker/get-secrets", post(worker::get_secrets))
        // Health check
        .route("/health", get(health_check))
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}
