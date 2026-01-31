# DeliDev Design Document

DeliDev is a desktop and mobile application for orchestrating AI coding agents with support for both local and remote execution.

## Table of Contents

1. [Architecture](#architecture)
2. [Technology Stack](#technology-stack)
3. [Entities](#entities)
4. [Configuration](#configuration)
5. [Authentication](#authentication)
6. [Secrets Management](#secrets-management)
7. [Workflows](#workflows)
8. [Error Handling](#error-handling)
9. [Related Documents](#related-documents)

---

## Architecture

### Component Roles

DeliDev consists of three main components:

| Component | Role |
|-----------|------|
| **Main Server** | Maintains the task list, provides RPC server, coordinates workers |
| **Worker Server** | Runs AI coding agents (Claude Code, OpenCode, etc.) in Docker sandboxes |
| **Client** | Desktop/Mobile app based on Tauri, provides user interface |

### Distributed Architecture

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

**Key Characteristics:**

- **Main Server**: Central hub for task management and worker coordination
- **Worker Servers**: Execute AI agents in isolated Docker containers
- **Client**: Never communicates directly with Worker Server (always through Main Server)

### Single Process Mode

For desktop usage, all components can run in a single process for a seamless local experience:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Single Process Desktop App                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐  │
│  │   Embedded Server   │  │   Embedded Worker   │  │      Client UI      │  │
│  │                     │  │                     │  │   (Tauri WebView)   │  │
│  │                     │  │                     │  │                     │  │
│  │  In-process calls   │◄─┤  In-process calls   │◄─┤   In-process calls  │  │
│  │  (no network)       │  │  (no network)       │  │   (no network)      │  │
│  └─────────────────────┘  └─────────────────────┘  └─────────────────────┘  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          SQLite Database                                 ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                             │
│  Auth: DISABLED (single user, trusted local execution)                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Features:**

- No network overhead - all RPC calls are direct function invocations
- SQLite storage for local data
- Authentication disabled (trusted local execution)
- Secrets read directly from local keychain

### Communication

| Layer | Protocol |
|-------|----------|
| Client ↔ Main Server | JSON-RPC over HTTP/WebSocket |
| Frontend State | react-query for data fetching |
| Client ↔ Worker | **Not allowed** - all communication goes through Main Server |

---

## Technology Stack

### Desktop/Mobile Framework

- **Tauri**: Rust-based framework using system WebView
  - Small binary size (~10MB)
  - Low memory footprint
  - Native system integration
  - Cross-platform: Desktop (Windows, macOS, Linux) and Mobile (iOS, Android)

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
| `coding_agents` | AI agent abstraction, output normalization, Docker sandboxing |
| `task_store` | Task storage (SQLite, PostgreSQL, in-memory) |
| `rpc_protocol` | JSON-RPC 2.0 protocol definitions |
| `git_ops` | Git operations & worktree management |
| `auth` | JWT authentication & RBAC |
| `secrets` | Cross-platform keychain access |

### Frontend (React + TypeScript)

| Package | Purpose |
|---------|---------|
| react | UI framework |
| typescript | Type safety |
| @rspack/core | Build tool (Rust-based) |
| tailwindcss | Utility-first CSS |
| shadcn/ui | Component library |
| zustand | State management |
| @tanstack/react-query | Server state management |

### Coding Agent Normalization

The `coding_agents` crate normalizes output from all AI coding agents:

```
┌─────────────────────────────────────────────────────────────────┐
│                        coding_agents crate                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │Claude Code  │  │  OpenCode   │  │   Aider     │  ...        │
│  │   Parser    │  │   Parser    │  │   Parser    │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┴────────────────┘                     │
│                          │                                      │
│                          ▼                                      │
│         ┌────────────────────────────────────┐                  │
│         │      Normalized Event Types        │                  │
│         │  - ToolUse, ToolResult             │                  │
│         │  - TextOutput, ErrorOutput         │                  │
│         │  - FileChange, CommandExecution    │                  │
│         │  - AskUserQuestion, etc.           │                  │
│         └────────────────────────────────────┘                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Frontend uses only normalized types
- Easy to add new AI agents by implementing a parser
- Consistent UI rendering regardless of agent type

---

## Entities

### VCSType

```
enum VCSType {
  git           // Git
}
```

### VCSProviderType

```
enum VCSProviderType {
  github        // GitHub
  gitlab        // GitLab
  bitbucket     // Bitbucket
}
```

### AIAgentType

```
enum AIAgentType {
  claude_code    // Claude Code - Anthropic's terminal-based agentic coding tool
  open_code      // OpenCode - Open-source Claude Code alternative
  gemini_cli     // Gemini CLI - Google's open-source AI agent
  codex_cli      // Codex CLI - OpenAI's terminal-based coding assistant
  aider          // Aider - Open-source CLI for multi-file changes
  amp            // Amp - Sourcegraph's agentic coding CLI
}
```

### AgentSession

A single AI coding agent session.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| aiAgentType | AIAgentType | Y | Agent type |
| aiAgentModel | string | N | Model to use |

### AgentTask

A collection of AgentSessions. The retryable unit.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| baseRemotes | BaseRemote[] | Y | Git repository information |
| agentSessions | AgentSession[] | Y | Session list |
| aiAgentType | AIAgentType | N | Agent type |
| aiAgentModel | string | N | Model to use |

### UnitTask

A single task unit visible to users.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| repositoryGroupId | string | Y | Associated RepositoryGroup ID |
| agentTask | AgentTask | Y | Associated AgentTask (1:1) |
| branchName | string | N | Custom branch name |
| linkedPrUrl | string | N | Created PR URL |
| baseCommit | string | N | Base commit hash |
| endCommit | string | N | End commit hash |
| autoFixTasks | AgentTask[] | Y | Auto-fix attempts |
| status | UnitTaskStatus | Y | Current status |

#### UnitTaskStatus

```
enum UnitTaskStatus {
  in_progress   // AI is working
  in_review     // AI work complete, awaiting human review
  approved      // Human approved
  pr_open       // PR created
  done          // PR merged
  rejected      // Rejected and discarded
}
```

### CompositeTask

Task graph-based Agent Orchestrator.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| repositoryGroupId | string | Y | Associated RepositoryGroup ID |
| planningTask | AgentTask | Y | AgentTask for generating PLAN.yaml |
| tasks | CompositeTaskNode[] | Y | List of task nodes |
| status | CompositeTaskStatus | Y | Current status |
| executionAgentType | AIAgentType | N | Agent type for UnitTasks |

#### CompositeTaskStatus

```
enum CompositeTaskStatus {
  planning           // Generating PLAN.yaml
  pending_approval   // Waiting for user approval
  in_progress        // Tasks are executing
  done               // All tasks completed
  rejected           // User rejected the plan
}
```

### CompositeTaskNode

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| unitTask | UnitTask | Y | Associated UnitTask |
| dependsOn | CompositeTaskNode[] | Y | Dependent nodes |

### TodoItem

Tasks that humans should do but AI can assist with.

#### type: "issue_triage"

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| issueUrl | string | Y | Issue URL |
| repositoryId | string | Y | Repository ID |
| issueTitle | string | Y | Issue title |
| suggestedLabels | string[] | N | AI suggested labels |
| suggestedAssignees | string[] | N | AI suggested assignees |

#### type: "pr_review"

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| prUrl | string | Y | PR/MR URL |
| repositoryId | string | Y | Repository ID |
| prTitle | string | Y | PR title |
| changedFilesCount | number | Y | Changed files count |
| aiSummary | string | N | AI analysis summary |

### Repository

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| vcsType | VCSType | Y | Version control system |
| vcsProviderType | VCSProviderType | Y | VCS provider |
| remoteUrl | string | Y | Remote URL |
| name | string | Y | Repository name |
| localPath | string | Y | Local filesystem path |
| defaultBranch | string | Y | Default branch |

### Workspace

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| name | string | Y | Workspace name |
| description | string | N | Description |
| createdAt | timestamp | Y | Creation time |
| updatedAt | timestamp | Y | Last update time |

### RepositoryGroup

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| name | string | N | Group name |
| workspaceId | string | Y | Parent workspace ID |
| repositoryIds | string[] | Y | Repository IDs |
| createdAt | timestamp | Y | Creation time |
| updatedAt | timestamp | Y | Last update time |

### TtyInputRequest

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier |
| taskId | string | Y | Associated UnitTask ID |
| sessionId | string | Y | Agent session ID |
| prompt | string | Y | Question from agent |
| inputType | TtyInputType | Y | Input type |
| options | string[] | N | Available options |
| createdAt | timestamp | Y | Creation time |
| status | TtyInputStatus | Y | Current status |
| response | string | N | User's response |
| respondedAt | timestamp | N | Response time |

---

## Configuration

### Global Settings

Location: `~/.delidev/config.toml`

```toml
[learning]
autoLearnFromReviews = false

[hotkey]
openChat = "Option+Z"

[notification]
enabled = true
approvalRequest = true
userQuestion = true
reviewReady = true

[agent.planning]
type = "claude_code"
model = "claude-sonnet-4-20250514"

[agent.execution]
type = "claude_code"
model = "claude-sonnet-4-20250514"

[agent.chat]
type = "claude_code"
model = "claude-sonnet-4-20250514"

[container]
runtime = "docker"
use_container = true

[composite_task]
auto_approve = false

[concurrency]
# max_concurrent_sessions = 3
```

### Repository Settings

Location: `.delidev/config.toml` (committed to git)

```toml
[branch]
template = "feature/${taskId}-${slug}"

[automation]
autoFixReviewComments = true
autoFixReviewCommentsFilter = "write_access_only"
autoFixCIFailures = true
maxAutoFixAttempts = 3

[learning]
autoLearnFromReviews = false

[composite_task]
auto_approve = true
```

### Configuration Precedence

1. Repository settings take precedence
2. Global settings as fallback
3. Built-in defaults if neither is set

---

## Authentication

### Server Authentication (Remote Mode)

In remote mode, the server requires authentication for all API requests.

#### JWT Authentication

- Tokens issued after successful OIDC authentication
- Includes user ID, email, and name claims
- Default expiration: 24 hours

| Variable | Description | Default |
|----------|-------------|---------|
| `DELIDEV_JWT_SECRET` | Secret key for signing JWTs | (required) |
| `DELIDEV_JWT_EXPIRATION_HOURS` | Token expiration | 24 |
| `DELIDEV_JWT_ISSUER` | JWT issuer claim | "delidev" |

#### OpenID Connect (OIDC)

Supports authentication with any OIDC provider (Google, GitHub, Keycloak, etc.).

| Variable | Description |
|----------|-------------|
| `DELIDEV_OIDC_ISSUER_URL` | OIDC provider issuer URL |
| `DELIDEV_OIDC_CLIENT_ID` | OAuth2 client ID |
| `DELIDEV_OIDC_CLIENT_SECRET` | OAuth2 client secret |
| `DELIDEV_OIDC_REDIRECT_URL` | Redirect URL after authentication |
| `DELIDEV_OIDC_SCOPES` | Comma-separated scopes |

**Security Features:**
- PKCE (S256 challenge method)
- CSRF protection via state parameter
- Database-backed state storage
- Redirect URI validation

#### Single Process Mode

**Authentication is DISABLED** in single process mode. This mode is intended for local desktop usage where all requests are trusted.

### VCS Provider Authentication

Stored in `~/.delidev/credentials.toml`:

```toml
[github]
token = "ghp_xxxxxxxxxxxx"

[gitlab]
token = "glpat-xxxxxxxxxxxx"

[bitbucket]
username = "your-username"
app_password = "xxxxxxxxxxxx"
```

---

## Secrets Management

Secrets are stored in the native system keychain and transported to workers when needed.

### Known Secret Keys

| Key | Description | Used By |
|-----|-------------|---------|
| `CLAUDE_CODE_OAUTH_TOKEN` | Claude Code OAuth token | Claude Code |
| `ANTHROPIC_API_KEY` | Anthropic API key | Claude Code, Amp |
| `OPENAI_API_KEY` | OpenAI API key | OpenCode, Aider, Codex CLI |
| `GOOGLE_AI_API_KEY` | Google AI API key | Gemini CLI |
| `GITHUB_TOKEN` | GitHub access token | All agents |

### Secret Storage

| Platform | Backend |
|----------|---------|
| macOS | Keychain Services |
| Windows | Windows Credential Manager |
| Linux | Secret Service (libsecret/KWallet) |

### Client-to-Worker Transport

```
┌─────────────┐    1. Client reads    ┌─────────────┐
│   Client    │    secrets from       │   Native    │
│   (Tauri)   │◄───────────────────── │  Keychain   │
└──────┬──────┘    local keychain     └─────────────┘
       │
       │ 2. Client sends secrets
       │    via sendSecrets RPC
       ▼
┌─────────────┐
│   Server    │    3. Server stores
│   (Main)    │    secrets temporarily
└──────┬──────┘    (in-memory, per-task)
       │
       │ 4. Server relays secrets
       │    when task starts
       ▼
┌─────────────┐
│   Worker    │    5. Worker injects
│             │    secrets as env vars
└─────────────┘
```

**Security:**
- Secrets encrypted at rest (native keychain)
- TLS for transport (HTTPS/WSS)
- Temporary in-memory storage on server
- Cleared after task completion
- Single-process mode reads directly from local keychain

---

## Workflows

### UnitTask Execution Flow

```
User creates UnitTask
        ▼
Create git worktree
        ▼
Start Docker container (on Worker)
        ▼
Run AI Agent (in_progress)
        ▼
AI work done (in_review)
        ▼
Human review ──┬──► Commit to repo (done)
               ├──► Create PR (pr_open → done)
               ├──► Request changes (back to in_progress)
               └──► Reject (rejected)
```

### CompositeTask Execution Flow

```
User creates CompositeTask
        ▼
planningTask generates PLAN.yaml
        ▼
User reviews and approves
        ▼
Execute tasks (parallel where possible)
        ▼
All tasks done
```

### PR Auto-Management

#### Auto-Fix Review Comments

When enabled:
1. PR receives review comment
2. Check author against filter
3. Create AgentTask to address feedback
4. AI applies fix and pushes
5. Repeat up to `maxAutoFixAttempts`

#### Auto-Fix CI Failures

When enabled:
1. CI fails on PR
2. Create AgentTask to fix
3. AI analyzes logs, fixes, pushes
4. Repeat up to `maxAutoFixAttempts`

### TTY Input Proxy

When an AI agent requires user input:

1. Agent outputs TTY input request
2. TTY Proxy Service intercepts and pauses execution
3. Desktop notification sent
4. User responds via web form
5. Response written to agent stdin
6. Execution resumes

### AI Document Learning

When user requests changes on a task:

1. Feedback appended to task prompt
2. AI agent re-runs with learning instructions
3. Agent considers if feedback is generalizable
4. If yes, updates AGENTS.md or CLAUDE.md

---

## Error Handling

### Docker Errors

| Error | Resolution |
|-------|------------|
| Docker daemon not running | Start Docker Desktop |
| Image pull failed | Check network, verify image name |
| Container start failed | Check resources, verify ports |

### VCS Provider Errors

| Error | Resolution |
|-------|------------|
| Authentication failed | Update token in credentials.toml |
| Permission denied | Verify token scopes |
| Rate limit exceeded | Wait for reset |
| PR creation failed | Check branch protection rules |

### Network Errors

| Error | Resolution |
|-------|------------|
| Connection timeout | Check internet connection |
| SSL certificate error | Update system certificates |

---

## Related Documents

- [Main Server](./main-server.md) - Main Server details
- [Worker Server](./worker-server.md) - Worker Server details
- [Tauri App](./tauri-app.md) - Desktop/Mobile app details
- [Local vs Remote Mode](./client-local-vs-remote.md) - Mode comparison
- [UI Design](./ui.md) - User interface specifications
- [PLAN.yaml Specification](./plan-yaml.md) - Task plan format
