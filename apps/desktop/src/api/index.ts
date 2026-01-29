import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  UnitTaskStatus,
  CompositeTaskStatus,
} from "../types";
import type {
  Repository,
  UnitTask,
  CompositeTask,
  GlobalConfig,
  RepositoryConfig,
  VCSProviderType,
  AIAgentType,
  AgentTask,
  LicenseInfo,
  Workspace,
  RepositoryGroup,
} from "../types";

// ========== System API ==========

export interface AppInfo {
  version: string;
  name: string;
}

export async function getAppInfo(): Promise<AppInfo> {
  return invoke("get_app_info");
}

export async function checkDocker(): Promise<boolean> {
  return invoke("check_docker");
}

export async function getDockerVersion(): Promise<string> {
  return invoke("get_docker_version");
}

/**
 * Opens a file in the configured external editor
 * Can optionally show a diff between two commits
 */
export async function openInEditor(params: {
  filePath: string;
  repoPath?: string;
  baseCommit?: string;
  headCommit?: string;
}): Promise<void> {
  return invoke("open_in_editor", params);
}

// ========== Config API ==========

export async function getGlobalConfig(): Promise<GlobalConfig> {
  return invoke("get_global_config");
}

export async function updateGlobalConfig(config: GlobalConfig): Promise<void> {
  return invoke("update_global_config", { config });
}

export async function getRepositoryConfig(
  repoPath: string
): Promise<RepositoryConfig> {
  return invoke("get_repository_config", { repoPath });
}

export async function updateRepositoryConfig(
  repoPath: string,
  config: RepositoryConfig
): Promise<void> {
  return invoke("update_repository_config", { repoPath, config });
}

export interface CredentialsStatus {
  github_configured: boolean;
  gitlab_configured: boolean;
  bitbucket_configured: boolean;
}

export async function getCredentialsStatus(): Promise<CredentialsStatus> {
  return invoke("get_credentials_status");
}

export interface VCSUser {
  username: string;
  name: string | null;
  avatar_url: string | null;
}

export async function setGithubToken(token: string): Promise<VCSUser> {
  return invoke("set_github_token", { token });
}

export async function setGitlabToken(token: string): Promise<VCSUser> {
  return invoke("set_gitlab_token", { token });
}

export async function setBitbucketCredentials(
  username: string,
  appPassword: string
): Promise<VCSUser> {
  return invoke("set_bitbucket_credentials", { username, appPassword });
}

export async function validateVcsCredentials(
  provider: VCSProviderType
): Promise<VCSUser> {
  return invoke("validate_vcs_credentials", { provider });
}

// ========== Repository API ==========

export async function listRepositories(): Promise<Repository[]> {
  return invoke("list_repositories");
}

export async function getRepository(id: string): Promise<Repository | null> {
  return invoke("get_repository", { id });
}

export async function addRepository(path: string): Promise<Repository> {
  return invoke("add_repository", { path });
}

export async function removeRepository(id: string): Promise<void> {
  return invoke("remove_repository", { id });
}

export interface RepositoryInfo {
  name: string;
  remote_url: string;
  provider: VCSProviderType;
  default_branch: string;
}

export async function validateRepositoryPath(
  path: string
): Promise<RepositoryInfo> {
  return invoke("validate_repository_path", { path });
}

export async function listRepositoryFiles(
  repositoryId: string,
  query?: string,
  limit?: number
): Promise<string[]> {
  return invoke("list_repository_files", { repositoryId, query, limit });
}

export async function listRepositoryGroupFiles(
  repositoryGroupId: string,
  query?: string,
  limit?: number
): Promise<string[]> {
  return invoke("list_repository_group_files", { repositoryGroupId, query, limit });
}

// ========== Workspace API ==========

export async function listWorkspaces(): Promise<Workspace[]> {
  return invoke("list_workspaces");
}

export async function getWorkspace(id: string): Promise<Workspace | null> {
  return invoke("get_workspace", { id });
}

export async function createWorkspace(
  name: string,
  description?: string
): Promise<Workspace> {
  return invoke("create_workspace", { name, description });
}

export async function updateWorkspace(
  id: string,
  name: string,
  description?: string
): Promise<Workspace> {
  return invoke("update_workspace", { id, name, description });
}

export async function deleteWorkspace(id: string): Promise<void> {
  return invoke("delete_workspace", { id });
}

export async function addRepositoryToWorkspace(
  workspaceId: string,
  repositoryId: string
): Promise<void> {
  return invoke("add_repository_to_workspace", { workspaceId, repositoryId });
}

