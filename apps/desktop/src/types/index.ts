// Version Control System types
export enum VCSType {
  Git = "git",
}

// VCS hosting provider types
export enum VCSProviderType {
  GitHub = "github",
  GitLab = "gitlab",
  Bitbucket = "bitbucket",
}

// AI coding agent types
export enum AIAgentType {
  ClaudeCode = "claude_code",
  OpenCode = "open_code",
}

// Container runtime types
export enum ContainerRuntime {
  Docker = "docker",
  Podman = "podman",
}

// External editor types
export enum EditorType {
  Vscode = "vscode",
  Cursor = "cursor",
  Windsurf = "windsurf",
  VscodeInsiders = "vscode_insiders",
  Vscodium = "vscodium",
}

// UnitTask status
export enum UnitTaskStatus {
  InProgress = "in_progress",
  InReview = "in_review",
  Approved = "approved",
  PrOpen = "pr_open",
  Done = "done",
  Rejected = "rejected",
}

// CompositeTask status
export enum CompositeTaskStatus {
  Planning = "planning",
  PendingApproval = "pending_approval",
  InProgress = "in_progress",
  Done = "done",
  Rejected = "rejected",
}

// TodoItem source
export enum TodoItemSource {
  Auto = "auto",
  Manual = "manual",
}

// TodoItem status
export enum TodoItemStatus {
  Pending = "pending",
  InProgress = "in_progress",
  Done = "done",
  Dismissed = "dismissed",
}

// Base interfaces
export interface BaseRemote {
  git_remote_dir_path: string;
  git_branch_name: string;
}

export interface AgentSession {
  id: string;
  ai_agent_type: AIAgentType;
  ai_agent_model?: string;
}

export interface AgentTask {
  id: string;
  base_remotes: BaseRemote[];
  agent_sessions: AgentSession[];
  ai_agent_type?: AIAgentType;
  ai_agent_model?: string;
}

export interface UnitTask {
  id: string;
  title: string;
  prompt: string;
  agent_task_id: string;
  branch_name?: string;
  linked_pr_url?: string;
  base_commit?: string;
  end_commit?: string;
  auto_fix_task_ids: string[];
  status: UnitTaskStatus;
  repository_group_id: string;
  created_at: string;
  updated_at: string;
  /** Parent CompositeTask ID if this UnitTask belongs to a CompositeTask */
  composite_task_id?: string;
  /** Whether the last execution attempt failed */
  last_execution_failed: boolean;
}

export interface CompositeTaskNode {
  id: string;
  unit_task_id: string;
  depends_on: string[];
}

export interface CompositeTask {
  id: string;
  title: string;
  prompt: string;
  planning_task_id: string;
  nodes: CompositeTaskNode[];
  status: CompositeTaskStatus;
  repository_group_id: string;
  plan_file_path?: string;
  plan_yaml_content?: string;
  execution_agent_type?: AIAgentType;
  created_at: string;
  updated_at: string;
}

export interface Repository {
  id: string;
  vcs_type: VCSType;
  vcs_provider_type: VCSProviderType;
  remote_url: string;
  name: string;
  local_path: string;
  default_branch: string;
  created_at: string;
}

// Workspace types
export interface Workspace {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

// Repository Group types
export interface RepositoryGroup {
  id: string;
  name?: string;
  workspace_id: string;
  repository_ids: string[];
  created_at: string;
  updated_at: string;
}

// TodoItem types
export interface BaseTodoItem {
  id: string;
  source: TodoItemSource;
  created_at: string;
  status: TodoItemStatus;
}

export interface IssueTriageTodoItem extends BaseTodoItem {
  type: "issue_triage";
  issue_url: string;
  repository_id: string;
  issue_title: string;
  suggested_labels?: string[];
  suggested_assignees?: string[];
}

export interface PrReviewTodoItem extends BaseTodoItem {
  type: "pr_review";
  pr_url: string;
  repository_id: string;
  pr_title: string;
  changed_files_count: number;
  ai_summary?: string;
}

export type TodoItem = IssueTriageTodoItem | PrReviewTodoItem;

// Configuration types
export interface AgentConfig {
  type: AIAgentType;
  model: string;
}

export interface LearningConfig {
  auto_learn_from_reviews: boolean;
}

export interface HotkeyConfig {
  open_chat: string;
}

export interface NotificationConfig {
  enabled: boolean;
  approval_request: boolean;
  user_question: boolean;
  review_ready: boolean;
}

export interface GlobalAgentConfig {
  planning: AgentConfig;
  execution: AgentConfig;
  chat: AgentConfig;
}

export interface ContainerConfig {
  runtime: ContainerRuntime;
  socket_path?: string;
  use_container: boolean;
}

export interface ConcurrencyConfig {
  max_concurrent_sessions?: number;
}

export interface EditorConfig {
  editor_type: EditorType;
}

export interface GlobalConfig {
  learning: LearningConfig;
  hotkey: HotkeyConfig;
  notification: NotificationConfig;
  agent: GlobalAgentConfig;
  container: ContainerConfig;
  editor: EditorConfig;
  concurrency: ConcurrencyConfig;
}

export enum AutoFixReviewFilter {
  WriteAccessOnly = "write_access_only",
  All = "all",
}

// Docker configuration is now done via .delidev/setup/Dockerfile
export interface DockerConfig {
  // All configuration is now done via .delidev/setup/Dockerfile
}

export interface BranchConfig {
  template: string;
}

export interface AutomationConfig {
  auto_fix_review_comments: boolean;
  auto_fix_review_comments_filter: AutoFixReviewFilter;
  auto_fix_ci_failures: boolean;
  max_auto_fix_attempts: number;
}

export interface RepositoryLearningConfig {
  auto_learn_from_reviews?: boolean;
}

export interface RepositoryConfig {
  docker: DockerConfig;
  branch: BranchConfig;
  automation: AutomationConfig;
  learning: RepositoryLearningConfig;
}

// License types
export enum LicenseStatus {
  Active = "active",
  Expired = "expired",
  Invalid = "invalid",
  Revoked = "revoked",
  Pending = "pending",
  NotConfigured = "not_configured",
}

export interface LicenseInfo {
  display_key?: string;
  status: LicenseStatus;
  customer_email?: string;
  customer_name?: string;
  expires_at?: string;
  last_validated_at?: string;
  activations_used?: number;
  activation_limit?: number;
  activation_id?: string;
}

// Custom Command types
export enum CommandSource {
  Project = "project",
  Global = "global",
}

export enum CommandFramework {
  ClaudeCode = "claude_code",
  OpenCode = "open_code",
}

export interface CustomCommand {
  name: string;
  display_name: string;
  description?: string;
  agent_type: AIAgentType;
  model?: string;
  template: string;
  source: CommandSource;
  framework: CommandFramework;
  relative_path: string;
  is_subtask: boolean;
}
