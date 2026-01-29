use serde::{Deserialize, Serialize};

/// TodoItem source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemSource {
    /// System auto-generated
    Auto,
    /// User manually added
    Manual,
}

/// TodoItem status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TodoItemStatus {
    /// Waiting
    #[default]
    Pending,
    /// In progress
    InProgress,
    /// Completed
    Done,
    /// Dismissed
    Dismissed,
}

/// TodoItem type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoItemType {
    IssueTriage,
    PrReview,
}

/// Base TodoItem fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItemBase {
    /// Unique identifier
    pub id: String,
    /// Creation source
    pub source: TodoItemSource,
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Current status
    pub status: TodoItemStatus,
}

/// VCS provider issue triage task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTriageTodoItem {
    #[serde(flatten)]
    pub base: TodoItemBase,
    /// Issue URL
    pub issue_url: String,
    /// Repository ID
    pub repository_id: String,
    /// Issue title
    pub issue_title: String,
    /// AI suggested labels
    pub suggested_labels: Vec<String>,
    /// AI suggested assignees
    pub suggested_assignees: Vec<String>,
}

/// PR review task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReviewTodoItem {
    #[serde(flatten)]
    pub base: TodoItemBase,
    /// PR/MR URL
    pub pr_url: String,
    /// Repository ID
    pub repository_id: String,
    /// PR title
    pub pr_title: String,
    /// Number of changed files
    pub changed_files_count: u32,
    /// AI analysis summary
    pub ai_summary: Option<String>,
}

/// Tagged union for TodoItem
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TodoItem {
    IssueTriage(IssueTriageTodoItem),
    PrReview(PrReviewTodoItem),
}

impl TodoItem {
    pub fn id(&self) -> &str {
        match self {
            Self::IssueTriage(item) => &item.base.id,
            Self::PrReview(item) => &item.base.id,
        }
    }

    pub fn status(&self) -> TodoItemStatus {
        match self {
            Self::IssueTriage(item) => item.base.status,
            Self::PrReview(item) => item.base.status,
        }
    }

    pub fn repository_id(&self) -> &str {
        match self {
            Self::IssueTriage(item) => &item.repository_id,
            Self::PrReview(item) => &item.repository_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod todo_item {
        use super::*;

        fn create_issue_triage_item() -> TodoItem {
            TodoItem::IssueTriage(IssueTriageTodoItem {
                base: TodoItemBase {
                    id: "todo-1".to_string(),
                    source: TodoItemSource::Auto,
                    created_at: chrono::Utc::now(),
                    status: TodoItemStatus::Pending,
                },
                issue_url: "https://github.com/owner/repo/issues/1".to_string(),
                repository_id: "repo-1".to_string(),
                issue_title: "Fix bug".to_string(),
                suggested_labels: vec![],
                suggested_assignees: vec![],
            })
        }

        fn create_pr_review_item() -> TodoItem {
            TodoItem::PrReview(PrReviewTodoItem {
                base: TodoItemBase {
                    id: "todo-2".to_string(),
                    source: TodoItemSource::Manual,
                    created_at: chrono::Utc::now(),
                    status: TodoItemStatus::Done,
                },
                pr_url: "https://github.com/owner/repo/pull/42".to_string(),
                repository_id: "repo-2".to_string(),
                pr_title: "Add feature".to_string(),
                changed_files_count: 5,
                ai_summary: None,
            })
        }

        #[test]
        fn test_accessor_methods_work_for_all_variants() {
            let issue_item = create_issue_triage_item();
            let pr_item = create_pr_review_item();

            // id() method works for both variants
            assert_eq!(issue_item.id(), "todo-1");
            assert_eq!(pr_item.id(), "todo-2");

            // status() method works for both variants
            assert_eq!(issue_item.status(), TodoItemStatus::Pending);
            assert_eq!(pr_item.status(), TodoItemStatus::Done);

            // repository_id() method works for both variants
            assert_eq!(issue_item.repository_id(), "repo-1");
            assert_eq!(pr_item.repository_id(), "repo-2");
        }

        #[test]
        fn test_tagged_union_serialization_includes_type_discriminator() {
            let issue_item = create_issue_triage_item();
            let pr_item = create_pr_review_item();

            let issue_json = serde_json::to_string(&issue_item).unwrap();
            let pr_json = serde_json::to_string(&pr_item).unwrap();

            assert!(issue_json.contains("\"type\":\"issue_triage\""));
            assert!(pr_json.contains("\"type\":\"pr_review\""));
        }

        #[test]
        fn test_tagged_union_deserialization_from_json() {
            let json = r#"{
                "type": "pr_review",
                "id": "todo-3",
                "source": "auto",
                "created_at": "2024-01-01T00:00:00Z",
                "status": "pending",
                "pr_url": "https://github.com/owner/repo/pull/1",
                "repository_id": "repo-1",
                "pr_title": "Test PR",
                "changed_files_count": 3,
                "ai_summary": null
            }"#;

            let item: TodoItem = serde_json::from_str(json).unwrap();
            assert_eq!(item.id(), "todo-3");
            assert_eq!(item.repository_id(), "repo-1");
        }
    }
}
