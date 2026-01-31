//! Todo item API endpoints.

use std::sync::Arc;

use axum::{extract::State, Json};
use entities::TodoItemStatus as EntityTodoItemStatus;
use rpc_protocol::{
    requests::*,
    responses::*,
    TodoItem, TodoItemStatus, TodoItemType, IssueTriageData, PrReviewData,
};
use task_store::{TaskStore, TodoFilter};
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Converts RPC TodoItemStatus to entity TodoItemStatus.
fn to_entity_status(status: TodoItemStatus) -> EntityTodoItemStatus {
    match status {
        TodoItemStatus::Unspecified | TodoItemStatus::Pending => EntityTodoItemStatus::Pending,
        TodoItemStatus::InProgress => EntityTodoItemStatus::InProgress,
        TodoItemStatus::Completed => EntityTodoItemStatus::Completed,
        TodoItemStatus::Dismissed => EntityTodoItemStatus::Dismissed,
    }
}

/// Converts entity TodoItemStatus to RPC TodoItemStatus.
fn to_rpc_status(status: EntityTodoItemStatus) -> TodoItemStatus {
    match status {
        EntityTodoItemStatus::Pending => TodoItemStatus::Pending,
        EntityTodoItemStatus::InProgress => TodoItemStatus::InProgress,
        EntityTodoItemStatus::Completed => TodoItemStatus::Completed,
        EntityTodoItemStatus::Dismissed => TodoItemStatus::Dismissed,
    }
}

/// Converts entity TodoItem to RPC TodoItem.
fn entity_to_rpc_todo_item(item: &entities::TodoItem) -> TodoItem {
    let (item_type, issue_triage, pr_review) = match &item.data {
        entities::TodoItemData::IssueTriage(data) => (
            TodoItemType::IssueTriage,
            Some(IssueTriageData {
                issue_url: data.issue_url.clone(),
                issue_title: data.issue_title.clone(),
                suggested_labels: data.suggested_labels.clone(),
                suggested_assignees: data.suggested_assignees.clone(),
            }),
            None,
        ),
        entities::TodoItemData::PrReview(data) => (
            TodoItemType::PrReview,
            None,
            Some(PrReviewData {
                pr_url: data.pr_url.clone(),
                pr_title: data.pr_title.clone(),
                changed_files_count: data.changed_files_count as i32,
                ai_summary: data.ai_summary.clone(),
            }),
        ),
    };

    TodoItem {
        id: item.id.to_string(),
        item_type,
        status: to_rpc_status(item.status),
        repository_id: item.repository_id.to_string(),
        issue_triage,
        pr_review,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

/// Lists todo items.
pub async fn list_todo_items<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ListTodoItemsRequest>,
) -> ServerResult<Json<ListTodoItemsResponse>> {
    let filter = TodoFilter {
        repository_id: request.repository_id.as_ref().and_then(|id| id.parse().ok()),
        status: request.status.map(to_entity_status),
        limit: Some(request.limit as u32),
        offset: Some(request.offset as u32),
    };

    let (items, total) = state.store.list_todo_items(filter).await?;

    Ok(Json(ListTodoItemsResponse {
        items: items.iter().map(entity_to_rpc_todo_item).collect(),
        total_count: total as i32,
    }))
}

/// Gets a todo item by ID.
pub async fn get_todo_item<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<GetTodoItemRequest>,
) -> ServerResult<Json<GetTodoItemResponse>> {
    let item_id: Uuid = request
        .item_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid item_id".to_string()))?;

    let item = state
        .store
        .get_todo_item(item_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Todo item not found".to_string()))?;

    Ok(Json(GetTodoItemResponse {
        item: entity_to_rpc_todo_item(&item),
    }))
}

/// Updates a todo item's status.
pub async fn update_todo_status<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<UpdateTodoStatusRequest>,
) -> ServerResult<Json<UpdateTodoStatusResponse>> {
    let item_id: Uuid = request
        .item_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid item_id".to_string()))?;

    let mut item = state
        .store
        .get_todo_item(item_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Todo item not found".to_string()))?;

    item.status = to_entity_status(request.status);
    item.updated_at = chrono::Utc::now();

    let item = state.store.update_todo_item(item).await?;

    tracing::info!(item_id = %item_id, status = ?item.status, "Todo item status updated");

    Ok(Json(UpdateTodoStatusResponse {
        item: entity_to_rpc_todo_item(&item),
    }))
}

/// Dismisses a todo item.
pub async fn dismiss_todo<S: TaskStore>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<DismissTodoRequest>,
) -> ServerResult<Json<DismissTodoResponse>> {
    let item_id: Uuid = request
        .item_id
        .parse()
        .map_err(|_| ServerError::InvalidRequest("Invalid item_id".to_string()))?;

    let mut item = state
        .store
        .get_todo_item(item_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Todo item not found".to_string()))?;

    item.status = EntityTodoItemStatus::Dismissed;
    item.updated_at = chrono::Utc::now();

    state.store.update_todo_item(item).await?;

    tracing::info!(item_id = %item_id, "Todo item dismissed");

    Ok(Json(DismissTodoResponse {}))
}
