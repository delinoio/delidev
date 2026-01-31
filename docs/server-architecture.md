# Server/Worker/Client Architecture Implementation Plan

This document outlines the comprehensive plan to transform DeliDev from a local-first desktop application into a distributed server/worker/client architecture.

## Table of Contents

1. [Overview](#overview)
2. [Architecture Components](#architecture-components)
3. [Crate Structure](#crate-structure)
4. [Phase 1: Extract Core Logic into Shared Crates](#phase-1-extract-core-logic-into-shared-crates)
5. [Phase 2: Implement Main Server](#phase-2-implement-main-server)
6. [Phase 3: Implement Worker Server](#phase-3-implement-worker-server)
7. [Phase 4: Implement Client](#phase-4-implement-client)
8. [Phase 5: Single Process Mode](#phase-5-single-process-mode)
9. [Phase 6: Authentication](#phase-6-authentication)
10. [Phase 7: Secrets Management](#phase-7-secrets-management)
11. [Communication Protocol](#communication-protocol)
12. [Migration Strategy](#migration-strategy)

---

## Overview

### Goals

1. **Remote Execution**: Enable AI coding agents to run on remote servers, preserving local CPU/memory.
2. **Mobile Support**: Allow coding from mobile apps (iOS/Android) via Tauri.
3. **Single Process Mode**: Desktop app remains simple to use as a single process.
4. **Normalization**: All AI agent data is normalized through the `coding_agents` crate.

### Architecture Diagram

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

### Single Process Mode

In single-client mode (desktop app running locally):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Single Process Desktop App                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐  │
│  │    Main Server      │  │   Worker Server     │  │      Client UI      │  │
│  │    (embedded)       │  │   (embedded)        │  │   (Tauri WebView)   │  │
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

---

## Architecture Components

### Main Server

**Responsibilities:**
- Maintain the list of tasks (UnitTask, CompositeTask, AgentTask, AgentSession)
- Manage workspaces, repositories, and repository groups
- Handle user authentication (JWT via OpenID Connect)
- Coordinate worker assignment for task execution
- Store and serve execution logs
- Manage configuration and credentials

**Technology:**
- Rust with Axum or Actix-web for HTTP/WebSocket server
- JSON-RPC 2.0 over HTTP and WebSocket
- PostgreSQL for multi-user mode, SQLite for single-user mode
- JWT authentication with OpenID Connect integration

### Worker Server

**Responsibilities:**
- Execute AI coding agents (Claude Code, OpenCode, Aider, etc.)
- Manage Docker containers for sandboxed execution
- Stream execution logs back to Main Server
- Handle secrets received from clients (via Main Server relay)

**Technology:**
- Rust-based service
- Uses `coding_agents` crate for all AI agent interactions
- Docker/Podman for sandboxing
- Registers with Main Server via heartbeat

### Client

**Responsibilities:**
- Provide UI for task management, code review, and settings
- Send local secrets (from keychain) to Main Server for relay to workers
- Display real-time execution logs
- **Never communicate directly with Worker Server**

**Technology:**
- Tauri (Rust + WebView) for Desktop and Mobile
- React + TypeScript frontend
- react-query for data fetching and caching
- JSON-RPC client for communication with Main Server

---

## Crate Structure

Following the requirement that crate names should **not** have a common prefix:

```
crates/
├── coding_agents/      # AI coding agent abstraction and Docker sandboxing
│   ├── src/
│   │   ├── lib.rs
│   │   ├── agent.rs         # Agent trait and types
│   │   ├── claude_code.rs   # Claude Code implementation
│   │   ├── opencode.rs      # OpenCode implementation
│   │   ├── aider.rs         # Aider implementation
│   │   ├── gemini_cli.rs    # Gemini CLI implementation
│   │   ├── codex_cli.rs     # Codex CLI implementation
│   │   ├── amp.rs           # Amp implementation
│   │   ├── docker.rs        # Docker sandboxing
│   │   ├── output.rs        # Normalized output types
│   │   └── stream.rs        # Stream message normalization
│   └── Cargo.toml
│
├── task_store/         # Task storage and management
│   ├── src/
│   │   ├── lib.rs
│   │   ├── entities/        # Task, Session, Repository entities
│   │   ├── sqlite.rs        # SQLite implementation
│   │   ├── postgres.rs      # PostgreSQL implementation
│   │   └── memory.rs        # In-memory implementation (for tests)
│   └── Cargo.toml
│
├── rpc_protocol/       # JSON-RPC protocol definitions
│   ├── src/
│   │   ├── lib.rs
│   │   ├── methods.rs       # RPC method definitions
│   │   ├── types.rs         # Request/response types
│   │   ├── error.rs         # Error types
│   │   └── client.rs        # JSON-RPC client
│   └── Cargo.toml
│
├── auth/               # Authentication and authorization
│   ├── src/
│   │   ├── lib.rs
│   │   ├── jwt.rs           # JWT token handling
│   │   ├── oidc.rs          # OpenID Connect integration
│   │   └── middleware.rs    # Auth middleware for servers
│   └── Cargo.toml
│
├── secrets/            # Secret management
│   ├── src/
│   │   ├── lib.rs
│   │   ├── keychain.rs      # macOS/Windows keychain access
│   │   ├── transport.rs     # Secure secret transport
│   │   └── env.rs           # Environment variable injection
│   └── Cargo.toml
│
└── git_ops/            # Git operations (extracted from current services/git.rs)
    ├── src/
    │   ├── lib.rs
    │   ├── worktree.rs      # Git worktree management
    │   ├── branch.rs        # Branch operations
    │   └── diff.rs          # Diff generation
    └── Cargo.toml
```

### Application Crates

```
apps/
├── desktop/            # Tauri desktop/mobile app (client)
│   ├── src-tauri/
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── commands/    # Tauri commands
│   │   │   └── single_process.rs  # Single process mode orchestration
│   │   └── Cargo.toml
│   └── src/            # React frontend
│
├── server/             # Main Server binary
│   ├── src/
│   │   ├── main.rs
│   │   ├── rpc/            # JSON-RPC handlers
│   │   ├── websocket/      # WebSocket handlers
│   │   ├── auth/           # Auth middleware integration
│   │   └── worker_registry.rs  # Worker management
│   └── Cargo.toml
│
└── worker/             # Worker Server binary
    ├── src/
    │   ├── main.rs
    │   ├── executor.rs     # Task execution
    │   ├── heartbeat.rs    # Heartbeat to main server
    │   └── secret_handler.rs  # Secret injection
    └── Cargo.toml
```

---

## Phase 1: Extract Core Logic into Shared Crates

### 1.1 Create `coding_agents` Crate

Extract and normalize AI agent code from current `apps/desktop/src-tauri/src/services/agent_execution.rs`.

**Key Components:**

```rust
// crates/coding_agents/src/lib.rs

/// Trait for AI coding agents
pub trait CodingAgent: Send + Sync {
    /// Execute the agent with the given prompt
    async fn execute(
        &self,
        context: ExecutionContext,
        prompt: &str,
    ) -> Result<ExecutionResult, AgentError>;

    /// Stream execution output
    fn output_stream(&self) -> impl Stream<Item = NormalizedMessage>;

    /// Stop execution
    async fn stop(&self) -> Result<(), AgentError>;
}

/// Normalized message type for all agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizedMessage {
    /// Agent is starting
    Start { timestamp: DateTime<Utc> },
    /// Text output from agent
    Text { content: String, timestamp: DateTime<Utc> },
    /// Tool usage
    ToolUse {
        tool_name: String,
        input: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Tool result
    ToolResult {
        tool_name: String,
        output: serde_json::Value,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    /// Agent asking user a question
    UserQuestion {
        question: String,
        options: Option<Vec<String>>,
        timestamp: DateTime<Utc>,
    },
    /// Agent completed
    Complete {
        success: bool,
        summary: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Error occurred
    Error {
        message: String,
        timestamp: DateTime<Utc>,
    },
}

/// Agent types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    Aider,
    GeminiCli,
    CodexCli,
    Amp,
}
```

**Docker Sandboxing:**

```rust
// crates/coding_agents/src/docker.rs

pub struct DockerSandbox {
    runtime: ContainerRuntime,  // Docker or Podman
    image: String,
    work_dir: PathBuf,
}

impl DockerSandbox {
    /// Create a new sandbox for agent execution
    pub async fn create(
        config: SandboxConfig,
    ) -> Result<Self, SandboxError>;

    /// Execute command in sandbox
    pub async fn exec(
        &self,
        command: &str,
        env: HashMap<String, String>,
    ) -> Result<ExecHandle, SandboxError>;

    /// Stream output from execution
    pub fn output_stream(&self) -> impl Stream<Item = Vec<u8>>;

    /// Destroy sandbox
    pub async fn destroy(self) -> Result<(), SandboxError>;
}
```

### 1.2 Create `task_store` Crate

Extract task storage from current `apps/desktop/src-tauri/src/services/task.rs` and `database/`.

**Storage Trait:**

```rust
// crates/task_store/src/lib.rs

#[async_trait]
pub trait TaskStore: Send + Sync {
    // Unit Tasks
    async fn create_unit_task(&self, task: CreateUnitTask) -> Result<UnitTask, StoreError>;
    async fn get_unit_task(&self, id: &str) -> Result<Option<UnitTask>, StoreError>;
    async fn update_unit_task_status(&self, id: &str, status: UnitTaskStatus) -> Result<(), StoreError>;
    async fn list_unit_tasks(&self, filter: TaskFilter) -> Result<Vec<UnitTask>, StoreError>;

    // Composite Tasks
    async fn create_composite_task(&self, task: CreateCompositeTask) -> Result<CompositeTask, StoreError>;
    async fn get_composite_task(&self, id: &str) -> Result<Option<CompositeTask>, StoreError>;
    // ... more methods

    // Repositories
    async fn create_repository(&self, repo: CreateRepository) -> Result<Repository, StoreError>;
    async fn get_repository(&self, id: &str) -> Result<Option<Repository>, StoreError>;
    // ... more methods

    // Workspaces
    async fn create_workspace(&self, workspace: CreateWorkspace) -> Result<Workspace, StoreError>;
    // ... more methods
}
```

**Implementations:**

```rust
// SQLite for single-user/single-process mode
pub struct SqliteStore { pool: SqlitePool }

// PostgreSQL for multi-user mode
pub struct PostgresStore { pool: PgPool }
```

### 1.3 Create `rpc_protocol` Crate

Define JSON-RPC protocol for server/client communication.

```rust
// crates/rpc_protocol/src/methods.rs

/// All RPC methods
pub enum RpcMethod {
    // Task methods
    CreateUnitTask,
    GetUnitTask,
    ListUnitTasks,
    UpdateUnitTaskStatus,
    StartTaskExecution,
    StopTaskExecution,

    // Composite task methods
    CreateCompositeTask,
    ApproveCompositePlan,

    // Repository methods
    AddRepository,
    ListRepositories,

    // Execution methods
    GetExecutionLogs,
    SubscribeExecutionLogs,

    // Secret methods
    SendSecrets,  // Client sends secrets to server for relay to worker

    // Worker methods (internal)
    RegisterWorker,
    WorkerHeartbeat,
    AssignTask,
}

// Request/Response types for each method
#[derive(Serialize, Deserialize)]
pub struct CreateUnitTaskRequest {
    pub repository_group_id: String,
    pub prompt: String,
    pub branch_name: Option<String>,
    pub agent_type: Option<AgentType>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUnitTaskResponse {
    pub task: UnitTask,
}
```

### 1.4 Create `git_ops` Crate

Extract git operations from current `apps/desktop/src-tauri/src/services/git.rs`.

```rust
// crates/git_ops/src/lib.rs

pub struct GitOperations;

impl GitOperations {
    /// Create a git worktree for isolated task execution
    pub fn create_worktree(
        repo_path: &Path,
        worktree_path: &Path,
        branch_name: &str,
        base_branch: &str,
    ) -> Result<Worktree, GitError>;

    /// Remove a worktree
    pub fn remove_worktree(worktree_path: &Path) -> Result<(), GitError>;

    /// Get diff between commits
    pub fn get_diff(
        repo_path: &Path,
        base_commit: &str,
        head_commit: &str,
    ) -> Result<String, GitError>;

    /// Get repository information
    pub fn get_repo_info(repo_path: &Path) -> Result<RepoInfo, GitError>;
}
```

---

## Phase 2: Implement Main Server

### 2.1 Server Structure

```rust
// apps/server/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ServerConfig::load()?;

    // Initialize database
    let store: Box<dyn TaskStore> = if config.single_user_mode {
        Box::new(SqliteStore::new(&config.database_path).await?)
    } else {
        Box::new(PostgresStore::new(&config.database_url).await?)
    };

    // Initialize auth (skip if single user mode)
    let auth = if config.single_user_mode {
        None
    } else {
        Some(OidcAuth::new(&config.oidc_config).await?)
    };

    // Initialize worker registry
    let worker_registry = WorkerRegistry::new();

    // Create app state
    let state = AppState {
        store: Arc::new(store),
        auth,
        worker_registry: Arc::new(RwLock::new(worker_registry)),
        config: Arc::new(config),
    };

    // Build router
    let app = Router::new()
        .route("/rpc", post(handle_rpc))
        .route("/ws", get(handle_websocket))
        .layer(auth_middleware(state.auth.clone()))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### 2.2 JSON-RPC Handler

```rust
// apps/server/src/rpc/handler.rs

pub async fn handle_rpc(
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthenticatedUser>>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Verify auth for non-single-user mode
    if state.auth.is_some() && user.is_none() {
        return JsonRpcResponse::error(
            request.id,
            JsonRpcError::unauthorized(),
        );
    }

    let result = match request.method.as_str() {
        "createUnitTask" => {
            let params: CreateUnitTaskRequest = serde_json::from_value(request.params)?;
            handle_create_unit_task(&state, params).await
        }
        "startTaskExecution" => {
            let params: StartTaskRequest = serde_json::from_value(request.params)?;
            handle_start_task_execution(&state, params).await
        }
        "sendSecrets" => {
            let params: SendSecretsRequest = serde_json::from_value(request.params)?;
            handle_send_secrets(&state, &user, params).await
        }
        // ... other methods
        _ => Err(JsonRpcError::method_not_found()),
    };

    match result {
        Ok(value) => JsonRpcResponse::success(request.id, value),
        Err(e) => JsonRpcResponse::error(request.id, e),
    }
}
```

### 2.3 Worker Registry

```rust
// apps/server/src/worker_registry.rs

pub struct WorkerRegistry {
    workers: HashMap<String, WorkerInfo>,
    task_assignments: HashMap<String, String>,  // task_id -> worker_id
}

pub struct WorkerInfo {
    pub id: String,
    pub address: SocketAddr,
    pub last_heartbeat: Instant,
    pub capacity: WorkerCapacity,
    pub current_tasks: Vec<String>,
}

impl WorkerRegistry {
    /// Register a new worker
    pub fn register(&mut self, worker: WorkerInfo) {
        self.workers.insert(worker.id.clone(), worker);
    }

    /// Update worker heartbeat
    pub fn heartbeat(&mut self, worker_id: &str) -> Result<(), RegistryError> {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(RegistryError::WorkerNotFound)
        }
    }

    /// Select best worker for a task
    pub fn select_worker_for_task(&self) -> Option<&WorkerInfo> {
        self.workers.values()
            .filter(|w| w.has_capacity())
            .min_by_key(|w| w.current_tasks.len())
    }

    /// Assign task to worker
    pub fn assign_task(&mut self, task_id: &str, worker_id: &str) {
        self.task_assignments.insert(task_id.to_string(), worker_id.to_string());
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.current_tasks.push(task_id.to_string());
        }
    }
}
```

### 2.4 WebSocket Handler for Real-time Updates

```rust
// apps/server/src/websocket/handler.rs

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(user): Extension<Option<AuthenticatedUser>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, user))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user: Option<AuthenticatedUser>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let request: JsonRpcRequest = serde_json::from_str(&text)?;
                match request.method.as_str() {
                    "subscribeExecutionLogs" => {
                        let params: SubscribeLogsRequest = ...;
                        // Subscribe to execution log stream
                        let mut rx = state.log_broadcaster.subscribe(params.task_id);
                        tokio::spawn(async move {
                            while let Ok(log) = rx.recv().await {
                                sender.send(Message::Text(
                                    serde_json::to_string(&log).unwrap()
                                )).await.ok();
                            }
                        });
                    }
                    // ... other subscriptions
                }
            }
            _ => {}
        }
    }
}
```

---

## Phase 3: Implement Worker Server

### 3.1 Worker Structure

```rust
// apps/worker/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WorkerConfig::load()?;

    // Initialize coding agents service
    let agents = CodingAgentsService::new()?;

    // Initialize Docker/Podman service
    let docker = DockerService::new(&config.container_runtime)?;

    // Create worker state
    let state = WorkerState {
        id: Uuid::new_v4().to_string(),
        config: Arc::new(config),
        agents: Arc::new(agents),
        docker: Arc::new(docker),
        active_tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // Connect to main server
    let server_client = MainServerClient::connect(&config.main_server_url).await?;

    // Register with main server
    server_client.register_worker(RegisterWorkerRequest {
        worker_id: state.id.clone(),
        capacity: state.get_capacity(),
    }).await?;

    // Start heartbeat task
    let heartbeat_client = server_client.clone();
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(e) = heartbeat_client.heartbeat(
                heartbeat_state.id.clone(),
                heartbeat_state.get_current_load(),
            ).await {
                tracing::error!("Heartbeat failed: {}", e);
            }
        }
    });

    // Listen for task assignments
    let mut task_receiver = server_client.subscribe_task_assignments().await?;
    while let Some(assignment) = task_receiver.recv().await {
        let state = state.clone();
        let client = server_client.clone();
        tokio::spawn(async move {
            execute_task(state, client, assignment).await
        });
    }

    Ok(())
}
```

### 3.2 Task Executor

```rust
// apps/worker/src/executor.rs

pub async fn execute_task(
    state: WorkerState,
    client: MainServerClient,
    assignment: TaskAssignment,
) -> Result<(), ExecutorError> {
    let task_id = &assignment.task_id;

    // Get task details from main server
    let task = client.get_unit_task(task_id).await?;
    let agent_task = client.get_agent_task(&task.agent_task_id).await?;

    // Get repository info
    let repo_group = client.get_repository_group(&task.repository_group_id).await?;

    // Create git worktree
    let worktree_path = PathBuf::from(format!("/tmp/delidev/worktrees/{}", task_id));
    git_ops::create_worktree(
        &repo_group.repositories[0].local_path,
        &worktree_path,
        &task.branch_name,
        &repo_group.repositories[0].default_branch,
    )?;

    // Create Docker sandbox
    let sandbox = DockerSandbox::create(SandboxConfig {
        image: get_docker_image(&repo_group),
        work_dir: worktree_path.clone(),
        env: assignment.secrets,  // Secrets from client via main server
    }).await?;

    // Get the appropriate agent
    let agent = state.agents.get_agent(agent_task.ai_agent_type)?;

    // Execute agent
    let execution_context = ExecutionContext {
        work_dir: worktree_path.clone(),
        sandbox: Some(sandbox),
        env: assignment.secrets,
    };

    // Stream logs to main server
    let log_stream = agent.output_stream();
    tokio::spawn({
        let client = client.clone();
        let task_id = task_id.clone();
        async move {
            pin_mut!(log_stream);
            while let Some(msg) = log_stream.next().await {
                client.send_execution_log(&task_id, msg).await.ok();
            }
        }
    });

    // Execute the agent
    let result = agent.execute(execution_context, &agent_task.prompt).await?;

    // Cleanup
    sandbox.destroy().await?;
    git_ops::remove_worktree(&worktree_path)?;

    // Report completion
    client.report_task_complete(task_id, result).await?;

    Ok(())
}
```

### 3.3 Secret Handler

```rust
// apps/worker/src/secret_handler.rs

/// Handles secrets received from main server (originally from client)
pub fn inject_secrets_to_env(
    secrets: HashMap<String, String>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    for (key, value) in secrets {
        match key.as_str() {
            "CLAUDE_CODE_OAUTH_TOKEN" => {
                env.insert("CLAUDE_CODE_USE_OAUTH".to_string(), "1".to_string());
                env.insert("CLAUDE_CODE_OAUTH_TOKEN".to_string(), value);
            }
            "ANTHROPIC_API_KEY" => {
                env.insert("ANTHROPIC_API_KEY".to_string(), value);
            }
            "OPENAI_API_KEY" => {
                env.insert("OPENAI_API_KEY".to_string(), value);
            }
            // ... other known secrets
            _ => {
                // Pass through unknown secrets
                env.insert(key, value);
            }
        }
    }

    env
}
```

---

## Phase 4: Implement Client

### 4.1 Client Architecture

The client uses react-query for all server communication.

```typescript
// apps/desktop/src/api/client.ts

import { createTRPCClient } from '@trpc/client';

// Create JSON-RPC client
class JsonRpcClient {
  private serverUrl: string;
  private ws: WebSocket | null = null;
  private authToken: string | null = null;

  constructor(serverUrl: string) {
    this.serverUrl = serverUrl;
  }

  setAuthToken(token: string) {
    this.authToken = token;
  }

  async call<T>(method: string, params: any): Promise<T> {
    const response = await fetch(`${this.serverUrl}/rpc`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.authToken ? { 'Authorization': `Bearer ${this.authToken}` } : {}),
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: crypto.randomUUID(),
        method,
        params,
      }),
    });

    const result = await response.json();
    if (result.error) {
      throw new Error(result.error.message);
    }
    return result.result;
  }

  // WebSocket for real-time subscriptions
  subscribe(method: string, params: any, callback: (data: any) => void) {
    if (!this.ws) {
      this.ws = new WebSocket(`${this.serverUrl.replace('http', 'ws')}/ws`);
    }
    // ... subscription logic
  }
}
```

### 4.2 React Query Integration

```typescript
// apps/desktop/src/api/hooks.ts

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { rpcClient } from './client';