export async function removeRepositoryFromWorkspace(
  workspaceId: string,
  repositoryId: string
): Promise<void> {
  return invoke("remove_repository_from_workspace", { workspaceId, repositoryId });
}

export async function listWorkspaceRepositories(
  workspaceId: string
): Promise<Repository[]> {
  return invoke("list_workspace_repositories", { workspaceId });
}

export async function getDefaultWorkspace(): Promise<Workspace> {
  return invoke("get_default_workspace");
}

// ========== Repository Group API ==========

export async function listRepositoryGroups(
  workspaceId?: string
): Promise<RepositoryGroup[]> {
  return invoke("list_repository_groups", { workspaceId });
}

export async function getRepositoryGroup(
  id: string
): Promise<RepositoryGroup | null> {
  return invoke("get_repository_group", { id });
}

export async function createRepositoryGroup(
  workspaceId: string,
  name?: string,
  repositoryIds?: string[]
): Promise<RepositoryGroup> {
  return invoke("create_repository_group", {
    workspaceId,
    name,
    repositoryIds: repositoryIds ?? []
  });
}

export async function updateRepositoryGroup(
  id: string,
  name?: string
): Promise<RepositoryGroup> {
  return invoke("update_repository_group", { id, name });
}

export async function deleteRepositoryGroup(id: string): Promise<void> {
  return invoke("delete_repository_group", { id });
}

export async function addRepositoryToGroup(
  groupId: string,
  repositoryId: string
): Promise<void> {
  return invoke("add_repository_to_group", { groupId, repositoryId });
}

export async function removeRepositoryFromGroup(
  groupId: string,
  repositoryId: string
): Promise<void> {
  return invoke("remove_repository_from_group", { groupId, repositoryId });
}

/**
 * Gets or creates a single-repo group for a repository
 * This is used when creating tasks for a single repository
 */
export async function getOrCreateSingleRepoGroup(
  workspaceId: string,
  repositoryId: string
): Promise<string> {
  return invoke("get_or_create_single_repo_group", { workspaceId, repositoryId });
}

// ========== Task API ==========

export async function listUnitTasks(
  repositoryId?: string
): Promise<UnitTask[]> {
  return invoke("list_unit_tasks", { repositoryId });
}

export async function getUnitTask(id: string): Promise<UnitTask | null> {
  return invoke("get_unit_task", { id });
}

export async function getAgentTask(id: string): Promise<AgentTask | null> {
  return invoke("get_agent_task", { id });
}

export async function createUnitTask(params: {
  repositoryGroupId: string;
  prompt: string;
  title?: string;
  branchName?: string;
  agentType?: AIAgentType;
}): Promise<UnitTask> {
  return invoke("create_unit_task", params);
}

export async function updateUnitTaskStatus(
  id: string,
  status: UnitTaskStatus
): Promise<void> {
  return invoke("update_unit_task_status", { id, status });
}

export async function deleteUnitTask(id: string): Promise<void> {
  return invoke("delete_unit_task", { id });
}

export async function renameUnitTaskBranch(id: string, branchName: string): Promise<void> {
  return invoke("rename_unit_task_branch", { id, branchName });
}

/**
 * Requests changes for a unit task by appending feedback to the prompt
 * and setting status back to in_progress
 */
export async function requestUnitTaskChanges(id: string, feedback: string): Promise<void> {
  return invoke("request_unit_task_changes", { id, feedback });
}

export async function listCompositeTasks(
  repositoryId?: string
): Promise<CompositeTask[]> {
  return invoke("list_composite_tasks", { repositoryId });
}

export async function getCompositeTask(
  id: string
): Promise<CompositeTask | null> {
  return invoke("get_composite_task", { id });
}

export async function createCompositeTask(params: {
  repositoryGroupId: string;
  prompt: string;
  title?: string;
  planningAgentType?: AIAgentType;
  executionAgentType?: AIAgentType;
}): Promise<CompositeTask> {
  return invoke("create_composite_task", params);
}

export async function updateCompositeTaskStatus(
  id: string,
  status: CompositeTaskStatus
): Promise<void> {
  return invoke("update_composite_task_status", { id, status });
}

export async function deleteCompositeTask(id: string): Promise<void> {
  return invoke("delete_composite_task", { id });
}

/**
 * Starts the planning phase for a composite task
 * This creates an agent session that generates a PLAN-{randomString}.yaml file
 */
export async function startCompositeTaskPlanning(
  compositeTaskId: string
): Promise<string> {
  return invoke("start_composite_task_planning", { compositeTaskId });
}

