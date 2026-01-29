mod schema;

use std::path::Path;

pub use schema::*;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Entity not found: {0}")]
    NotFound(String),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Database connection pool
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Creates a new database connection
    pub async fn new(db_path: &Path) -> DatabaseResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Returns a reference to the connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Runs database migrations
    async fn run_migrations(&self) -> DatabaseResult<()> {
        sqlx::query(SCHEMA_SQL)
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::Migration(e.to_string()))?;

        // Run incremental migrations for existing databases
        self.run_incremental_migrations().await?;

        Ok(())
    }

    /// Runs incremental migrations for existing databases
    async fn run_incremental_migrations(&self) -> DatabaseResult<()> {
        // Migration: Add execution_agent_type column to composite_tasks if it doesn't
        // exist Check if column exists by trying to add it (SQLite will error
        // if it exists)
        let _ = sqlx::query("ALTER TABLE composite_tasks ADD COLUMN execution_agent_type TEXT")
            .execute(&self.pool)
            .await;
        // Ignore the error if column already exists

        // Migration: Add plan_yaml_content column to composite_tasks if it doesn't
        // exist
        let _ = sqlx::query("ALTER TABLE composite_tasks ADD COLUMN plan_yaml_content TEXT")
            .execute(&self.pool)
            .await;
        // Ignore the error if column already exists

        // Migration: Add last_execution_failed column to unit_tasks if it doesn't
        // exist
        let _ = sqlx::query(
            "ALTER TABLE unit_tasks ADD COLUMN last_execution_failed INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&self.pool)
        .await;
        // Ignore the error if column already exists

        // Migration: Migrate repository_id to repository_group_id for workspaces
        // support
        self.migrate_to_repository_groups().await?;

        Ok(())
    }

    /// Migrates existing repository_id references to repository_group_id
    async fn migrate_to_repository_groups(&self) -> DatabaseResult<()> {
        // Check if migration is needed by checking if old column exists
        let has_old_column: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('unit_tasks') WHERE name = 'repository_id'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if !has_old_column {
            // Already migrated or fresh install
            return Ok(());
        }

        tracing::info!("Starting migration from repository_id to repository_group_id");

        let now = chrono::Utc::now().to_rfc3339();

        // 1. Create default workspace if none exists
        let workspace_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let default_workspace_id = if workspace_count == 0 {
            let workspace_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO workspaces (id, name, description, created_at, updated_at) VALUES \
                 (?, 'Default', 'Default workspace', ?, ?)",
            )
            .bind(&workspace_id)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            tracing::info!("Created default workspace: {}", workspace_id);
            workspace_id
        } else {
            // Get existing default workspace
            sqlx::query_scalar("SELECT id FROM workspaces LIMIT 1")
                .fetch_one(&self.pool)
                .await?
        };

        // 2. Register all existing repositories to the default workspace
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO workspace_repositories (workspace_id, repository_id) SELECT ?, \
             id FROM repositories",
        )
        .bind(&default_workspace_id)
        .execute(&self.pool)
        .await;

        // 3. Create single-repo groups for repositories used in unit_tasks
        let unit_task_repos: Vec<(String, String)> = sqlx::query_as(
            "SELECT DISTINCT ut.id, ut.repository_id FROM unit_tasks ut WHERE NOT EXISTS (SELECT \
             1 FROM repository_groups rg JOIN repository_group_members rgm ON rg.id = \
             rgm.repository_group_id WHERE rgm.repository_id = ut.repository_id)",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for (task_id, repo_id) in &unit_task_repos {
            let group_id = uuid::Uuid::new_v4().to_string();

            // Create single-repo group (name is NULL for single-repo groups)
            sqlx::query(
                "INSERT INTO repository_groups (id, name, workspace_id, created_at, updated_at) \
                 VALUES (?, NULL, ?, ?, ?)",
            )
            .bind(&group_id)
            .bind(&default_workspace_id)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            // Add repository to group
            sqlx::query(
                "INSERT INTO repository_group_members (repository_group_id, repository_id) VALUES \
                 (?, ?)",
            )
            .bind(&group_id)
            .bind(repo_id)
            .execute(&self.pool)
            .await?;

            // Update the task to use repository_group_id
            // Note: We need to handle this differently - add column first, then migrate
            tracing::info!(
                "Created single-repo group {} for task {} with repo {}",
                group_id,
                task_id,
                repo_id
            );
        }

        // 4. Add repository_group_id column to unit_tasks if it doesn't exist
        let _ = sqlx::query("ALTER TABLE unit_tasks ADD COLUMN repository_group_id TEXT")
            .execute(&self.pool)
            .await;

        // 5. Update unit_tasks to set repository_group_id from repository_id
        sqlx::query(
            "UPDATE unit_tasks SET repository_group_id = ( SELECT rg.id FROM repository_groups rg \
             JOIN repository_group_members rgm ON rg.id = rgm.repository_group_id WHERE \
             rgm.repository_id = unit_tasks.repository_id LIMIT 1 ) WHERE repository_group_id IS \
             NULL",
        )
        .execute(&self.pool)
        .await?;

        // 6. Same for composite_tasks
        let _ = sqlx::query("ALTER TABLE composite_tasks ADD COLUMN repository_group_id TEXT")
            .execute(&self.pool)
            .await;

        // Create single-repo groups for composite_tasks
        let composite_task_repos: Vec<(String, String)> = sqlx::query_as(
            "SELECT DISTINCT ct.id, ct.repository_id FROM composite_tasks ct WHERE NOT EXISTS \
             (SELECT 1 FROM repository_groups rg JOIN repository_group_members rgm ON rg.id = \
             rgm.repository_group_id WHERE rgm.repository_id = ct.repository_id)",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for (_task_id, repo_id) in &composite_task_repos {
            let group_id = uuid::Uuid::new_v4().to_string();

            sqlx::query(
                "INSERT INTO repository_groups (id, name, workspace_id, created_at, updated_at) \
                 VALUES (?, NULL, ?, ?, ?)",
            )
            .bind(&group_id)
            .bind(&default_workspace_id)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                "INSERT INTO repository_group_members (repository_group_id, repository_id) VALUES \
                 (?, ?)",
            )
            .bind(&group_id)
            .bind(repo_id)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            "UPDATE composite_tasks SET repository_group_id = ( SELECT rg.id FROM \
             repository_groups rg JOIN repository_group_members rgm ON rg.id = \
             rgm.repository_group_id WHERE rgm.repository_id = composite_tasks.repository_id \
             LIMIT 1 ) WHERE repository_group_id IS NULL",
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Migration to repository_group_id completed");

        Ok(())
    }
}

