# DeliDev Design Document

DeliDev is a local-first desktop application for orchestrating AI coding agents.

## Table of Contents

1. [Architecture](#architecture)
2. [Technology Stack](#technology-stack)
3. [Entities](#entities)
4. [Configuration](#configuration)
5. [Workflows](#workflows)
6. [UI Design](./ui.md)
7. [PLAN.yaml Specification](./plan-yaml.md)

---

## Architecture

### Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Desktop App (Local)                      │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Dashboard  │  │  Chat + AI  │  │  Review Interface   │  │
│  │  (Kanban)   │  │   (Voice)   │  │  (Diff Viewer)      │  │
│  └─────────────┘  └──────┬──────┘  └─────────────────────┘  │
│                          │                                   │
│                          ▼                                   │
│                 ┌─────────────────┐                          │
│                 │ Local AI Agent  │ (no Docker)              │
│                 │ (Claude Code,   │                          │
│                 │  OpenCode, etc) │                          │
│                 └─────────────────┘                          │
├─────────────────────────────────────────────────────────────┤
│                      Core Services                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │ Task     │ │ Docker   │ │ VCS      │ │ Learning     │   │
│  │ Manager  │ │ Manager  │ │ Provider │ │ Service      │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                      Local Storage                           │
│  ┌──────────────────┐  ┌─────────────────────────────────┐  │
│  │ SQLite Database  │  │ Global Config (~/.delidev/)     │  │
│  └──────────────────┘  └─────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Docker Containers                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │      Agent Sandbox (per task session, NOT chat)      │    │
│  │  ┌───────────────┐  ┌─────────────────────────────┐ │    │
│  │  │ Git Worktree  │  │ AI Agent (Claude Code, etc) │ │    │
│  │  └───────────────┘  └─────────────────────────────┘ │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    External Services                         │
│  ┌──────────────────┐  ┌─────────────────────────────────┐  │
│  │ VCS Provider API │  │   AI Provider APIs              │  │
│  │ (GitHub, GitLab, │  │   (Anthropic, OpenAI, etc)      │  │
│  │  Bitbucket, etc) │  │                                 │  │
│  └──────────────────┘  └─────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Key Components

#### Desktop App

- **Framework**: Tauri (Rust + WebView)
- **All processing runs locally** - no backend server required
- **Global hotkey** support for quick access to chat

#### Core Services

**Task Manager**
- Manages UnitTask and CompositeTask lifecycle
- Handles task status transitions
- Coordinates parallel execution of CompositeTaskNodes

**Docker Manager**
- Creates and manages Docker containers for agent sandboxes
- Builds image from `.delidev/setup/Dockerfile` or uses default (node:20-slim)
- Isolates each AgentSession in its own container

**VCS Provider Client**
- Abstracts interactions with VCS provider APIs (GitHub, GitLab, Bitbucket, etc.)
- Creates and manages PRs/MRs
- Monitors review comments and CI status
- Fetches issues for triage
- Provider-specific implementations behind a common interface

**Learning Service**
- Extracts learning points from VCS provider reviews
- Requests local user approval
- Updates CLAUDE.md / AGENTS.md

**Notification Service**
- Sends desktop notifications for task status changes
- Notifies on task completion, review readiness, and failures
- Alerts on CI failures and review comments
- Uses native OS notification system via custom platform-specific module:
  - **Windows**: Uses `tauri-winrt-notification` with click handler callback
  - **Linux**: Uses `notify-rust` with D-Bus action support
  - **macOS**: Uses AppleScript for display (native delegate implementation TODO)
- Emits Tauri events when notifications are clicked for deep linking to task detail pages

**TTY Input Proxy Service**
- Intercepts TTY input requests from AI coding agents (Claude Code, OpenCode, etc.)
- Shows desktop notifications when agents request user input
- Provides web form interface for users to answer questions
- Sends responses back to agents via pseudo-TTY
- Enables human-in-the-loop interaction during agent execution

#### Local Storage

**SQLite Database**
- Stores all entities (Tasks, Sessions, TodoItems, etc.)
- Maintains task history and logs

**Global Config**
- User-specific settings in `~/.delidev/config.toml`
- Hotkey configuration
- Default agent settings (planning agent/model, execution agent/model)

**Chat AI Agent**
- Runs AI coding agents locally without Docker containers
- Executes directly in the user's working directory
- Uses planning agent settings for CompositeTask planning
- Uses execution agent settings for direct code modifications

#### Docker Containers

Each AgentSession runs in an isolated Docker container:

1. **Git Worktree**: Created before execution for isolated workspace
2. **AI Agent**: The actual coding agent (Claude Code, OpenCode, etc.)
3. **Base Image**: Configured per repository (`.delidev/config.toml`)

**Container Directory Structure:**
- `HOME=/workspace` - Home directory for the agent
- `CWD=/workspace/$repoName` - Working directory where repository code is mounted (e.g., `/workspace/myrepo`)

#### External Services

- **VCS Provider APIs**: PR/MR creation, review management, CI status (GitHub, GitLab, Bitbucket, etc.)
- **AI Provider APIs**: Model inference for coding agents

### Design Principles

1. **Local-First**: All processing happens locally, no backend server
2. **Sandboxed Execution**: Each agent runs in isolated Docker container
3. **Git Worktree Isolation**: Each task gets its own worktree
4. **Human-in-the-Loop**: Code review gate before PR creation (AI slop prevention)
5. **Automation with Control**: Auto-fix features can be toggled on/off

### Distributed Architecture (Optional)

DeliDev supports an optional distributed mode for remote execution:

```
                                ┌─────────────────────────────────┐
                                │         Main Server             │
                                │  (Task Management, RPC Server)  │
                                │                                 │
                                │  ┌─────────────────────────────┐│
                                │  │      PostgreSQL / SQLite    ││
                                │  │      (multi/single mode)    ││
                                │  └─────────────────────────────┘│
                                │                                 │
                                │  JWT Auth (OpenID Connect)      │
                                └─────────────┬───────────────────┘
                                              │
                       JSON-RPC over HTTP/WebSocket
                                              │
                ┌─────────────────────────────┼─────────────────────────────┐
                │                             │                             │
                ▼                             ▼                             ▼
    ┌───────────────────┐       ┌───────────────────┐       ┌───────────────────┐
    │   Worker Server   │       │   Worker Server   │       │      Client       │
    │                   │       │                   │       │  (Desktop/Mobile) │
    │  ┌─────────────┐  │       │  ┌─────────────┐  │       │                   │
    │  │Claude Code  │  │       │  │Claude Code  │  │       │  React + Tauri    │
    │  │OpenCode     │  │       │  │OpenCode     │  │       │  react-query      │
    │  │Aider, etc.  │  │       │  │Aider, etc.  │  │       │                   │
    │  └─────────────┘  │       │  └─────────────┘  │       │  Keychain Access  │
    │                   │       │                   │       │  (macOS, etc.)    │
    │  Docker Sandbox   │       │  Docker Sandbox   │       │                   │
    └───────────────────┘       └───────────────────┘       └───────────────────┘
```

**Components:**

- **Main Server (`apps/server`)**: Task management, worker coordination, JWT authentication
- **Worker Server (`apps/worker`)**: AI agent execution, Docker sandboxing, heartbeat reporting
- **Client**: Desktop/mobile app connecting via JSON-RPC

**Single Process Mode:**

In single-process mode, the desktop app embeds all components (server + worker) for a seamless local experience with no network overhead.

---

## Technology Stack

### Desktop Framework
- **Tauri**: Rust-based desktop framework using system WebView
  - Small binary size (~10MB)
  - Low memory footprint
  - Native system integration

### Backend (Rust)
| Crate | Purpose |
|-------|---------|
| sqlx | Async SQLite/PostgreSQL driver |
| bollard | Docker API client |
| git2 | Git operations |
| reqwest | HTTP client for VCS/RPC APIs |
| serde | Serialization |
| tokio | Async runtime |
| axum | Web server framework |
| jsonwebtoken | JWT authentication |

### Shared Crates

| Crate | Purpose |
|-------|---------|
| coding_agents | AI agent abstraction & Docker sandboxing |
| task_store | Task storage (SQLite, PostgreSQL, in-memory) |
| rpc_protocol | JSON-RPC 2.0 protocol definitions |
| git_ops | Git operations & worktree management |
| auth | JWT authentication & RBAC |
| secrets | Cross-platform keychain access |

### Frontend (React + TypeScript)
| Package | Purpose |
|---------|---------|
| react | UI framework |
| typescript | Type safety |
| @rspack/core | Build tool (Rust-based) |
| tailwindcss | Utility-first CSS |
| shadcn/ui | Component library |
| zustand | State management |

### Frontend API Layer

The frontend includes a flexible API layer that supports both single-process mode (Tauri invoke) and remote client mode (JSON-RPC):

| Module | Purpose |
|--------|---------|
| `api/rpc.ts` | JSON-RPC 2.0 client for server communication |
| `api/hooks.ts` | React hooks for data fetching (works in both modes) |
| `api/client-config.ts` | Client mode configuration management |
| `api/ClientProvider.tsx` | React context provider for client state |

**Mode Switching:**
- **Single Process Mode**: Uses Tauri `invoke()` commands directly. No network overhead.
- **Remote Mode**: Uses JSON-RPC over HTTP/WebSocket to communicate with remote server.

The hooks automatically detect the current mode and route API calls appropriately.

### Supported Platforms
- Windows (x64, arm64)
- macOS (x64, arm64)
- Linux (x64, arm64)

---

## Entities

Core data models for DeliDev.

### VCSType

Version Control System types.

```
enum VCSType {
  git           // Git
  // Future: mercurial, svn, etc.
}
```

### VCSProviderType

VCS hosting provider types.

```
enum VCSProviderType {
  github        // GitHub
  gitlab        // GitLab
  bitbucket     // Bitbucket
  // Future: Azure DevOps, Gitea, etc.
}
```

### AIAgentType

Types of AI coding agents.

```
enum AIAgentType {
  claude_code    // Claude Code - Anthropic's terminal-based agentic coding tool
  open_code      // OpenCode - Open-source Claude Code alternative supporting any model
  gemini_cli     // Gemini CLI - Google's open-source AI agent for terminal (Apache 2.0 license)
  codex_cli      // Codex CLI - OpenAI's interactive terminal-based coding assistant
  aider          // Aider - Open-source CLI tool for multi-file changes via natural language
  amp            // Amp - Sourcegraph's agentic coding CLI tool
}
```

### AgentSession

A single AI coding agent session.

The system creates a workspace using git worktree before execution, then runs the AI coding agent inside a Docker container. The Docker image is built from `.delidev/setup/Dockerfile` if present, otherwise the default image (node:20-slim) is used.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| aiAgentType | AIAgentType | Y | Agent type |
| aiAgentModel | string | N | Model to use (uses default if not specified) |

### AgentTask

A collection of AgentSessions. The retryable unit.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| baseRemotes | BaseRemote[] | Y | Git repository information |
| agentSessions | AgentSession[] | Y | Session list (default 1, more on retry) |
| aiAgentType | AIAgentType | N | Agent type (uses default agent if not specified) |
| aiAgentModel | string | N | Model to use |

#### BaseRemote

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| gitRemoteDirPath | string | Y | Git repository path |
| gitBranchName | string | Y | Branch name |

### UnitTask

A single task unit visible to users.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| repositoryGroupId | string | Y | Associated RepositoryGroup ID |
| agentTask | AgentTask | Y | Associated AgentTask (1:1) |
| branchName | string | N | Custom branch name (uses template if not specified) |
| linkedPrUrl | string | N | Created PR URL |
| baseCommit | string | N | Base commit hash of default branch when task was created (for accurate diffs) |
| endCommit | string | N | End commit hash when task execution completed (for task-specific diffs) |
| autoFixTasks | AgentTask[] | Y | List of auto-fix attempts |
| status | UnitTaskStatus | Y | Current status |

#### UnitTaskStatus

```
enum UnitTaskStatus {
  in_progress   // AI is working
  in_review     // AI work complete, awaiting human review
  approved      // Human approved, ready to merge or create PR
  pr_open       // PR created, awaiting merge
  done          // PR merged
  rejected      // Rejected and discarded
}
```

### CompositeTask

Task graph-based Agent Orchestrator.

When a user creates a CompositeTask, the system creates an Agent Session that generates a task graph as PLAN-{randomString}.yaml to perform the requested work.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| repositoryGroupId | string | Y | Associated RepositoryGroup ID |
| planningTask | AgentTask | Y | AgentTask for generating PLAN.yaml (1:1) |
| tasks | CompositeTaskNode[] | Y | List of task nodes |
| status | CompositeTaskStatus | Y | Current status |
| executionAgentType | AIAgentType | N | Agent type for executing UnitTasks (defaults to global config)

#### CompositeTaskStatus

```
enum CompositeTaskStatus {
  planning           // planningTask is generating PLAN.yaml
  pending_approval   // Waiting for user approval
  in_progress        // Tasks are executing
  done               // All tasks completed
  rejected           // User rejected the plan
}
```

### CompositeTaskNode

A task node belonging to a CompositeTask.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| unitTask | UnitTask | Y | Associated UnitTask |
| dependsOn | CompositeTaskNode[] | Y | List of dependent nodes |

#### Parallel Execution Rules

When a CompositeTask is approved:
- Nodes with empty `dependsOn` execute immediately in parallel
- When each node completes, other nodes whose `dependsOn` conditions are satisfied also execute in parallel
- CompositeTask completes when all nodes are done

### TodoItem

Tasks that humans should do but AI can assist with. Tagged union structure.

#### Common Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| source | TodoItemSource | Y | Creation source |
| createdAt | timestamp | Y | Creation time |
| status | TodoItemStatus | Y | Current status |
| type | string | Y | Type discriminator |

```
enum TodoItemSource {
  auto     // System auto-generated
  manual   // User manually added
}

enum TodoItemStatus {
  pending      // Waiting
  in_progress  // In progress
  done         // Completed
  dismissed    // Dismissed
}
```

#### type: "issue_triage"

VCS provider issue triage task.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| issueUrl | string | Y | Issue URL |
| repositoryId | string | Y | Repository ID |
| issueTitle | string | Y | Issue title |
| suggestedLabels | string[] | N | AI suggested labels |
| suggestedAssignees | string[] | N | AI suggested assignees |

#### type: "pr_review"

PR review task.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| prUrl | string | Y | PR/MR URL |
| repositoryId | string | Y | Repository ID |
| prTitle | string | Y | PR title |
| changedFilesCount | number | Y | Number of changed files |
| aiSummary | string | N | AI analysis summary |

### Repository

A managed repository.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| vcsType | VCSType | Y | Version control system type |
| vcsProviderType | VCSProviderType | Y | VCS hosting provider type |
| remoteUrl | string | Y | Remote URL |
| name | string | Y | Repository name |
| localPath | string | Y | Local filesystem path to the repository |
| defaultBranch | string | Y | Default branch name (e.g., "main") |

### Workspace

A logical grouping of repositories for organizing work.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| name | string | Y | Workspace name |
| description | string | N | Optional description |
| createdAt | timestamp | Y | Creation time |
| updatedAt | timestamp | Y | Last update time |

Workspaces provide a way to organize repositories. A default workspace is automatically created when the app starts. Repositories can belong to multiple workspaces.

### RepositoryGroup

A group of repositories that tasks operate on.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| name | string | N | Group name (null for single-repo groups) |
| workspaceId | string | Y | Parent workspace ID |
| repositoryIds | string[] | Y | List of repository IDs in this group |
| createdAt | timestamp | Y | Creation time |
| updatedAt | timestamp | Y | Last update time |

Repository groups enable multi-repository task execution:
- **Single-repo groups**: When `name` is null, the group contains exactly one repository. The repository name is displayed as the group name.
- **Multi-repo groups**: When `name` is set, the group can contain multiple repositories.

Tasks (UnitTask, CompositeTask) reference a `repositoryGroupId` instead of a single `repositoryId`, allowing agents to work across multiple repositories simultaneously.

### TtyInputRequest

A TTY input request from an AI coding agent.

When an AI agent requires user input (e.g., asking a clarifying question or requesting confirmation), the system captures the request and notifies the user.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier for the request |
| taskId | string | Y | Associated UnitTask ID |
| sessionId | string | Y | Agent session ID |
| prompt | string | Y | The question or prompt from the agent |
| inputType | TtyInputType | Y | Type of input expected |
| options | string[] | N | Available options (for select type) |
| createdAt | timestamp | Y | When the request was created |
| status | TtyInputStatus | Y | Current status of the request |
| response | string | N | User's response (when answered) |
| respondedAt | timestamp | N | When the user responded |

#### TtyInputType

```
enum TtyInputType {
  text         // Free-form text input
  confirm      // Yes/No confirmation
  select       // Selection from options
}
```

#### TtyInputStatus

```
enum TtyInputStatus {
  pending      // Waiting for user response
  answered     // User has responded
  cancelled    // Request was cancelled (e.g., task stopped)
  expired      // Request timed out
}
```

---

## Configuration

DeliDev uses two levels of configuration: global settings and repository-specific settings.

### Global Settings

Location: `~/.delidev/config.toml` (or OS-specific data directory)

These settings are user-specific and not shared with the team.

```toml
[learning]
# Automatically learn from VCS provider reviews and update AI docs
autoLearnFromReviews = false

[hotkey]
# Global hotkey to open chat window (even when app is not focused)
openChat = "Option+Z"  # Alt+Z on Windows/Linux

[notification]
# Enable desktop notifications
enabled = true

# Notify when AI agent requests approval for a task or plan
approvalRequest = true

# Notify when AI agent asks a question
userQuestion = true

# Notify when AI work is complete and ready for review
reviewReady = true

[agent.planning]
# AI agent type for planning tasks (CompositeTask planning)
type = "claude_code"

# AI model for planning tasks
model = "claude-sonnet-4-20250514"

[agent.execution]
# AI agent type for execution tasks (UnitTask, auto-fix)
type = "claude_code"

# AI model for execution tasks
model = "claude-sonnet-4-20250514"

[agent.chat]
# AI agent type for chat interface
type = "claude_code"

# AI model for chat
model = "claude-sonnet-4-20250514"

[container]
# Container runtime to use: "docker" or "podman"
runtime = "docker"

# Whether to use container (Docker/Podman) for agent execution
# When false, agents run directly on the host without containerization
use_container = true

# Custom socket path (optional, uses default if not set)
# socket_path = "unix:///var/run/docker.sock"

[composite_task]
# Automatically approve composite task plans without user review
# When enabled, plans are approved and execution starts immediately after planning
auto_approve = false

[concurrency]
# Maximum number of concurrent agent sessions (premium feature, requires license)
# Leave unset for unlimited concurrent sessions
# max_concurrent_sessions = 3
```

#### Configuration Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| learning | autoLearnFromReviews | bool | false | Auto-learn from VCS provider reviews |
| hotkey | openChat | string | "Option+Z" | Global hotkey to open chat (Alt+Z on Windows/Linux) |
| notification | enabled | bool | true | Enable desktop notifications |
| notification | approvalRequest | bool | true | Notify when AI agent requests approval |
| notification | userQuestion | bool | true | Notify when AI agent asks a question |
| notification | reviewReady | bool | true | Notify when AI work is complete and ready for review |
| agent.planning | type | string | "claude_code" | AI agent type for planning tasks |
| agent.planning | model | string | "claude-sonnet-4-20250514" | AI model for planning tasks |
| agent.execution | type | string | "claude_code" | AI agent type for execution tasks |
| agent.execution | model | string | "claude-sonnet-4-20250514" | AI model for execution tasks |
| agent.chat | type | string | "claude_code" | AI agent type for chat interface |
| agent.chat | model | string | "claude-sonnet-4-20250514" | AI model for chat |
| container | runtime | string | "docker" | Container runtime: "docker" or "podman" |
| container | use_container | bool | true | Use container for agent execution. When false, runs directly on host |
| container | socket_path | string | (runtime default) | Custom container runtime socket path |
| composite_task | auto_approve | bool | false | Auto-approve composite task plans without user review |
| concurrency | max_concurrent_sessions | u32 | unlimited | Maximum concurrent agent sessions (premium feature, requires license) |

### Repository Settings

Location: `.delidev/config.toml` (committed to git, shared with team)

These settings are repository-specific and shared with the team via git.

```toml
# Docker image configuration is done via .delidev/setup/Dockerfile
# If no Dockerfile is present, the default image (node:20-slim) will be used.
# See .delidev/setup/Dockerfile for customizing the agent environment.

[branch]
# Branch name template (available variables: ${taskId}, ${slug})
template = "feature/${taskId}-${slug}"

[automation]
# Automatically apply review comments
autoFixReviewComments = true

# Filter for auto-fix review comments
# Options: "write_access_only" (default), "all"
autoFixReviewCommentsFilter = "write_access_only"

# Automatically fix CI failures
autoFixCIFailures = true

# Maximum number of auto-fix attempts
maxAutoFixAttempts = 3

[learning]
# Override global setting for this repository
autoLearnFromReviews = false

[composite_task]
# Override global auto_approve setting for this repository
auto_approve = true
```

#### Docker Environment

Docker image configuration is done via `.delidev/setup/Dockerfile`. If no Dockerfile is present, the default image (`node:20-slim`) will be used.

Example Dockerfile:
```dockerfile
FROM node:20

# Install pnpm globally
RUN npm config set prefix /tmp/npm-global && npm install -g pnpm

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add additional paths to PATH
ENV PATH="/tmp/npm-global/bin:/root/.cargo/bin:${PATH}"
```

#### Configuration Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| branch | template | string | "delidev/${taskId}" | Branch name template (variables: `${taskId}`, `${slug}`) |
| automation | autoFixReviewComments | bool | true | Auto-apply review comments |
| automation | autoFixReviewCommentsFilter | string | "write_access_only" | Filter for review comments: "write_access_only", "all" |
| automation | autoFixCIFailures | bool | true | Auto-fix CI failures |
| automation | maxAutoFixAttempts | int | 3 | Max auto-fix attempts |
| learning | autoLearnFromReviews | bool | (inherit) | Override global learning setting |
| composite_task | auto_approve | bool | (inherit) | Override global auto_approve setting for composite tasks |

### Configuration Precedence

1. Repository settings (`.delidev/config.toml`) take precedence
2. Global settings (`~/.delidev/config.toml`) are used as fallback
3. Built-in defaults are used if neither is set

---

## Authentication

DeliDev requires authentication for VCS providers. AI agent authentication uses locally stored credentials from tools like Claude Code.

### VCS Provider Authentication

VCS provider tokens are stored in `~/.delidev/credentials.toml`.

```toml
[github]
token = "ghp_xxxxxxxxxxxx"

[gitlab]
token = "glpat-xxxxxxxxxxxx"

[bitbucket]
username = "your-username"
app_password = "xxxxxxxxxxxx"
```

#### Required Permissions (Scopes)

| Provider | Required Scopes |
|----------|-----------------|
| GitHub | `repo`, `read:user`, `workflow` |
| GitLab | `api`, `read_user`, `read_repository`, `write_repository` |
| Bitbucket | Repository: Read/Write, Pull Requests: Read/Write |

### AI Agent Authentication

AI agent authentication uses locally stored credentials from the AI coding agent itself (e.g., Claude Code's `~/.claude/` settings). No additional configuration is required in DeliDev.

### License Management

DeliDev uses Polar.sh for license key management. Licenses are stored locally in `~/.delidev/license.toml`.

```toml
# License credentials (auto-generated, do not edit manually)
key = "DELI-XXXX-XXXX-XXXX"
activation_id = "act_xxxxxxxxxxxxxxxx"
device_label = "hostname (platform)"
```

#### Pricing

- **$4/month** per user
- Team membership support for combined billing via Polar.sh

#### License Status

| Status | Description |
|--------|-------------|
| active | License is valid and active |
| expired | License has expired |
| invalid | License key is invalid |
| revoked | License has been revoked |
| pending | License validation is pending |
| not_configured | No license configured |

#### License Operations

| Operation | Description |
|-----------|-------------|
| Activate | Register a license key for this device |
| Validate | Check current license status with Polar.sh |
| Deactivate | Remove license activation from this device |
| Remove | Clear stored license credentials |

---

### AI Document Auto-Update

When `autoLearnFromReviews` is enabled:

1. A user with write access to the repository leaves a review
2. The system extracts learning points from the review
3. The system requests approval from the local user
4. Upon approval, the feedback is added to `CLAUDE.md` or `AGENTS.md`

This feature can be:
- Enabled/disabled globally via global settings
- Overridden per repository via repository settings
- Always requires local user approval before updating docs

---

## Workflows

Main workflows in DeliDev.

### UnitTask Execution Flow

```
┌──────────────┐
│  User creates │
│   UnitTask    │
└──────┬───────┘
       ▼
┌──────────────┐
│ Create git   │
│  worktree    │
└──────┬───────┘
       ▼
┌──────────────┐
│ Start Docker │
│  container   │
└──────┬───────┘
       ▼
┌──────────────┐
│  Run AI      │
│   Agent      │
│ (in_progress)│
└──────┬───────┘
       ▼
┌──────────────┐
│ AI work done │
│ (in_review)  │
└──────┬───────┘
       ▼
┌──────────────┐
│ Human review │◄─────────────────────┐
│ (self UI)    │                      │
└──────┬───────┘                      │
       │                              │
       ├─── Commit to ──►┌──────────────┐
       │   Repository    │   Merged     │
       │                 │   (done)     │
       │                 └──────────────┘
       │
       ├─── Create PR ──►┌──────────────┐
       │                 │ PR Created   │
       │                 │   (done)     │
       │                 └──────────────┘
       │
       ├─── Request ──►┌──────────────┐
       │   Changes     │ AI rework    │
       │               └──────┬───────┘
       │                      │
       │                      └───────────┘
       │
       └─── Reject ───►┌──────────────┐
                       │  Discarded   │
                       │ (rejected)   │
                       └──────────────┘
```

#### Status Transitions

| From | To | Trigger |
|------|-----|---------|
| - | in_progress | Task created |
| in_progress | in_review | AI completes work |
| in_review | in_progress | User requests changes |
| in_review | done | User commits to repository |
| in_review | done | User creates PR |
| in_review | rejected | User rejects |

### CompositeTask Execution Flow

```
┌────────────────────┐
│  User creates      │
│  CompositeTask     │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  Create temporary  │
│  git worktree for  │
│  planning          │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  planningTask      │
│  generates         │
│  PLAN-{random}.yaml│
│  (in worktree)     │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  Copy plan file    │
│  to main repo,     │
│  cleanup worktree  │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  User reviews      │
│  and approves      │
│  PLAN-{random}.yaml│
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  Execute tasks     │
│  (parallel where   │
│   possible)        │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  All tasks done    │
│  CompositeTask     │
│  complete          │
└────────────────────┘
```

#### Planning Worktree Isolation

CompositeTask planning uses a temporary git worktree to ensure the main repository is not affected during the planning phase:

1. **Worktree Creation**: A temporary worktree is created at `/tmp/delidev/planning/{compositeTaskId}` from the repository's default branch
2. **Plan Generation**: The planning agent runs in this isolated worktree and creates the `PLAN-{random}.yaml` file
3. **Plan File Copy**: After successful planning, the plan file is copied from the worktree to the main repository
4. **Worktree Cleanup**: The temporary worktree is removed regardless of success or failure

This isolation ensures that any temporary files or changes made by the planning agent do not pollute the main repository.

#### Parallel Execution

When a CompositeTask is approved:

1. All nodes with empty `dependsOn` start executing in parallel
2. When a node completes, check all other nodes
3. If a node's `dependsOn` is fully satisfied, start it
4. Repeat until all nodes are complete

Example:
```yaml
# PLAN-{randomString}.yaml
tasks:
  - id: "setup-db"
    prompt: "Set up database schema"
    # No dependsOn - starts immediately

  - id: "setup-auth"
    prompt: "Set up authentication"
    # No dependsOn - starts immediately (parallel with setup-db)

  - id: "api-endpoints"
    prompt: "Implement API endpoints"
    dependsOn: ["setup-db", "setup-auth"]
    # Waits for both setup-db and setup-auth

  - id: "frontend"
    prompt: "Implement frontend"
    dependsOn: ["api-endpoints"]
    # Waits for api-endpoints
```

Execution order:
```
Time 0: setup-db ─────┐
        setup-auth ───┼──► api-endpoints ──► frontend
```

### PR Auto-Management

#### PR Creation via AI Agent

When a user creates a PR for a task, the system:

1. **Detects Existing PRs**: First checks if a PR already exists for the task's branch
   - Uses VCS provider API to search for open PRs by head branch name
   - If found, associates the existing PR with the task

2. **AI Agent PR Creation**: If no PR exists, uses an AI coding agent to create one
   - Creates an AgentTask to execute in the task's worktree
   - The agent pushes the branch and creates the PR using `gh pr create`
   - PR URL is extracted from agent output or detected via API

3. **PR Association**: The created/found PR URL is stored in the UnitTask
   - Task status transitions to `pr_open`
   - Worktree is cleaned up after successful PR creation

This approach allows the AI agent to craft intelligent PR descriptions and handle any necessary git operations autonomously.

#### Auto-Fix Review Comments

When enabled (`automation.autoFixReviewComments = true`):

1. PR/MR receives a review comment
2. System checks the comment author against `autoFixReviewCommentsFilter`:
   - `write_access_only` (default): Only applies comments from users with write permission (excludes bots)
   - `all`: Applies all comments including bots
3. Creates new AgentTask to address the feedback
4. AgentTask added to UnitTask's `autoFixTasks`
5. AI applies the fix and pushes
6. Repeat up to `maxAutoFixAttempts`

#### Auto-Fix CI Failures

When enabled (`automation.autoFixCIFailures = true`):

1. CI fails on PR/MR
2. System detects the failure
3. Creates new AgentTask to fix the issue
4. AgentTask added to UnitTask's `autoFixTasks`
5. AI analyzes logs, fixes, and pushes
6. Repeat up to `maxAutoFixAttempts`

#### Rate Limiting

- `maxAutoFixAttempts` prevents infinite loops
- Each auto-fix attempt is tracked in `autoFixTasks`
- When limit reached, task requires manual intervention

### TTY Input Proxy

When an AI coding agent requires user input during execution:

```
┌────────────────────────────────────────────────────────────┐
│                    Agent Execution                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Agent outputs TTY input request                      │  │
│  │  (e.g., "Which database should I use?")               │  │
│  └────────────────────────┬─────────────────────────────┘  │
│                           │                                 │
│                           ▼                                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  TTY Input Proxy Service intercepts request           │  │
│  │  - Pauses agent execution                             │  │
│  │  - Creates TtyInputRequest entity                     │  │
│  │  - Emits "tty-input-request" event to frontend        │  │
│  └────────────────────────┬─────────────────────────────┘  │
└───────────────────────────┼─────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                    Desktop Notification                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  🤖 DeliDev - Agent Question                           │  │
│  │                                                        │  │
│  │  "Which database should I use?"                        │  │
│  │                                                        │  │
│  │  Click to respond                                      │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    Task Detail Page                           │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  TTY Input Request                                     │  │
│  │  ─────────────────────────────────────────             │  │
│  │                                                        │  │
│  │  Agent is asking: "Which database should I use?"       │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │ PostgreSQL                                       │  │  │
│  │  │ MySQL                                            │  │  │
│  │  │ SQLite                                           │  │  │
│  │  │ [Custom response...]                             │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                        │  │
│  │                              [Cancel]    [Submit]      │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    Response Handling                          │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  - Frontend calls submitTtyInput command               │  │
│  │  - Backend writes response to agent's stdin            │  │
│  │  - Agent execution resumes                             │  │
│  │  - TtyInputRequest status updated to "answered"        │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

#### TTY Input Detection

The system detects TTY input requests by monitoring agent output for specific patterns:

1. **Claude Code**: Detects `AskUserQuestion` tool use events in stream-json output
2. **OpenCode**: Detects question events in JSON output

#### Event Flow

| Step | Component | Action |
|------|-----------|--------|
| 1 | Agent | Outputs TTY input request |
| 2 | TTY Proxy Service | Intercepts and pauses execution |
| 3 | TTY Proxy Service | Emits "tty-input-request" event |
| 4 | Frontend | Shows notification and input dialog |
| 5 | User | Submits response via web form |
| 6 | Frontend | Calls "submit_tty_input" command |
| 7 | TTY Proxy Service | Writes response to agent stdin |
| 8 | Agent | Receives response and continues |

### AI Document Learning

DeliDev supports two modes of AI document learning:

#### 1. Request Changes Learning (Automatic)

When a user requests changes on a UnitTask (via "Request Changes" action):

```
┌──────────────────┐
│ User requests    │
│ changes on task  │
│ (InReview state) │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ Feedback appended│
│ to task prompt   │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ AI agent re-runs │
│ with learning    │
│ instructions     │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ Agent considers  │
│ if feedback is   │
│ generalizable    │
└────────┬─────────┘
         │
         ├── Yes ───► Update AGENTS.md / CLAUDE.md
         │            (during task execution)
         │
         └── No ────► Apply fix only
```

The agent automatically receives instructions to:
- Consider whether feedback represents a general guideline
- Add code style preferences to `AGENTS.md`
- Add architecture patterns to `AGENTS.md`
- Add AI-specific behaviors to `CLAUDE.md`
- Only add truly generalizable rules, not task-specific corrections

#### 2. VCS Provider Review Learning (Future)

When a reviewer (with write access) leaves feedback on VCS provider:

```
┌──────────────────┐
│ Review comment   │
│ on PR/MR         │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ Extract learning │
│ points           │
└────────┬─────────┘
         ▼
┌──────────────────┐
│ Request local    │
│ user approval    │
└────────┬─────────┘
         │
         ├── Approve ──► Update CLAUDE.md / AGENTS.md
         │
         └── Reject ───► Discard
```

This enables continuous improvement of AI behavior based on human feedback.

---

## Custom Commands

DeliDev supports custom commands (also known as slash commands) that allow users to define reusable prompt templates. Commands are automatically discovered based on the AI agent being used.

### Command Discovery

Custom commands are discovered from the following locations, depending on the agent framework:

#### Claude Code
- **Project commands**: `.claude/commands/` (committed to repository, shared with team)
- **Global commands**: `~/.claude/commands/` (user-specific, available across all projects)

#### OpenCode
- **Project commands**: `.opencode/command/` (committed to repository, shared with team)
- **Global commands**: `~/.config/opencode/command/` (user-specific, available across all projects)

### Command Format

Commands are defined as Markdown files with optional YAML frontmatter:

```markdown
---
description: Create a new React component
agent: claude_code
model: claude-sonnet-4-20250514
---
Create a new React component called $1 with the following features:
- TypeScript support
- Unit tests
- Storybook story

$ARGUMENTS
```

### Frontmatter Options

| Option | Type | Description |
|--------|------|-------------|
| `description` | string | Brief description shown in command list |
| `agent` | string | Agent type override (e.g., `claude_code`, `open_code`) |
| `model` | string | Model override for this command |
| `subtask` | boolean | Run as a subtask (OpenCode style) |
| `context` | string | Set to `fork` for sub-agent context (Claude Code style) |

### Argument Placeholders

Commands support the following placeholders:

| Placeholder | Description | Example |
|-------------|-------------|---------|
| `$ARGUMENTS` | All arguments as a single string | `fix-issue 123 high` → `123 high` |
| `$1`, `$2`, etc. | Individual positional arguments | `$1` = `123`, `$2` = `high` |

### Command Precedence

When the same command name exists in multiple locations:

1. **Project commands** take precedence over global commands
2. Within the same scope, the first match wins

### Namespace Support

Commands in subdirectories gain a namespace prefix in their display name:

```
.claude/commands/frontend/component.md  →  component (project:frontend)
.claude/commands/backend/api.md         →  api (project:backend)
```

---

## Branch Strategy

DeliDev creates isolated branches for each task using git worktree.

### Branch Naming

Branch names can be configured using templates or specified directly per task.

#### Template Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `${taskId}` | Unique task identifier | `abc123` |
| `${slug}` | URL-safe task description | `add-user-auth` |

#### Configuration

Set the branch name template in repository settings (`.delidev/config.toml`):

```toml
[branch]
template = "feature/${taskId}-${slug}"
```

Example output: `feature/abc123-add-user-auth`

#### Direct Specification

When creating a UnitTask, you can directly specify the branch name via the `branchName` field. This overrides the template.

#### Priority Order

1. **UnitTask.branchName** - If specified, use this value directly
2. **Template** - If configured in repository settings
3. **Default** - `delidev/${taskId}`

### Git Worktree Management

For each task:

1. **Creation**: A new worktree is created from the base branch
2. **Isolation**: Each worktree is independent, allowing parallel task execution
3. **Cleanup**: Worktrees are automatically removed after task completion or rejection

---

## Error Handling

Common error scenarios and their resolutions.

### Docker Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| Docker daemon not running | Docker Desktop is not started | Start Docker Desktop |
| Image pull failed | Network issues or invalid image name | Check network connection, verify `baseImage` in config |
| Container start failed | Resource constraints or port conflicts | Check Docker resource settings, verify no port conflicts |

### VCS Provider Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| Authentication failed | Invalid or expired token | Update token in `~/.delidev/credentials.toml` |
| Permission denied | Insufficient token scopes | Verify token has required scopes (see [Authentication](#authentication)) |
| Rate limit exceeded | Too many API requests | Wait for rate limit reset or use different token |
| PR creation failed | Branch protection rules | Check repository branch protection settings |

### AI Agent Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| Agent authentication failed | Invalid or missing API key | Ensure AI agent (e.g., Claude Code) is properly configured |
| Model not available | Invalid model name or API issues | Verify model name in settings, check AI provider status |
| Context limit exceeded | Task description too long | Simplify task description or split into smaller tasks |

### Network Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| Connection timeout | Network issues | Check internet connection |
| SSL certificate error | Certificate issues | Update system certificates |
| Proxy error | Proxy misconfiguration | Configure proxy settings if required |