/**
 * Gets the plan YAML content for a composite task
 * Returns null if the plan file doesn't exist or hasn't been generated yet
 */
export async function getCompositeTaskPlan(
  compositeTaskId: string
): Promise<string | null> {
  return invoke("get_composite_task_plan", { compositeTaskId });
}

/**
 * Approves the plan for a composite task and starts execution
 * This creates UnitTasks and CompositeTaskNodes from the plan
 */
export async function approveCompositeTaskPlan(
  compositeTaskId: string
): Promise<void> {
  return invoke("approve_composite_task_plan", { compositeTaskId });
}

/**
 * Rejects the plan for a composite task
 * This deletes the plan file and sets the status to Rejected
 */
export async function rejectCompositeTaskPlan(
  compositeTaskId: string
): Promise<void> {
  return invoke("reject_composite_task_plan", { compositeTaskId });
}

/**
 * Updates the plan for a composite task based on user feedback
 * This runs a new Claude session with the current plan and user's update request
 */
export async function updateCompositeTaskPlan(
  compositeTaskId: string,
  updateRequest: string
): Promise<string> {
  return invoke("update_composite_task_plan", { compositeTaskId, updateRequest });
}

// Composite task status keys for the kanban board
// Note: Planning is grouped into "in_progress" for the kanban view
// PendingApproval is grouped into "in_review" for the kanban view
export enum CompositeTaskStatusKey {
  InProgress = "composite_in_progress",
  InReview = "composite_in_review",
  Done = "composite_done",
  Rejected = "composite_rejected",
}

export interface TasksByStatus {
  [UnitTaskStatus.InProgress]: UnitTask[];
  [UnitTaskStatus.InReview]: UnitTask[];
  [UnitTaskStatus.Approved]: UnitTask[];
  [UnitTaskStatus.PrOpen]: UnitTask[];
  [UnitTaskStatus.Done]: UnitTask[];
  [UnitTaskStatus.Rejected]: UnitTask[];
  [CompositeTaskStatusKey.InProgress]: CompositeTask[];
  [CompositeTaskStatusKey.InReview]: CompositeTask[];
  [CompositeTaskStatusKey.Done]: CompositeTask[];
  [CompositeTaskStatusKey.Rejected]: CompositeTask[];
}

export async function getTasksByStatus(workspaceId?: string): Promise<TasksByStatus> {
  return invoke("get_tasks_by_status", { workspaceId });
}

// ========== Execution API ==========

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface ExecutionLog {
  id: string;
  session_id: string;
  timestamp: string;
  level: LogLevel;
  message: string;
}

export async function startTaskExecution(taskId: string): Promise<void> {
  return invoke("start_task_execution", { taskId });
}

export async function stopTaskExecution(taskId: string): Promise<void> {
  return invoke("stop_task_execution", { taskId });
}

export async function getExecutionLogs(
  sessionId: string
): Promise<ExecutionLog[]> {
  return invoke("get_execution_logs", { sessionId });
}

export async function getAllExecutionLogs(): Promise<ExecutionLog[]> {
  return invoke("get_all_execution_logs");
}

export async function getHistoricalExecutionLogs(
  sessionId: string
): Promise<ExecutionLog[]> {
  return invoke("get_historical_execution_logs", { sessionId });
}

export async function cleanupTask(taskId: string): Promise<void> {
  return invoke("cleanup_task", { taskId });
}

export async function isDockerAvailable(): Promise<boolean> {
  return invoke("is_docker_available");
}

/**
 * Checks if a task is currently executing (Docker container is running)
 */
export async function isTaskExecuting(taskId: string): Promise<boolean> {
  return invoke("is_task_executing", { taskId });
}

/**
 * Gets the git diff for a task's worktree
 * Returns null if worktree doesn't exist or diff cannot be computed
 */
export async function getTaskDiff(taskId: string): Promise<string | null> {
  return invoke("get_task_diff", { taskId });
}

/**
 * Creates a PR for a task and returns the PR URL
 * This will push the branch and create a PR on the VCS provider
 */
export async function createPrForTask(taskId: string): Promise<string> {
  return invoke("create_pr_for_task", { taskId });
}

/**
 * Merge strategy for local merging
 */
export type MergeStrategy = "merge" | "squash" | "rebase";

/**
 * Commits worktree changes to the main repository's current branch
 * This will merge the task's branch into the current branch and update status to Done
 * @param taskId - The task ID to commit
 * @param mergeStrategy - Optional merge strategy (defaults to "merge")
 */