// Task queries
export function useUnitTasks(filter?: TaskFilter) {
  return useQuery({
    queryKey: ['unitTasks', filter],
    queryFn: () => rpcClient.call<UnitTask[]>('listUnitTasks', { filter }),
  });
}

export function useUnitTask(id: string) {
  return useQuery({
    queryKey: ['unitTask', id],
    queryFn: () => rpcClient.call<UnitTask>('getUnitTask', { id }),
    enabled: !!id,
  });
}

// Task mutations
export function useCreateUnitTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateUnitTaskRequest) =>
      rpcClient.call<UnitTask>('createUnitTask', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['unitTasks'] });
    },
  });
}

export function useStartTaskExecution() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ taskId, secrets }: { taskId: string; secrets: Secrets }) => {
      // First send secrets
      await rpcClient.call('sendSecrets', { taskId, secrets });
      // Then start execution
      return rpcClient.call('startTaskExecution', { taskId });
    },
    onSuccess: (_, { taskId }) => {
      queryClient.invalidateQueries({ queryKey: ['unitTask', taskId] });
    },
  });
}

// Real-time execution logs
export function useExecutionLogs(taskId: string) {
  const [logs, setLogs] = useState<NormalizedMessage[]>([]);

  useEffect(() => {
    const unsubscribe = rpcClient.subscribe(
      'subscribeExecutionLogs',
      { taskId },
      (message) => {
        setLogs(prev => [...prev, message]);
      }
    );

    return () => unsubscribe();
  }, [taskId]);

  return logs;
}
```

### 4.3 Secrets from Keychain

```typescript
// apps/desktop/src/api/secrets.ts

