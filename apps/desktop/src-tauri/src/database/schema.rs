use sqlx::FromRow;

use crate::entities::*;

/// Database row for Workspace
#[derive(Debug, FromRow)]
pub struct WorkspaceRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<WorkspaceRow> for Workspace {
    fn from(row: WorkspaceRow) -> Self {
        Workspace {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

impl From<&Workspace> for WorkspaceRow {
    fn from(ws: &Workspace) -> Self {
        Self {
            id: ws.id.clone(),
            name: ws.name.clone(),
            description: ws.description.clone(),
            created_at: ws.created_at.to_rfc3339(),
            updated_at: ws.updated_at.to_rfc3339(),
        }
    }
}

/// Database row for RepositoryGroup
#[derive(Debug, FromRow)]
pub struct RepositoryGroupRow {
    pub id: String,
    pub name: Option<String>,
    pub workspace_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl RepositoryGroupRow {
    pub fn into_repository_group(self, repository_ids: Vec<String>) -> RepositoryGroup {
        RepositoryGroup {
            id: self.id,
            name: self.name,
            workspace_id: self.workspace_id,
            repository_ids,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

impl From<&RepositoryGroup> for RepositoryGroupRow {
    fn from(rg: &RepositoryGroup) -> Self {
        Self {
            id: rg.id.clone(),
            name: rg.name.clone(),
            workspace_id: rg.workspace_id.clone(),
            created_at: rg.created_at.to_rfc3339(),
            updated_at: rg.updated_at.to_rfc3339(),
        }
    }
}

/// Database row for Repository
#[derive(Debug, FromRow)]
pub struct RepositoryRow {
    pub id: String,
    pub vcs_type: String,
    pub vcs_provider_type: String,
    pub remote_url: String,
    pub name: String,
    pub local_path: String,
    pub default_branch: String,
    pub created_at: String,
}

impl From<RepositoryRow> for Repository {
    fn from(row: RepositoryRow) -> Self {
        let vcs_provider_type = match row.vcs_provider_type.as_str() {
            "github" => VCSProviderType::GitHub,
            "gitlab" => VCSProviderType::GitLab,
            "bitbucket" => VCSProviderType::Bitbucket,
            _ => VCSProviderType::GitHub,
        };

        Repository {
            id: row.id,
            vcs_type: VCSType::Git,
            vcs_provider_type,
            remote_url: row.remote_url,
            name: row.name,
            local_path: row.local_path,
            default_branch: row.default_branch,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

impl From<&Repository> for RepositoryRow {
    fn from(repo: &Repository) -> Self {
        let vcs_provider_type = match repo.vcs_provider_type {
            VCSProviderType::GitHub => "github",
            VCSProviderType::GitLab => "gitlab",
            VCSProviderType::Bitbucket => "bitbucket",
        };

        Self {
            id: repo.id.clone(),
            vcs_type: "git".to_string(),
            vcs_provider_type: vcs_provider_type.to_string(),
            remote_url: repo.remote_url.clone(),
            name: repo.name.clone(),
            local_path: repo.local_path.clone(),
            default_branch: repo.default_branch.clone(),
            created_at: repo.created_at.to_rfc3339(),
        }
    }
}

/// Database row for UnitTask
#[derive(Debug, FromRow)]
pub struct UnitTaskRow {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub agent_task_id: String,
    pub branch_name: Option<String>,
    pub linked_pr_url: Option<String>,
    pub base_commit: Option<String>,
    pub end_commit: Option<String>,
    pub status: String,
    pub repository_group_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_execution_failed: i32,
}

impl UnitTaskRow {
    pub fn into_unit_task(
        self,
        auto_fix_task_ids: Vec<String>,
        composite_task_id: Option<String>,
    ) -> UnitTask {
        let status = match self.status.as_str() {
            "in_progress" => UnitTaskStatus::InProgress,
            "in_review" => UnitTaskStatus::InReview,
            "approved" => UnitTaskStatus::Approved,
            "pr_open" => UnitTaskStatus::PrOpen,
            "done" => UnitTaskStatus::Done,
            "rejected" => UnitTaskStatus::Rejected,
            _ => UnitTaskStatus::InProgress,
        };

        UnitTask {
            id: self.id,
            title: self.title,
            prompt: self.prompt,
            agent_task_id: self.agent_task_id,
            branch_name: self.branch_name,
            linked_pr_url: self.linked_pr_url,
            base_commit: self.base_commit,
            end_commit: self.end_commit,
            auto_fix_task_ids,
            status,
            repository_group_id: self.repository_group_id,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            is_executing: None,
            composite_task_id,
            last_execution_failed: self.last_execution_failed != 0,
        }
    }
}

/// Database row for CompositeTask
#[derive(Debug, FromRow)]
pub struct CompositeTaskRow {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub planning_task_id: String,
    pub status: String,
    pub repository_group_id: String,
    pub plan_file_path: Option<String>,
    pub plan_yaml_content: Option<String>,
    pub execution_agent_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl CompositeTaskRow {
    pub fn into_composite_task(self, nodes: Vec<CompositeTaskNode>) -> CompositeTask {
        let status = match self.status.as_str() {
            "planning" => CompositeTaskStatus::Planning,
            "pending_approval" => CompositeTaskStatus::PendingApproval,
            "in_progress" => CompositeTaskStatus::InProgress,
            "done" => CompositeTaskStatus::Done,
            "rejected" => CompositeTaskStatus::Rejected,
            _ => CompositeTaskStatus::Planning,
        };

        let execution_agent_type = self
            .execution_agent_type
            .as_deref()
            .and_then(ai_agent_type_from_string);

        CompositeTask {
            id: self.id,
            title: self.title,
            prompt: self.prompt,
            planning_task_id: self.planning_task_id,
            nodes,
            status,
            repository_group_id: self.repository_group_id,
            plan_file_path: self.plan_file_path,
            plan_yaml_content: self.plan_yaml_content,
            execution_agent_type,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

/// Database row for AgentTask
#[derive(Debug, FromRow)]
pub struct AgentTaskRow {
    pub id: String,
    pub ai_agent_type: Option<String>,
    pub ai_agent_model: Option<String>,
    pub created_at: String,
}

/// Database row for AgentSession
#[derive(Debug, FromRow)]
pub struct AgentSessionRow {
    pub id: String,
    pub agent_task_id: String,
    pub ai_agent_type: String,
    pub ai_agent_model: Option<String>,
    pub created_at: String,
}

impl From<AgentSessionRow> for AgentSession {
    fn from(row: AgentSessionRow) -> Self {
        let ai_agent_type = match row.ai_agent_type.as_str() {
            "claude_code" => AIAgentType::ClaudeCode,
            "open_code" => AIAgentType::OpenCode,
            _ => AIAgentType::ClaudeCode,
        };

        AgentSession {
            id: row.id,
            ai_agent_type,
            ai_agent_model: row.ai_agent_model,
        }
    }
}

/// Database row for BaseRemote
#[derive(Debug, FromRow)]
pub struct BaseRemoteRow {
    pub id: i64,
    pub agent_task_id: String,
    pub git_remote_dir_path: String,
    pub git_branch_name: String,
}

impl From<BaseRemoteRow> for BaseRemote {
    fn from(row: BaseRemoteRow) -> Self {
        BaseRemote {
            git_remote_dir_path: row.git_remote_dir_path,
            git_branch_name: row.git_branch_name,
        }
    }
}

/// Database row for CompositeTaskNode
#[derive(Debug, FromRow)]
pub struct CompositeTaskNodeRow {
    pub id: String,
    pub composite_task_id: String,
    pub unit_task_id: String,
}

/// Database row for TodoItem
#[derive(Debug, FromRow)]
pub struct TodoItemRow {
    pub id: String,
    pub item_type: String,
    pub source: String,
    pub status: String,
    pub repository_id: String,
    pub created_at: String,
    pub issue_url: Option<String>,
    pub issue_title: Option<String>,
    pub suggested_labels: Option<String>,
    pub suggested_assignees: Option<String>,
    pub pr_url: Option<String>,
    pub pr_title: Option<String>,
    pub changed_files_count: Option<i32>,
    pub ai_summary: Option<String>,
}

impl From<TodoItemRow> for TodoItem {
    fn from(row: TodoItemRow) -> Self {
        let source = match row.source.as_str() {
            "auto" => TodoItemSource::Auto,
            "manual" => TodoItemSource::Manual,
            _ => TodoItemSource::Auto,
        };

        let status = match row.status.as_str() {
            "pending" => TodoItemStatus::Pending,
            "in_progress" => TodoItemStatus::InProgress,
            "done" => TodoItemStatus::Done,
            "dismissed" => TodoItemStatus::Dismissed,
            _ => TodoItemStatus::Pending,
        };

        let base = TodoItemBase {
            id: row.id,
            source,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            status,
        };

        match row.item_type.as_str() {
            "issue_triage" => TodoItem::IssueTriage(IssueTriageTodoItem {
                base,
                issue_url: row.issue_url.unwrap_or_default(),
                repository_id: row.repository_id,
                issue_title: row.issue_title.unwrap_or_default(),
                suggested_labels: row
                    .suggested_labels
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                suggested_assignees: row
                    .suggested_assignees
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            }),
            "pr_review" => TodoItem::PrReview(PrReviewTodoItem {
                base,
                pr_url: row.pr_url.unwrap_or_default(),
                repository_id: row.repository_id,
                pr_title: row.pr_title.unwrap_or_default(),
                changed_files_count: row.changed_files_count.unwrap_or(0) as u32,
                ai_summary: row.ai_summary,
            }),
            _ => TodoItem::IssueTriage(IssueTriageTodoItem {
                base,
                issue_url: String::new(),
                repository_id: row.repository_id,
                issue_title: String::new(),
                suggested_labels: Vec::new(),
                suggested_assignees: Vec::new(),
            }),
        }
    }
}

/// Helper to convert status enum to string
pub fn unit_task_status_to_string(status: UnitTaskStatus) -> &'static str {
    match status {
        UnitTaskStatus::InProgress => "in_progress",
        UnitTaskStatus::InReview => "in_review",
        UnitTaskStatus::Approved => "approved",
        UnitTaskStatus::PrOpen => "pr_open",
        UnitTaskStatus::Done => "done",
        UnitTaskStatus::Rejected => "rejected",
    }
}

pub fn composite_task_status_to_string(status: CompositeTaskStatus) -> &'static str {
    match status {
        CompositeTaskStatus::Planning => "planning",
        CompositeTaskStatus::PendingApproval => "pending_approval",
        CompositeTaskStatus::InProgress => "in_progress",
        CompositeTaskStatus::Done => "done",
        CompositeTaskStatus::Rejected => "rejected",
    }
}

pub fn ai_agent_type_to_string(agent_type: AIAgentType) -> &'static str {
    match agent_type {
        AIAgentType::ClaudeCode => "claude_code",
        AIAgentType::OpenCode => "open_code",
        AIAgentType::GeminiCli => "gemini_cli",
        AIAgentType::CodexCli => "codex_cli",
        AIAgentType::Aider => "aider",
        AIAgentType::Amp => "amp",
    }
}

pub fn ai_agent_type_from_string(s: &str) -> Option<AIAgentType> {
    match s {
        "claude_code" => Some(AIAgentType::ClaudeCode),
        "open_code" => Some(AIAgentType::OpenCode),
        "gemini_cli" => Some(AIAgentType::GeminiCli),
        "codex_cli" => Some(AIAgentType::CodexCli),
        "aider" => Some(AIAgentType::Aider),
        "amp" => Some(AIAgentType::Amp),
        _ => None,
    }
}

pub fn vcs_provider_type_to_string(provider: VCSProviderType) -> &'static str {
    match provider {
        VCSProviderType::GitHub => "github",
        VCSProviderType::GitLab => "gitlab",
        VCSProviderType::Bitbucket => "bitbucket",
    }
}

pub fn todo_item_source_to_string(source: TodoItemSource) -> &'static str {
    match source {
        TodoItemSource::Auto => "auto",
        TodoItemSource::Manual => "manual",
    }
}

pub fn todo_item_status_to_string(status: TodoItemStatus) -> &'static str {
    match status {
        TodoItemStatus::Pending => "pending",
        TodoItemStatus::InProgress => "in_progress",
        TodoItemStatus::Done => "done",
        TodoItemStatus::Dismissed => "dismissed",
    }
}

/// Database row for TokenUsage
#[derive(Debug, FromRow)]
pub struct TokenUsageRow {
    pub id: String,
    pub session_id: String,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<f64>,
    pub duration_api_ms: Option<f64>,
    pub num_turns: Option<i64>,
    pub is_error: i32,
    pub created_at: String,
}

impl From<TokenUsageRow> for TokenUsage {
    fn from(row: TokenUsageRow) -> Self {
        TokenUsage {
            id: row.id,
            session_id: row.session_id,
            cost_usd: row.cost_usd,
            duration_ms: row.duration_ms,
            duration_api_ms: row.duration_api_ms,
            num_turns: row.num_turns.map(|n| n as u32),
            is_error: row.is_error != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

impl From<&TokenUsage> for TokenUsageRow {
    fn from(usage: &TokenUsage) -> Self {
        Self {
            id: usage.id.clone(),
            session_id: usage.session_id.clone(),
            cost_usd: usage.cost_usd,
            duration_ms: usage.duration_ms,
            duration_api_ms: usage.duration_api_ms,
            num_turns: usage.num_turns.map(|n| n as i64),
            is_error: if usage.is_error { 1 } else { 0 },
            created_at: usage.created_at.to_rfc3339(),
        }
    }
}