/// SQL schema definition
const SCHEMA_SQL: &str = r#"
-- Repositories table
CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY NOT NULL,
    vcs_type TEXT NOT NULL DEFAULT 'git',
    vcs_provider_type TEXT NOT NULL,
    remote_url TEXT NOT NULL,
    name TEXT NOT NULL,
    local_path TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    created_at TEXT NOT NULL
);

-- Workspaces table
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Repository groups table
CREATE TABLE IF NOT EXISTS repository_groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT,  -- NULL if single repository (display repo name instead)
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Repository group members (M:N relationship)
CREATE TABLE IF NOT EXISTS repository_group_members (
    repository_group_id TEXT NOT NULL REFERENCES repository_groups(id) ON DELETE CASCADE,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    PRIMARY KEY (repository_group_id, repository_id)
);

-- Workspace repositories (1:N relationship)
CREATE TABLE IF NOT EXISTS workspace_repositories (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, repository_id)
);

-- Agent tasks table
CREATE TABLE IF NOT EXISTS agent_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    ai_agent_type TEXT,
    ai_agent_model TEXT,
    created_at TEXT NOT NULL
);

-- Agent task base remotes (one-to-many)
CREATE TABLE IF NOT EXISTS agent_task_remotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    git_remote_dir_path TEXT NOT NULL,
    git_branch_name TEXT NOT NULL
);

-- Agent sessions table
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    agent_task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    ai_agent_type TEXT NOT NULL,
    ai_agent_model TEXT,
    created_at TEXT NOT NULL
);