import { invoke } from '@tauri-apps/api/core';

export interface Secrets {
  CLAUDE_CODE_OAUTH_TOKEN?: string;
  ANTHROPIC_API_KEY?: string;
  OPENAI_API_KEY?: string;
  [key: string]: string | undefined;
}

export async function getSecretsFromKeychain(): Promise<Secrets> {
  // Call Tauri backend to get secrets from system keychain
  const secrets = await invoke<Secrets>('get_secrets_from_keychain');
  return secrets;
}

// In Rust (apps/desktop/src-tauri/src/commands/secrets.rs)
#[tauri::command]
pub async fn get_secrets_from_keychain() -> Result<HashMap<String, String>, String> {
    use secrets::keychain::KeychainService;

    let keychain = KeychainService::new();
    let mut secrets = HashMap::new();

    // Get Claude Code OAuth token
    if let Ok(token) = keychain.get("CLAUDE_CODE_OAUTH_TOKEN") {
        secrets.insert("CLAUDE_CODE_OAUTH_TOKEN".to_string(), token);
    }

    // Get Anthropic API key
    if let Ok(key) = keychain.get("ANTHROPIC_API_KEY") {
        secrets.insert("ANTHROPIC_API_KEY".to_string(), key);
    }

    // ... other secrets

    Ok(secrets)
}
```

---

## Phase 5: Single Process Mode

### 5.1 Single Process Orchestration

```rust
// apps/desktop/src-tauri/src/single_process.rs