export async function commitToRepository(
  taskId: string,
  mergeStrategy?: MergeStrategy
): Promise<void> {
  return invoke("commit_to_repository", { taskId, mergeStrategy });
}

// ========== Claude Stream Types ==========

export type ClaudeStreamMessageType = "system" | "assistant" | "user" | "result";

export interface ContentBlockText {
  type: "text";
  text: string;
}

export interface ContentBlockToolUse {
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export interface ContentBlockToolResult {
  type: "tool_result";
  tool_use_id: string;
  content?: string;
  is_error?: boolean;
}

export type ContentBlock = ContentBlockText | ContentBlockToolUse | ContentBlockToolResult;

export interface AssistantMessage {
  id?: string;
  type?: string;
  role?: string;
  content: ContentBlock[];
  model?: string;
  stop_reason?: string;
  stop_sequence?: string;
}

export interface UserMessage {
  role?: string;
  content: ContentBlock[];
}

export interface ClaudeStreamSystem {
  type: "system";
  subtype: string;
  parent_tool_use_id?: string | null;
  [key: string]: unknown;
}

export interface ClaudeStreamAssistant {
  type: "assistant";
  message: AssistantMessage;
  parent_tool_use_id?: string | null;
}

export interface ClaudeStreamUser {
  type: "user";
  message: UserMessage;
  parent_tool_use_id?: string | null;
}

export interface ClaudeStreamResult {
  type: "result";
  subtype: string;
  cost_usd?: number;
  duration_ms?: number;
  duration_api_ms?: number;
  is_error?: boolean;
  num_turns?: number;
  result?: string;
  session_id?: string;
  parent_tool_use_id?: string | null;
}

export type ClaudeStreamMessage =
  | ClaudeStreamSystem
  | ClaudeStreamAssistant
  | ClaudeStreamUser
  | ClaudeStreamResult;

export interface ClaudeStreamEvent {
  task_id: string;
  session_id: string;
  timestamp: string;
  message: ClaudeStreamMessage;
}

// ========== Event Types ==========

export interface ExecutionLogEvent {
  task_id: string;
  session_id: string;
  log: ExecutionLog;
}

export interface ExecutionProgressEvent {
  task_id: string;
  session_id: string;
  phase:
    | "starting"
    | "worktree"
    | "container"
    | "executing"
    | "completed"
    | "failed"
    | "cleanup";
  message: string;
}

export interface TaskStatusEvent {
  task_id: string;
  old_status: string;
  new_status: string;
}

// ========== Event Listeners ==========

/**
 * Listen for execution log events (real-time streaming)
 */
export async function onExecutionLog(
  callback: (event: ExecutionLogEvent) => void
): Promise<UnlistenFn> {
  return listen<ExecutionLogEvent>("execution-log", (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for execution progress events
 */
export async function onExecutionProgress(
  callback: (event: ExecutionProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<ExecutionProgressEvent>("execution-progress", (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for task status change events
 */
export async function onTaskStatusChanged(
  callback: (event: TaskStatusEvent) => void
): Promise<UnlistenFn> {
  return listen<TaskStatusEvent>("task-status-changed", (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for Claude Code stream events (structured JSON)
 */
export async function onClaudeStream(
  callback: (event: ClaudeStreamEvent) => void
): Promise<UnlistenFn> {
  return listen<ClaudeStreamEvent>("claude-stream", (event) => {
    callback(event.payload);
  });
}

/**
 * Agent stream message entry (stored in database)
 */
export interface AgentStreamMessageEntry {
  id: string;
  session_id: string;
  timestamp: string;
  message: ClaudeStreamMessage;
}

/**
 * Gets historical agent stream messages for a session
 */
export async function getStreamMessages(
  sessionId: string
): Promise<AgentStreamMessageEntry[]> {
  return invoke("get_stream_messages", { sessionId });
}

// ========== License API ==========

/**
 * Gets the current license information
 */
export async function getLicenseInfo(): Promise<LicenseInfo> {
  return invoke("get_license_info");
}

/**
 * Checks if a license is configured
 */
export async function hasLicense(): Promise<boolean> {
  return invoke("has_license");
}

/**
 * Checks if the license is valid and active
 */
export async function isLicenseValid(): Promise<boolean> {
  return invoke("is_license_valid");
}

/**
 * Validates the current license key with Polar.sh
 */
export async function validateLicense(): Promise<LicenseInfo> {
  return invoke("validate_license");
}

/**
 * Activates a license key for this device
 */
export async function activateLicense(
  key: string,
  deviceLabel?: string
): Promise<LicenseInfo> {
  return invoke("activate_license", { key, deviceLabel });
}

/**
 * Deactivates the license for this device
 */
export async function deactivateLicense(): Promise<void> {
  return invoke("deactivate_license");
}

/**
 * Sets a license key without activation (for keys that don't require activation)
 */
export async function setLicenseKey(key: string): Promise<LicenseInfo> {
  return invoke("set_license_key", { key });
}

/**
 * Removes the license key from this device
 */
export async function removeLicense(): Promise<void> {
  return invoke("remove_license");
}

/**
 * Gets the suggested device label for activation
 */
export async function getDeviceLabel(): Promise<string> {
  return invoke("get_device_label");
}

// ========== TTY Input API ==========

/**
 * Type of TTY input expected from the user
 */
export type TtyInputType = "text" | "confirm" | "select";

/**
 * Status of a TTY input request
 */
export type TtyInputStatus = "pending" | "answered" | "cancelled" | "expired";

/**
 * An option for select-type TTY input
 */
export interface TtyInputOption {
  label: string;
  description?: string;
}

/**
 * A TTY input request from an AI coding agent
 */
export interface TtyInputRequest {
  id: string;
  task_id: string;
  session_id: string;
  prompt: string;
  input_type: TtyInputType;
  options: TtyInputOption[];
  created_at: string;
  status: TtyInputStatus;
  response?: string;
  responded_at?: string;
}

/**
 * Event payload for TTY input request
 */
export interface TtyInputRequestEvent {
  request: TtyInputRequest;
}

/**
 * Gets all pending TTY input requests
 */
export async function getPendingTtyInputs(): Promise<TtyInputRequest[]> {
  return invoke("get_pending_tty_inputs");
}

/**
 * Gets pending TTY input requests for a specific task
 */
export async function getTaskTtyInputs(taskId: string): Promise<TtyInputRequest[]> {
  return invoke("get_task_tty_inputs", { taskId });
}

/**
 * Gets a specific TTY input request by ID
 */
export async function getTtyInput(requestId: string): Promise<TtyInputRequest | null> {
  return invoke("get_tty_input", { requestId });
}

/**
 * Submits a response to a TTY input request
 */
export async function submitTtyInput(requestId: string, response: string): Promise<void> {
  return invoke("submit_tty_input", { requestId, response });
}

/**
 * Cancels a TTY input request
 */
export async function cancelTtyInput(requestId: string): Promise<void> {
  return invoke("cancel_tty_input", { requestId });
}

/**
 * Cancels all TTY input requests for a task
 */
export async function cancelTaskTtyInputs(taskId: string): Promise<void> {
  return invoke("cancel_task_tty_inputs", { taskId });
}

/**
 * Listen for TTY input request events
 */
export async function onTtyInputRequest(
  callback: (event: TtyInputRequestEvent) => void
): Promise<UnlistenFn> {
  return listen<TtyInputRequestEvent>("tty-input-request", (event) => {
    callback(event.payload);
  });
}

// ========== Custom Command API ==========

import type {
  CustomCommand,
  CommandFramework,
} from "../types";

/**
 * Lists all custom commands for a repository
 * Discovers commands from .claude/commands/, .opencode/command/, and global locations
 */
export async function listCustomCommands(
  repositoryId: string
): Promise<CustomCommand[]> {
  return invoke("list_custom_commands", { repositoryId });
}

/**
 * Gets a specific custom command by name
 */
export async function getCustomCommand(
  repositoryId: string,
  commandName: string
): Promise<CustomCommand> {
  return invoke("get_custom_command", { repositoryId, commandName });
}

/**
 * Lists custom commands filtered by agent type
 */
export async function listCustomCommandsByAgent(
  repositoryId: string,
  agentType: AIAgentType
): Promise<CustomCommand[]> {
  return invoke("list_custom_commands_by_agent", { repositoryId, agentType });
}

/**
 * Lists custom commands filtered by framework
 */
export async function listCustomCommandsByFramework(
  repositoryId: string,
  framework: CommandFramework
): Promise<CustomCommand[]> {
  return invoke("list_custom_commands_by_framework", { repositoryId, framework });
}

/**
 * Renders a custom command template with arguments
 * Replaces $ARGUMENTS with all args, $1, $2, etc. with positional args
 */
export async function renderCustomCommand(
  repositoryId: string,
  commandName: string,
  args: string
): Promise<string> {
  return invoke("render_custom_command", { repositoryId, commandName, arguments: args });
}