-- Unit tasks table
CREATE TABLE IF NOT EXISTS unit_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    prompt TEXT NOT NULL,
    agent_task_id TEXT NOT NULL REFERENCES agent_tasks(id),
    branch_name TEXT,
    linked_pr_url TEXT,
    base_commit TEXT,
    end_commit TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress',
    repository_group_id TEXT NOT NULL REFERENCES repository_groups(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_execution_failed INTEGER NOT NULL DEFAULT 0
);

-- Unit task auto-fix tasks (one-to-many)
CREATE TABLE IF NOT EXISTS unit_task_auto_fixes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    unit_task_id TEXT NOT NULL REFERENCES unit_tasks(id) ON DELETE CASCADE,
    agent_task_id TEXT NOT NULL REFERENCES agent_tasks(id)
);

-- Composite tasks table
CREATE TABLE IF NOT EXISTS composite_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    prompt TEXT NOT NULL,
    planning_task_id TEXT NOT NULL REFERENCES agent_tasks(id),
    status TEXT NOT NULL DEFAULT 'planning',
    repository_group_id TEXT NOT NULL REFERENCES repository_groups(id),
    plan_file_path TEXT,
    plan_yaml_content TEXT,
    execution_agent_type TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Composite task nodes table
CREATE TABLE IF NOT EXISTS composite_task_nodes (
    id TEXT NOT NULL,
    composite_task_id TEXT NOT NULL REFERENCES composite_tasks(id) ON DELETE CASCADE,
    unit_task_id TEXT NOT NULL REFERENCES unit_tasks(id) ON DELETE CASCADE,
    PRIMARY KEY (composite_task_id, id)
);

-- Composite task node dependencies (many-to-many)
CREATE TABLE IF NOT EXISTS composite_task_node_deps (
    composite_task_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    PRIMARY KEY (composite_task_id, node_id, depends_on_id),
    FOREIGN KEY (composite_task_id, node_id) REFERENCES composite_task_nodes(composite_task_id, id) ON DELETE CASCADE
);

-- Todo items table (polymorphic)
CREATE TABLE IF NOT EXISTS todo_items (
    id TEXT PRIMARY KEY NOT NULL,
    item_type TEXT NOT NULL,
    source TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    repository_id TEXT NOT NULL REFERENCES repositories(id),
    created_at TEXT NOT NULL,
    -- issue_triage fields
    issue_url TEXT,
    issue_title TEXT,
    suggested_labels TEXT, -- JSON array
    suggested_assignees TEXT, -- JSON array
    -- pr_review fields
    pr_url TEXT,
    pr_title TEXT,
    changed_files_count INTEGER,
    ai_summary TEXT
);

-- Execution logs table
CREATE TABLE IF NOT EXISTS execution_logs (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    message TEXT NOT NULL
);

-- Agent stream messages table
CREATE TABLE IF NOT EXISTS agent_stream_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    message_json TEXT NOT NULL
);

-- Session usage table for tracking token usage per session
CREATE TABLE IF NOT EXISTS session_usage (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL,
    model TEXT,
    created_at TEXT NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_unit_tasks_repository_group ON unit_tasks(repository_group_id);
CREATE INDEX IF NOT EXISTS idx_unit_tasks_status ON unit_tasks(status);
CREATE INDEX IF NOT EXISTS idx_composite_tasks_repository_group ON composite_tasks(repository_group_id);
CREATE INDEX IF NOT EXISTS idx_composite_tasks_status ON composite_tasks(status);
CREATE INDEX IF NOT EXISTS idx_todo_items_repository ON todo_items(repository_id);
CREATE INDEX IF NOT EXISTS idx_todo_items_status ON todo_items(status);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_task ON agent_sessions(agent_task_id);
CREATE INDEX IF NOT EXISTS idx_execution_logs_session ON execution_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_stream_messages_session ON agent_stream_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_repository_groups_workspace ON repository_groups(workspace_id);
CREATE INDEX IF NOT EXISTS idx_repository_group_members_group ON repository_group_members(repository_group_id);
CREATE INDEX IF NOT EXISTS idx_workspace_repositories_workspace ON workspace_repositories(workspace_id);
CREATE INDEX IF NOT EXISTS idx_session_usage_session ON session_usage(session_id);
"#;