use coding_agents::{CodingAgentsService, DockerSandbox};
use task_store::SqliteStore;
use rpc_protocol::RpcHandler;

/// Single process mode embeds all components in one process
pub struct SingleProcessApp {
    store: Arc<SqliteStore>,
    agents: Arc<CodingAgentsService>,
    docker: Arc<DockerService>,
    // No network, direct function calls
}

impl SingleProcessApp {
    pub async fn new(data_dir: PathBuf) -> Result<Self, AppError> {
        let db_path = data_dir.join("delidev.db");
        let store = Arc::new(SqliteStore::new(&db_path).await?);
        let agents = Arc::new(CodingAgentsService::new()?);
        let docker = Arc::new(DockerService::new_with_default_runtime()?);

        Ok(Self { store, agents, docker })
    }

    /// Execute task locally (no network)
    pub async fn execute_task(&self, task_id: &str) -> Result<(), ExecutionError> {
        let task = self.store.get_unit_task(task_id).await?
            .ok_or(ExecutionError::TaskNotFound)?;
        let agent_task = self.store.get_agent_task(&task.agent_task_id).await?
            .ok_or(ExecutionError::TaskNotFound)?;

        // Get secrets from local keychain
        let secrets = secrets::keychain::get_all_secrets()?;

        // Create worktree
        let worktree_path = PathBuf::from(format!("/tmp/delidev/worktrees/{}", task_id));
        git_ops::create_worktree(...)?;

        // Execute agent locally
        let agent = self.agents.get_agent(agent_task.ai_agent_type)?;
        let context = ExecutionContext {
            work_dir: worktree_path.clone(),
            sandbox: if self.config.use_container {
                Some(DockerSandbox::create(...).await?)
            } else {
                None
            },
            env: secrets,
        };

        agent.execute(context, &agent_task.prompt).await?;

        // Cleanup
        git_ops::remove_worktree(&worktree_path)?;

        Ok(())
    }
}

