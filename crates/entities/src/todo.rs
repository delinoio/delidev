//! TodoItem entity definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a TodoItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemStatus {
    /// Pending action.
    #[default]
    Pending,
    /// In progress.
    InProgress,
    /// Completed.
    Completed,
    /// Dismissed.
    Dismissed,
}

/// Source of a TodoItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemSource {
    /// Automatically created.
    #[default]
    Auto,
    /// Manually created by user.
    Manual,
}

/// Type of TodoItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemType {
    /// Issue that needs triage.
    IssueTriage,
    /// PR that needs review.
    PrReview,
}

/// Data for an issue triage todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTriageData {
    /// Issue URL.
    pub issue_url: String,
    /// Issue title.
    pub issue_title: String,
    /// AI-suggested labels.
    pub suggested_labels: Vec<String>,
    /// AI-suggested assignees.
    pub suggested_assignees: Vec<String>,
}

/// Data for a PR review todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReviewData {
    /// PR URL.
    pub pr_url: String,
    /// PR title.
    pub pr_title: String,
    /// Number of changed files.
    pub changed_files_count: u32,
    /// AI analysis summary.
    pub ai_summary: Option<String>,
}

/// Data associated with a TodoItem.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TodoItemData {
    /// Issue triage data.
    IssueTriage(IssueTriageData),
    /// PR review data.
    PrReview(PrReviewData),
}

/// Tasks that humans should do but AI can assist with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier.
    pub id: Uuid,
    /// Type of todo item.
    pub item_type: TodoItemType,
    /// Source of the item.
    pub source: TodoItemSource,
    /// Current status.
    pub status: TodoItemStatus,
    /// Associated repository ID.
    pub repository_id: Uuid,
    /// Type-specific data.
    pub data: TodoItemData,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated.
    pub updated_at: DateTime<Utc>,
}

impl TodoItem {
    /// Creates a new issue triage todo item.
    pub fn issue_triage(repository_id: Uuid, issue_url: String, issue_title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            item_type: TodoItemType::IssueTriage,
            source: TodoItemSource::Auto,
            status: TodoItemStatus::Pending,
            repository_id,
            data: TodoItemData::IssueTriage(IssueTriageData {
                issue_url,
                issue_title,
                suggested_labels: Vec::new(),
                suggested_assignees: Vec::new(),
            }),
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new PR review todo item.
    pub fn pr_review(
        repository_id: Uuid,
        pr_url: String,
        pr_title: String,
        changed_files_count: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            item_type: TodoItemType::PrReview,
            source: TodoItemSource::Auto,
            status: TodoItemStatus::Pending,
            repository_id,
            data: TodoItemData::PrReview(PrReviewData {
                pr_url,
                pr_title,
                changed_files_count,
                ai_summary: None,
            }),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_triage_creation() {
        let repo_id = Uuid::new_v4();
        let item = TodoItem::issue_triage(
            repo_id,
            "https://github.com/user/repo/issues/1".to_string(),
            "Bug report".to_string(),
        );

        assert_eq!(item.item_type, TodoItemType::IssueTriage);
        assert_eq!(item.repository_id, repo_id);
        assert_eq!(item.status, TodoItemStatus::Pending);

        if let TodoItemData::IssueTriage(data) = &item.data {
            assert_eq!(data.issue_url, "https://github.com/user/repo/issues/1");
            assert_eq!(data.issue_title, "Bug report");
        } else {
            panic!("Expected IssueTriage data");
        }
    }

    #[test]
    fn test_pr_review_creation() {
        let repo_id = Uuid::new_v4();
        let item = TodoItem::pr_review(
            repo_id,
            "https://github.com/user/repo/pull/1".to_string(),
            "Add feature".to_string(),
            5,
        );

        assert_eq!(item.item_type, TodoItemType::PrReview);
        assert_eq!(item.repository_id, repo_id);

        if let TodoItemData::PrReview(data) = &item.data {
            assert_eq!(data.pr_url, "https://github.com/user/repo/pull/1");
            assert_eq!(data.pr_title, "Add feature");
            assert_eq!(data.changed_files_count, 5);
        } else {
            panic!("Expected PrReview data");
        }
    }
}
