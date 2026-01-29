use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rand::Rng;
use thiserror::Error;

use super::{
    make_branch_name_unique, AgentExecutionService, GitService, RepositoryGroupService,
    RepositoryService, TaskService,
};
use crate::entities::{
    AgentTask, BaseRemote, CompositeTask, CompositeTaskNode, CompositeTaskStatus,
    PlanValidationError, PlanYaml, UnitTask,
};

#[derive(Error, Debug)]
pub enum PlanningError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    #[error("Composite task not found: {0}")]
    CompositeTaskNotFound(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Plan file not found: {0}")]
    PlanFileNotFound(String),
    #[error("Plan parsing error: {0}")]
    PlanParsing(String),
    #[error("Plan validation error: {0}")]
    PlanValidation(#[from] PlanValidationError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Task is not in planning state")]
    InvalidState,
    #[error("Invalid plan filename: {0}")]
    InvalidFilename(String),
}

pub type PlanningResult<T> = Result<T, PlanningError>;

/// Service for managing CompositeTask planning
pub struct CompositePlanningService {
    task_service: Arc<TaskService>,
    repository_service: Arc<RepositoryService>,
    repository_group_service: Arc<RepositoryGroupService>,
    git_service: Arc<GitService>,
}

impl CompositePlanningService {
    pub fn new(
        task_service: Arc<TaskService>,
        repository_service: Arc<RepositoryService>,
        repository_group_service: Arc<RepositoryGroupService>,
        git_service: Arc<GitService>,
    ) -> Self {
        Self {
            task_service,
            repository_service,
            repository_group_service,
            git_service,
        }
    }

    /// Gets the primary repository from a repository group.
    async fn get_primary_repository_from_group(
        &self,
        repository_group_id: &str,
    ) -> PlanningResult<crate::entities::Repository> {
        let group = self
            .repository_group_service
            .get(repository_group_id)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?
            .ok_or_else(|| {
                PlanningError::RepositoryNotFound(format!(
                    "Repository group not found: {}",
                    repository_group_id
                ))
            })?;

        if group.repository_ids.is_empty() {
            return Err(PlanningError::RepositoryNotFound(format!(
                "Repository group {} has no repositories",
                repository_group_id
            )));
        }

        let repo_id = &group.repository_ids[0];
        self.repository_service
            .get(repo_id)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?
            .ok_or_else(|| PlanningError::RepositoryNotFound(repo_id.clone()))
    }

    /// Validates that the plan filename does not contain path traversal
    /// sequences
    fn validate_plan_filename(filename: &str) -> PlanningResult<()> {
        // Check for path separators or traversal sequences
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(PlanningError::InvalidFilename(format!(
                "Plan filename contains invalid characters: {}",
                filename
            )));
        }
        Ok(())
    }

    /// Generates a random string for the plan filename
    fn generate_random_string() -> String {
        let mut rng = rand::thread_rng();
        let chars: Vec<char> = (0..16)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();
        chars.into_iter().collect()
    }

    /// Generates the planning prompt for the AI agent
    pub fn generate_planning_prompt(user_prompt: &str, plan_filename: &str) -> String {
        format!(
            r#"You are an expert software architect. Your task is to decompose a complex development request into logical, cohesive tasks that you can complete as separate pull requests.

**IMPORTANT: For simple, straightforward tasks that can be completed in a single PR, create a single-node graph with no dependencies. Not every task needs to be broken down into multiple steps.**

## Instructions
1. Analyze the user's request and break it down into smaller, independent tasks.
2. Create a YAML plan file at the repository root with the exact filename: `{plan_filename}`
3. Each task should have:
   - A unique `id` (lowercase, hyphen-separated, e.g., "setup-database", "implement-api")
   - An optional `title` providing a human-readable name for the task (defaults to `id` if not specified)
   - A clear `prompt` describing what the AI agent should do for this task
   - An optional `branchName` specifying a custom git branch name for this task
   - Optional `dependsOn` array listing task IDs that must complete before this task starts

## Plan File Format
The plan file MUST be valid YAML with the following structure:

```yaml
tasks:
  - id: "task-id-1"
    title: "Human-readable Task Title"
    prompt: "Description of what the AI agent should do for this task"
    branchName: "feature/custom-branch"

  - id: "task-id-2"
    title: "Second Task Title"
    prompt: "Description of the second task"
    dependsOn: ["task-id-1"]

  - id: "task-id-3"
    prompt: "Description of the third task"
    dependsOn: ["task-id-1", "task-id-2"]
```

## Important Guidelines
- Tasks with no dependencies will execute in parallel
- Tasks should be granular enough to be completed in a single AI agent session
- Use descriptive IDs that indicate what the task does
- Use `title` for more descriptive, human-readable names (with spaces, capitalization, etc.)
- Use `branchName` when you want a specific git branch name instead of the auto-generated one
- Each prompt should be self-contained and provide enough context for the task
- Include any necessary context about the codebase or requirements in each task prompt
- The plan should enable maximum parallelization while respecting true dependencies

## Context Requirements
- **Codebase Analysis**: Analyze the existing codebase to understand architecture, patterns, and conventions
- **Context Completeness**: Each task prompt should be self-contained with all necessary context so the AI agent understands what to work on
- **Web Search**: Only use web search when absolutely necessary (e.g., for new technologies not in the codebase, or when explicitly requested by the user). Avoid excessive web searches - focus on the codebase first.

## Execution Checklist
1. **Determine if this is a simple task**: If it can be completed in one PR, create a single-node graph
2. Analyze the existing codebase to understand the context
3. Structure tasks for maximum parallelism (if breaking down into multiple tasks)
4. Write comprehensive task prompts with all necessary context from the codebase
5. Ensure each task is self-contained with complete context

Generate a streamlined task breakdown. For simple tasks, create a single node with no dependencies. For complex tasks, use fewer, more comprehensive tasks that represent meaningful progress toward the goal.

## Output
Write the plan file `{plan_filename}` to the repository root. Do not include any other files or explanations - just create the plan file.

## Original Request
{user_prompt}"#,
            plan_filename = plan_filename,
            user_prompt = user_prompt
        )
    }

    /// Starts the planning phase for a composite task
    ///
    /// This method creates a temporary git worktree for the planning agent to
    /// work in, ensuring the main repository is not affected during
    /// planning.
    pub async fn start_planning(
        &self,
        composite_task: &CompositeTask,
        execution_service: &AgentExecutionService,
    ) -> PlanningResult<String> {
        // Verify task is in Planning state
        if composite_task.status != CompositeTaskStatus::Planning {
            return Err(PlanningError::InvalidState);
        }

        // Get repository from repository group
        let repo = self
            .get_primary_repository_from_group(&composite_task.repository_group_id)
            .await?;

        // Generate random string for plan filename
        let random_string = Self::generate_random_string();
        let plan_filename = format!("PLAN-{}.yaml", random_string);

        // Store the plan file path in the composite task
        self.task_service
            .update_composite_task_plan_path(&composite_task.id, &plan_filename)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?;

        // Generate the planning prompt
        let planning_prompt =
            Self::generate_planning_prompt(&composite_task.prompt, &plan_filename);

        // Get the planning agent task
        let agent_task = self
            .task_service
            .get_agent_task(&composite_task.planning_task_id)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?
            .ok_or_else(|| PlanningError::Database("Planning agent task not found".to_string()))?;

        let repo_path = PathBuf::from(&repo.local_path);

        // Create a temporary worktree for planning
        // Resolve symlinks in /tmp (macOS uses /tmp -> /private/tmp)
        let base_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let worktree_path = base_tmp.join(format!("delidev/planning/{}", composite_task.id));
        let planning_branch_name = format!("delidev/planning/{}", composite_task.id);

        tracing::info!(
            "Creating planning worktree at {:?} for composite task {}",
            worktree_path,
            composite_task.id
        );

        // Remove existing worktree if it exists (e.g., from a previous failed run)
        if worktree_path.exists() {
            tracing::info!("Removing existing planning worktree from previous run");
            if let Err(e) = self.git_service.remove_worktree(&repo_path, &worktree_path) {
                tracing::warn!("Failed to remove existing planning worktree: {}", e);
            }
        }

        // Create the planning worktree
        self.git_service
            .create_worktree(
                &repo_path,
                &worktree_path,
                &planning_branch_name,
                &repo.default_branch,
            )
            .map_err(|e| {
                PlanningError::Execution(format!("Failed to create planning worktree: {}", e))
            })?;

        // Execute the planning agent task in the worktree
        let result = execution_service
            .execute_agent_task(
                &agent_task,
                &worktree_path,
                &planning_prompt,
                &composite_task.id,
            )
            .await;

        // Handle execution result
        match result {
            Ok(exec_result) => {
                if !exec_result.success {
                    // Clean up worktree on failure
                    self.cleanup_planning_worktree(
                        &repo_path,
                        &worktree_path,
                        &planning_branch_name,
                    );
                    return Err(PlanningError::Execution(
                        exec_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    ));
                }
            }
            Err(e) => {
                // Clean up worktree on error
                self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
                return Err(PlanningError::Execution(e.to_string()));
            }
        }

        // Copy the plan file from worktree to repo before cleaning up
        let worktree_plan_path = worktree_path.join(&plan_filename);
        let repo_plan_path = repo_path.join(&plan_filename);

        if worktree_plan_path.exists() {
            std::fs::copy(&worktree_plan_path, &repo_plan_path).map_err(|e| {
                self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
                PlanningError::Io(e)
            })?;
            tracing::info!(
                "Copied plan file from worktree to repo: {:?}",
                repo_plan_path
            );
        } else {
            // Clean up worktree if plan file was not created
            self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
            return Err(PlanningError::PlanFileNotFound(format!(
                "Plan file '{}' was not created by the planning agent",
                plan_filename
            )));
        }

        // Clean up the planning worktree
        self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);

        Ok(plan_filename)
    }

    /// Cleans up a planning worktree and its associated branch
    fn cleanup_planning_worktree(&self, repo_path: &Path, worktree_path: &Path, branch_name: &str) {
        // First remove the worktree
        if worktree_path.exists() {
            if let Err(e) = self.git_service.remove_worktree(repo_path, worktree_path) {
                tracing::warn!("Failed to cleanup planning worktree: {}", e);
            } else {
                tracing::info!("Cleaned up planning worktree: {:?}", worktree_path);
            }
        }

        // Then delete the planning branch
        if let Err(e) = self.git_service.delete_branch(repo_path, branch_name) {
            tracing::warn!("Failed to delete planning branch '{}': {}", branch_name, e);
        } else {
            tracing::info!("Deleted planning branch: {}", branch_name);
        }
    }

    /// Reads the plan file content from the filesystem.
    /// This is a helper method to avoid code duplication between read_plan and
    /// get_plan_content.
    async fn read_plan_file_content(
        &self,
        composite_task: &CompositeTask,
    ) -> PlanningResult<String> {
        let repo = self
            .get_primary_repository_from_group(&composite_task.repository_group_id)
            .await?;

        let plan_filename = composite_task
            .plan_file_path
            .as_ref()
            .ok_or_else(|| PlanningError::PlanFileNotFound("Plan file path not set".to_string()))?;

        // Validate the filename to prevent path traversal attacks
        Self::validate_plan_filename(plan_filename)?;

        let repo_path = PathBuf::from(&repo.local_path);
        let plan_file_path = repo_path.join(plan_filename);

        // Read the plan file
        std::fs::read_to_string(&plan_file_path).map_err(|e| {
            PlanningError::PlanFileNotFound(format!(
                "Failed to read plan file '{}': {}",
                plan_file_path.display(),
                e
            ))
        })
    }

    /// Stores the plan content in the database and returns the content.
    /// Logs a warning if storage fails but still returns the content.
    async fn store_plan_content_if_needed(
        &self,
        composite_task_id: &str,
        content: &str,
    ) -> PlanningResult<()> {
        self.task_service
            .update_composite_task_plan_content(composite_task_id, content)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))
    }

    /// Deletes the plan file from the repository if it exists.
    /// This is a best-effort cleanup operation - deletion failures are logged
    /// but not propagated as errors since the primary operation (persistence)
    /// has already succeeded.
    async fn delete_plan_file(&self, composite_task: &CompositeTask) {
        let plan_filename = match &composite_task.plan_file_path {
            Some(path) => path,
            None => {
                tracing::debug!("No plan file path set, nothing to delete");
                return;
            }
        };

        // Validate the filename to prevent path traversal attacks
        if let Err(e) = Self::validate_plan_filename(plan_filename) {
            tracing::warn!("Invalid plan filename, skipping deletion: {}", e);
            return;
        }

        let repo = match self
            .get_primary_repository_from_group(&composite_task.repository_group_id)
            .await
        {
            Ok(repo) => repo,
            Err(e) => {
                tracing::warn!("Repository not found for plan file deletion: {}", e);
                return;
            }
        };

        let repo_path = PathBuf::from(&repo.local_path);
        let plan_file_path = repo_path.join(plan_filename);

        if !plan_file_path.exists() {
            tracing::debug!(
                "Plan file '{}' does not exist, already deleted or never created",
                plan_file_path.display()
            );
            return;
        }

        if let Err(e) = std::fs::remove_file(&plan_file_path) {
            tracing::warn!(
                "Failed to delete plan file '{}' after persisting: {}",
                plan_file_path.display(),
                e
            );
        } else {
            tracing::info!(
                "Deleted plan file '{}' after persisting to database",
                plan_file_path.display()
            );
        }
    }

    /// Persists plan content to database and deletes the file from filesystem.
    /// This is a helper to ensure consistent behavior across methods.
    /// Deletion is best-effort - errors are logged but not propagated.
    async fn persist_and_cleanup_plan_file(
        &self,
        composite_task: &CompositeTask,
        content: &str,
    ) -> PlanningResult<()> {
        // Store the plan content in the database
        self.store_plan_content_if_needed(&composite_task.id, content)
            .await?;

        // Delete the plan file (best-effort, errors are logged but not propagated)
        self.delete_plan_file(composite_task).await;

        Ok(())
    }

    /// Reads and parses the plan file from the repository
    /// Also stores the content in the database for persistence and deletes the
    /// file after successful parsing and validation.
    pub async fn read_plan(&self, composite_task: &CompositeTask) -> PlanningResult<PlanYaml> {
        // Read the plan file content using the helper
        let content = self.read_plan_file_content(composite_task).await?;

        // Parse and validate the plan BEFORE persisting/deleting to avoid data loss
        let plan =
            PlanYaml::parse(&content).map_err(|e| PlanningError::PlanParsing(e.to_string()))?;
        plan.validate()?;

        // Only persist and delete the file after successful parsing and validation
        self.persist_and_cleanup_plan_file(composite_task, &content)
            .await?;

        Ok(plan)
    }

    /// Gets the plan content from the database or file.
    /// If reading from file, persists to database and deletes the file.
    pub async fn get_plan_content(&self, composite_task: &CompositeTask) -> PlanningResult<String> {
        // First check if we have the content stored in the database
        if let Some(content) = &composite_task.plan_yaml_content {
            return Ok(content.clone());
        }

        // Otherwise try to read from file using the helper
        let content = self.read_plan_file_content(composite_task).await?;

        // Persist to database and delete the file (best-effort cleanup)
        self.persist_and_cleanup_plan_file(composite_task, &content)
            .await?;

        Ok(content)
    }

    /// Gets and parses the plan from the database or file
    /// This method prioritizes the persisted YAML content in the database
    /// over reading from the file system.
    pub async fn get_plan(&self, composite_task: &CompositeTask) -> PlanningResult<PlanYaml> {
        // Get the plan content (prioritizes database over file)
        let content = self.get_plan_content(composite_task).await?;

        let plan =
            PlanYaml::parse(&content).map_err(|e| PlanningError::PlanParsing(e.to_string()))?;

        // Validate the plan
        plan.validate()?;

        Ok(plan)
    }

    /// Approves the plan and creates UnitTasks and CompositeTaskNodes
    pub async fn approve_plan(&self, composite_task: &CompositeTask) -> PlanningResult<()> {
        // Verify task is in PendingApproval state
        if composite_task.status != CompositeTaskStatus::PendingApproval {
            return Err(PlanningError::InvalidState);
        }

        // Get the plan from persisted YAML (prioritizes database over file)
        let plan = self.get_plan(composite_task).await?;

        // Get repository
        let repo = self
            .get_primary_repository_from_group(&composite_task.repository_group_id)
            .await?;

        // Create UnitTasks and AgentTasks for each plan task
        let mut node_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut nodes: Vec<CompositeTaskNode> = Vec::new();

        for plan_task in &plan.tasks {
            // Create AgentTask with execution agent type from composite task
            let agent_task_id = uuid::Uuid::new_v4().to_string();
            let mut agent_task = AgentTask::new(
                agent_task_id.clone(),
                vec![BaseRemote {
                    git_remote_dir_path: repo.local_path.clone(),
                    git_branch_name: repo.default_branch.clone(),
                }],
            );

            // Set execution agent type if specified in the composite task
            if let Some(at) = composite_task.execution_agent_type {
                agent_task = agent_task.with_agent_type(at);
            }

            self.task_service
                .create_agent_task(&agent_task)
                .await
                .map_err(|e| PlanningError::Database(e.to_string()))?;

            // Create UnitTask
            let unit_task_id = uuid::Uuid::new_v4().to_string();
            // Use title from plan if specified, otherwise fall back to ID
            let task_title = plan_task
                .title
                .clone()
                .unwrap_or_else(|| plan_task.id.clone());
            let mut unit_task = UnitTask::new(
                unit_task_id.clone(),
                task_title,
                plan_task.prompt.clone(),
                agent_task_id,
                composite_task.repository_group_id.clone(),
            );

            // Set branch name if specified in the plan (with unique suffix)
            if let Some(branch_name) = &plan_task.branch_name {
                unit_task = unit_task.with_branch_name(make_branch_name_unique(branch_name));
            }

            let unit_task = unit_task;

            self.task_service
                .create_unit_task(&unit_task)
                .await
                .map_err(|e| PlanningError::Database(e.to_string()))?;

            // Map plan task ID to unit task ID
            node_id_map.insert(plan_task.id.clone(), unit_task_id.clone());

            // Create CompositeTaskNode (dependencies will be set after all tasks are
            // created)
            let node = CompositeTaskNode::new(plan_task.id.clone(), unit_task_id);
            nodes.push(node);
        }

        // Update nodes with dependencies (convert plan task IDs to node IDs)
        for (i, plan_task) in plan.tasks.iter().enumerate() {
            nodes[i].depends_on = plan_task.depends_on.clone();
        }

        // Add nodes to the composite task
        for node in &nodes {
            self.task_service
                .add_composite_task_node(&composite_task.id, node)
                .await
                .map_err(|e| PlanningError::Database(e.to_string()))?;
        }

        // Delete the plan file from the repository (cleanup)
        self.delete_plan_file(composite_task).await;

        // Update composite task status to InProgress
        self.task_service
            .update_composite_task_status(&composite_task.id, CompositeTaskStatus::InProgress)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?;

        Ok(())
    }

    /// Transitions a composite task from Planning to PendingApproval
    pub async fn complete_planning(&self, composite_task_id: &str) -> PlanningResult<()> {
        self.task_service
            .update_composite_task_status(composite_task_id, CompositeTaskStatus::PendingApproval)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?;

        Ok(())
    }

    /// Rejects the plan and transitions the composite task to Rejected state
    pub async fn reject_plan(&self, composite_task: &CompositeTask) -> PlanningResult<()> {
        // Delete the plan file from the repository (cleanup)
        self.delete_plan_file(composite_task).await;

        // Update status to Rejected
        self.task_service
            .update_composite_task_status(&composite_task.id, CompositeTaskStatus::Rejected)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?;

        Ok(())
    }

    /// Generates the prompt for updating an existing plan
    pub fn generate_update_plan_prompt(
        original_prompt: &str,
        current_plan: &str,
        update_request: &str,
        plan_filename: &str,
    ) -> String {
        format!(
            r#"You are an expert software architect. Your task is to update an existing development plan based on user feedback.

## Current Plan
The following is the current plan that was generated for this task:

```yaml
{current_plan}
```

## Original Request
{original_prompt}

## Update Request
The user has requested the following changes to the plan:
{update_request}

## Instructions
1. Review the current plan and understand its structure
2. Apply the user's requested changes to the plan
3. Ensure the updated plan maintains consistency (dependencies, task IDs, etc.)
4. Write the updated plan to the exact filename: `{plan_filename}`

## Plan File Format
The plan file MUST be valid YAML with the following structure:

```yaml
tasks:
  - id: "task-id-1"
    title: "Human-readable Task Title"
    prompt: "Description of what the AI agent should do for this task"
    branchName: "feature/custom-branch"

  - id: "task-id-2"
    title: "Second Task Title"
    prompt: "Description of the second task"
    dependsOn: ["task-id-1"]
```

## Important Guidelines
- Preserve existing task IDs where possible to maintain continuity
- Update task prompts based on the user's feedback
- Add or remove tasks as requested
- Update dependencies if the task structure changes
- Ensure the plan is still valid (no circular dependencies, all referenced IDs exist)
- You MUST perform web searches if you need additional information

## Output
Write the updated plan file `{plan_filename}` to the repository root. Do not include any other files or explanations - just create the updated plan file."#,
            current_plan = current_plan,
            original_prompt = original_prompt,
            update_request = update_request,
            plan_filename = plan_filename,
        )
    }

    /// Updates an existing plan based on user feedback
    ///
    /// This method runs a new Claude session with the current plan and user's
    /// update request to generate an updated plan.
    pub async fn update_plan(
        &self,
        composite_task: &CompositeTask,
        update_request: &str,
        execution_service: &AgentExecutionService,
    ) -> PlanningResult<String> {
        // Verify task is in PendingApproval state
        if composite_task.status != CompositeTaskStatus::PendingApproval {
            return Err(PlanningError::InvalidState);
        }

        // Get the current plan content
        let current_plan = self.get_plan_content(composite_task).await?;

        // Get repository from repository group
        let repo = self
            .get_primary_repository_from_group(&composite_task.repository_group_id)
            .await?;

        // Generate new random string for plan filename
        let random_string = Self::generate_random_string();
        let plan_filename = format!("PLAN-{}.yaml", random_string);

        // Store the new plan file path in the composite task
        self.task_service
            .update_composite_task_plan_path(&composite_task.id, &plan_filename)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?;

        // Generate the update prompt
        let update_prompt = Self::generate_update_plan_prompt(
            &composite_task.prompt,
            &current_plan,
            update_request,
            &plan_filename,
        );

        // Get the planning agent task
        let agent_task = self
            .task_service
            .get_agent_task(&composite_task.planning_task_id)
            .await
            .map_err(|e| PlanningError::Database(e.to_string()))?
            .ok_or_else(|| PlanningError::Database("Planning agent task not found".to_string()))?;

        let repo_path = PathBuf::from(&repo.local_path);

        // Create a temporary worktree for planning update
        let base_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let worktree_path = base_tmp.join(format!("delidev/planning/{}", composite_task.id));
        let planning_branch_name = format!("delidev/planning/{}", composite_task.id);

        tracing::info!(
            "Creating planning worktree for plan update at {:?} for composite task {}",
            worktree_path,
            composite_task.id
        );

        // Remove existing worktree if it exists
        if worktree_path.exists() {
            tracing::info!("Removing existing planning worktree from previous run");
            if let Err(e) = self.git_service.remove_worktree(&repo_path, &worktree_path) {
                tracing::warn!("Failed to remove existing planning worktree: {}", e);
            }
        }

        // Create the planning worktree
        self.git_service
            .create_worktree(
                &repo_path,
                &worktree_path,
                &planning_branch_name,
                &repo.default_branch,
            )
            .map_err(|e| {
                PlanningError::Execution(format!("Failed to create planning worktree: {}", e))
            })?;

        // Execute the planning agent task in the worktree
        let result = execution_service
            .execute_agent_task(
                &agent_task,
                &worktree_path,
                &update_prompt,
                &composite_task.id,
            )
            .await;

        // Handle execution result
        match result {
            Ok(exec_result) => {
                if !exec_result.success {
                    self.cleanup_planning_worktree(
                        &repo_path,
                        &worktree_path,
                        &planning_branch_name,
                    );
                    return Err(PlanningError::Execution(
                        exec_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    ));
                }
            }
            Err(e) => {
                self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
                return Err(PlanningError::Execution(e.to_string()));
            }
        }

        // Copy the plan file from worktree to repo before cleaning up
        let worktree_plan_path = worktree_path.join(&plan_filename);
        let repo_plan_path = repo_path.join(&plan_filename);

        if worktree_plan_path.exists() {
            std::fs::copy(&worktree_plan_path, &repo_plan_path).map_err(|e| {
                self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
                PlanningError::Io(e)
            })?;
            tracing::info!(
                "Copied updated plan file from worktree to repo: {:?}",
                repo_plan_path
            );
        } else {
            self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);
            return Err(PlanningError::PlanFileNotFound(format!(
                "Updated plan file '{}' was not created by the planning agent",
                plan_filename
            )));
        }

        // Clean up the planning worktree
        self.cleanup_planning_worktree(&repo_path, &worktree_path, &planning_branch_name);

        // Read and persist the updated plan content
        let updated_content = std::fs::read_to_string(&repo_plan_path).map_err(|e| {
            PlanningError::PlanFileNotFound(format!(
                "Failed to read updated plan file '{}': {}",
                repo_plan_path.display(),
                e
            ))
        })?;

        // Store the updated plan content in the database
        self.store_plan_content_if_needed(&composite_task.id, &updated_content)
            .await?;

        // Delete the plan file from the repository
        if let Err(e) = std::fs::remove_file(&repo_plan_path) {
            tracing::warn!(
                "Failed to delete plan file '{}' after persisting: {}",
                repo_plan_path.display(),
                e
            );
        }

        Ok(plan_filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_string() {
        let s1 = CompositePlanningService::generate_random_string();
        let s2 = CompositePlanningService::generate_random_string();

        assert_eq!(s1.len(), 16);
        assert_eq!(s2.len(), 16);
        assert_ne!(s1, s2);

        // Check that all characters are alphanumeric
        for c in s1.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn test_generate_planning_prompt() {
        let prompt = CompositePlanningService::generate_planning_prompt(
            "Add user authentication to the app",
            "PLAN-abc123.yaml",
        );

        assert!(prompt.contains("Add user authentication to the app"));
        assert!(prompt.contains("PLAN-abc123.yaml"));
        assert!(prompt.contains("tasks:"));
        assert!(prompt.contains("dependsOn"));
    }
}