// Tauri integration
#[tauri::command]
pub async fn create_unit_task(
    state: State<'_, SingleProcessApp>,
    request: CreateUnitTaskRequest,
) -> Result<UnitTask, String> {
    state.store.create_unit_task(request.into())
        .await
        .map_err(|e| e.to_string())
}
```

### 5.2 Mode Detection

```rust
// apps/desktop/src-tauri/src/lib.rs

pub fn run() {
    let config = load_app_config();

    if config.server_mode == ServerMode::SingleProcess {
        // Run everything in single process
        run_single_process_app()
    } else {
        // Run as client only, connect to remote server
        run_client_app(&config.server_url)
    }
}

fn run_single_process_app() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize single process app
            let single_app = tauri::async_runtime::block_on(async {
                SingleProcessApp::new(app_data_dir()).await
            })?;

            app_handle.manage(Arc::new(single_app));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Direct commands to SingleProcessApp
            create_unit_task,
            get_unit_task,
            start_task_execution,
            // ... all commands
        ])
        .run(tauri::generate_context!())
        .expect("error running single process app");
}

fn run_client_app(server_url: &str) {
    tauri::Builder::default()
        .setup(|app| {
            let client = JsonRpcClient::new(server_url);
            app.manage(Arc::new(client));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Commands that proxy to server via JSON-RPC
            proxy_create_unit_task,
            proxy_get_unit_task,
            proxy_start_task_execution,
            // ... all proxy commands
        ])
        .run(tauri::generate_context!())
        .expect("error running client app");
}
```

---

## Phase 6: Authentication

### 6.1 JWT Authentication

```rust
// crates/auth/src/jwt.rs

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // User ID
    pub email: String,
    pub exp: usize,        // Expiration time
    pub iat: usize,        // Issued at
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtAuth {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
        }
    }

    pub fn create_token(&self, user_id: &str, email: &str) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: now + 3600 * 24,  // 24 hours
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(AuthError::JwtEncode)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(AuthError::JwtDecode)
    }
}
```

### 6.2 OpenID Connect Integration

```rust
// crates/auth/src/oidc.rs

use openidconnect::{
    core::{CoreClient, CoreProviderMetadata},
    AuthorizationCode, ClientId, ClientSecret, IssuerUrl, RedirectUrl,
};

pub struct OidcAuth {
    client: CoreClient,
}

impl OidcAuth {
    pub async fn new(config: &OidcConfig) -> Result<Self, AuthError> {
        let issuer_url = IssuerUrl::new(config.issuer_url.clone())?;
        let metadata = CoreProviderMetadata::discover_async(
            issuer_url,
            async_http_client,
        ).await?;

        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url.clone())?);

        Ok(Self { client })
    }

    /// Generate authorization URL for user to authenticate
    pub fn get_auth_url(&self) -> (Url, CsrfToken, Nonce) {
        self.client
            .authorize_url(
                openidconnect::AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .url()
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        code: &str,
        nonce: &Nonce,
    ) -> Result<AuthenticatedUser, AuthError> {
        let token_response = self.client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await?;

        // Verify ID token
        let id_token = token_response.id_token()
            .ok_or(AuthError::MissingIdToken)?;
        let claims = id_token.claims(
            &self.client.id_token_verifier(),
            nonce,
        )?;

        Ok(AuthenticatedUser {
            id: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
        })
    }
}
```

### 6.3 Auth Middleware

```rust
// crates/auth/src/middleware.rs

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware<B>(
    State(auth): State<Option<Arc<JwtAuth>>>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Skip auth if not configured (single process mode)
    if auth.is_none() {
        request.extensions_mut().insert::<Option<AuthenticatedUser>>(None);
        return Ok(next.run(request).await);
    }

    let auth = auth.unwrap();

    // Extract token from Authorization header
    let token = request.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    match token {
        Some(token) => {
            match auth.verify_token(token) {
                Ok(claims) => {
                    let user = AuthenticatedUser {
                        id: claims.sub,
                        email: claims.email,
                    };
                    request.extensions_mut().insert(Some(user));
                    Ok(next.run(request).await)
                }
                Err(_) => Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
```

---

## Phase 7: Secrets Management

### 7.1 Keychain Access (Client Side)

```rust
// crates/secrets/src/keychain.rs

#[cfg(target_os = "macos")]
mod macos {
    use security_framework::passwords::*;

    pub fn get_secret(service: &str, account: &str) -> Result<String, KeychainError> {
        get_generic_password(service, account)
            .map(|p| String::from_utf8_lossy(&p).to_string())
            .map_err(KeychainError::from)
    }

    pub fn set_secret(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
        set_generic_password(service, account, secret.as_bytes())
            .map_err(KeychainError::from)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use keyring::Entry;

    pub fn get_secret(service: &str, account: &str) -> Result<String, KeychainError> {
        Entry::new(service, account)?
            .get_password()
            .map_err(KeychainError::from)
    }

    pub fn set_secret(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
        Entry::new(service, account)?
            .set_password(secret)
            .map_err(KeychainError::from)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use keyring::Entry;

    // Similar to Windows using libsecret/KWallet
}

/// Get all relevant secrets for AI agents
pub fn get_all_secrets() -> Result<HashMap<String, String>, KeychainError> {
    let mut secrets = HashMap::new();

    // Claude Code OAuth token
    if let Ok(token) = get_secret("com.delino.delidev", "claude_code_oauth") {
        secrets.insert("CLAUDE_CODE_OAUTH_TOKEN".to_string(), token);
    }

    // Anthropic API key
    if let Ok(key) = get_secret("com.delino.delidev", "anthropic_api_key") {
        secrets.insert("ANTHROPIC_API_KEY".to_string(), key);
    }

    // OpenAI API key
    if let Ok(key) = get_secret("com.delino.delidev", "openai_api_key") {
        secrets.insert("OPENAI_API_KEY".to_string(), key);
    }

    Ok(secrets)
}
```

### 7.2 Secure Secret Transport

```rust
// crates/secrets/src/transport.rs

/// Secrets are sent from client to main server, then relayed to worker
/// They should be encrypted in transit (TLS) and not stored on server

#[derive(Serialize, Deserialize)]
pub struct SecretPayload {
    pub task_id: String,
    /// Secrets encrypted with worker's public key (optional additional encryption)
    pub secrets: HashMap<String, String>,
    /// Timestamp to prevent replay attacks
    pub timestamp: i64,
    /// Signature from client
    pub signature: String,
}

impl SecretPayload {
    pub fn new(task_id: &str, secrets: HashMap<String, String>) -> Self {
        Self {
            task_id: task_id.to_string(),
            secrets,
            timestamp: chrono::Utc::now().timestamp(),
            signature: String::new(),  // Will be signed before sending
        }
    }

    pub fn sign(&mut self, private_key: &[u8]) {
        // Sign the payload for verification
        let data = format!("{}:{}:{:?}", self.task_id, self.timestamp, self.secrets);
        self.signature = sign_data(&data, private_key);
    }

    pub fn verify(&self, public_key: &[u8]) -> bool {
        // Verify signature
        let data = format!("{}:{}:{:?}", self.task_id, self.timestamp, self.secrets);
        verify_signature(&data, &self.signature, public_key)
    }
}
```

---

## Communication Protocol

### JSON-RPC 2.0 Specification

All communication uses JSON-RPC 2.0 over HTTP POST and WebSocket.

**Request Format:**
```json
{
  "jsonrpc": "2.0",
  "id": "uuid-here",
  "method": "methodName",
  "params": {
    "param1": "value1"
  }
}
```

**Response Format:**
```json
{
  "jsonrpc": "2.0",
  "id": "uuid-here",
  "result": { ... }
}
```

**Error Format:**
```json
{
  "jsonrpc": "2.0",
  "id": "uuid-here",
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": { ... }
  }
}
```

### Method Categories

#### Task Methods
- `createUnitTask` - Create a new unit task
- `getUnitTask` - Get unit task by ID
- `listUnitTasks` - List unit tasks with filter
- `updateUnitTaskStatus` - Update task status
- `deleteUnitTask` - Delete a unit task
- `createCompositeTask` - Create composite task
- `approveCompositePlan` - Approve composite task plan
- `rejectCompositePlan` - Reject composite task plan

#### Execution Methods
- `startTaskExecution` - Start task execution
- `stopTaskExecution` - Stop running task
- `getExecutionLogs` - Get historical logs
- `subscribeExecutionLogs` - Subscribe to real-time logs (WebSocket)

#### Repository Methods
- `addRepository` - Add repository
- `listRepositories` - List repositories
- `removeRepository` - Remove repository

#### Secret Methods
- `sendSecrets` - Send secrets for task execution

#### Worker Methods (Internal)
- `registerWorker` - Register worker with server
- `workerHeartbeat` - Worker heartbeat
- `assignTask` - Assign task to worker
- `reportTaskComplete` - Report task completion

---

## Migration Strategy

### Phase 1: Extract Crates (2-3 weeks)

1. Create `crates/` directory structure
2. Extract `coding_agents` crate from current agent execution code
3. Extract `task_store` crate from database/services
4. Extract `git_ops` crate from git service
5. Update desktop app to use new crates
6. Run existing tests to verify no regression

### Phase 2: Add RPC Layer (2 weeks)

1. Create `rpc_protocol` crate
2. Define all method types
3. Update desktop app to use RPC internally (prepare for extraction)

### Phase 3: Implement Servers (3-4 weeks)

1. Create `apps/server/` with basic structure
2. Implement JSON-RPC handler
3. Implement WebSocket handler
4. Create `apps/worker/` with basic structure
5. Implement worker registration and heartbeat
6. Implement task execution

### Phase 4: Update Client (2 weeks)

1. Create react-query hooks
2. Add JSON-RPC client
3. Add mode switching (single process vs client)

### Phase 5: Authentication (2 weeks)

1. Create `auth` crate
2. Implement JWT
3. Implement OpenID Connect
4. Add auth middleware to server

### Phase 6: Secrets Management (1-2 weeks)

1. Create `secrets` crate
2. Implement keychain access
3. Implement secure transport

### Phase 7: Testing & Polish (2 weeks)

1. Integration tests
2. Performance testing
3. Documentation
4. Error handling improvements

---

## Database Considerations

### SQLite (Single Process Mode)

Current schema continues to be used. Located at `~/.delidev/delidev.db`.

### PostgreSQL (Multi-User Mode)

Same schema structure but with:
- UUID primary keys (instead of TEXT)
- Proper foreign key constraints
- Indexes for common queries
- Connection pooling

### Migration

```sql
-- Example PostgreSQL schema migration
CREATE TABLE unit_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_group_id UUID NOT NULL REFERENCES repository_groups(id),
    agent_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    branch_name TEXT,
    linked_pr_url TEXT,
    base_commit TEXT,
    end_commit TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_unit_tasks_status ON unit_tasks(status);
CREATE INDEX idx_unit_tasks_repository_group ON unit_tasks(repository_group_id);
```

---

## Summary

This implementation plan transforms DeliDev from a local-first desktop application into a flexible server/worker/client architecture while maintaining backward compatibility through single-process mode.

Key principles:
1. **No breaking changes** - Existing single-process usage continues to work
2. **Gradual migration** - Each phase can be completed and tested independently
3. **Clean separation** - Each crate has clear responsibilities
4. **Type safety** - All communication uses strongly-typed Rust structs
5. **Normalization** - All AI agent output is normalized through `coding_agents` crate

Total estimated timeline: **14-18 weeks** for full implementation.
