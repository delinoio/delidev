# Main Server

The Main Server is the central hub of DeliDev's distributed architecture. It maintains the task list, coordinates workers, and provides the RPC interface for clients.

## Role

| Responsibility | Description |
|----------------|-------------|
| Task Management | Maintains the list of tasks (UnitTask, CompositeTask) |
| Worker Coordination | Assigns tasks to available workers, tracks worker health |
| RPC Server | Provides Connect RPC API for clients |
| Authentication | Handles JWT authentication and OIDC (in remote mode) |
| Secret Relay | Receives secrets from clients, relays to workers when tasks start |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Main Server                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────┐  ┌─────────────────────────────────┐  │
│  │   RPC Endpoints     │  │      Worker Registry            │  │
│  │   (Connect RPC)     │  │   - Active workers list         │  │
│  │                     │  │   - Health check status         │  │
│  └──────────┬──────────┘  │   - Task assignments            │  │
│             │             └─────────────────────────────────┘  │
│             ▼                                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Task Store                            │   │
│  │   - UnitTask, CompositeTask                              │   │
│  │   - AgentTask, AgentSession                              │   │
│  │   - TodoItem, Repository                                 │   │
│  └─────────────────────────────────────────────────────────┘   │
│             │                                                   │
│             ▼                                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Database                              │   │
│  │   PostgreSQL (multi-user) / SQLite (single-user)        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────┐  ┌─────────────────────────────────┐  │
│  │   Auth Module       │  │      Secret Cache               │  │
│  │   - JWT issuance    │  │   - Per-task secret storage     │  │
│  │   - OIDC flow       │  │   - In-memory, cleared on       │  │
│  │   - Token verify    │  │     task completion             │  │
│  └─────────────────────┘  └─────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## API Endpoints (Connect RPC)

All endpoints use Connect RPC protocol over HTTP. Services are defined in Protobuf and generated for both Rust and TypeScript.

### Task Management

| Method | Description |
|--------|-------------|
| `task.createUnit` | Create a new UnitTask |
| `task.createComposite` | Create a new CompositeTask |
| `task.get` | Get task by ID |
| `task.list` | List tasks with filters |
| `task.updateStatus` | Update task status |
| `task.delete` | Delete a task |
| `task.retry` | Retry a failed task |
| `task.approve` | Approve a task (CompositeTask plan or UnitTask review) |
| `task.reject` | Reject a task |
| `task.requestChanges` | Request changes on a task in review |

### Agent Session

| Method | Description |
|--------|-------------|
| `session.getLog` | Get agent session output log |
| `session.streamLog` | Stream agent session output (WebSocket) |
| `session.stop` | Stop a running agent session |
| `session.submitTtyInput` | Submit response to TTY input request |

### Repository Management

| Method | Description |
|--------|-------------|
| `repository.add` | Add a repository |
| `repository.list` | List repositories |
| `repository.get` | Get repository by ID |
| `repository.remove` | Remove a repository |
| `repositoryGroup.create` | Create a repository group |
| `repositoryGroup.list` | List repository groups |
| `repositoryGroup.update` | Update a repository group |
| `repositoryGroup.delete` | Delete a repository group |

### Workspace Management

| Method | Description |
|--------|-------------|
| `workspace.create` | Create a workspace |
| `workspace.list` | List workspaces |
| `workspace.get` | Get workspace by ID |
| `workspace.update` | Update workspace |
| `workspace.delete` | Delete workspace |

### TodoItem

| Method | Description |
|--------|-------------|
| `todo.list` | List todo items |
| `todo.get` | Get todo item by ID |
| `todo.updateStatus` | Update todo item status |
| `todo.dismiss` | Dismiss a todo item |

### Secrets

| Method | Description |
|--------|-------------|
| `secrets.send` | Send secrets from client to server (for task execution) |
| `secrets.clear` | Clear cached secrets for a task |

### Worker (Internal)

These endpoints are used by Worker Servers, not clients.

| Method | Description |
|--------|-------------|
| `worker.register` | Register a new worker |
| `worker.heartbeat` | Send heartbeat from worker |
| `worker.unregister` | Unregister a worker |
| `worker.getTask` | Get next task to execute |
| `worker.reportStatus` | Report task execution status |
| `worker.getSecrets` | Get secrets for a task (called by worker when task starts) |

### Authentication

| Method | Description |
|--------|-------------|
| `auth.getLoginUrl` | Get OIDC login URL |
| `auth.handleCallback` | Handle OIDC callback |
| `auth.refreshToken` | Refresh access token |
| `auth.getCurrentUser` | Get current authenticated user |
| `auth.logout` | Logout (invalidate token) |

## Database Schema

### Core Tables

```sql
-- Users (multi-user mode only)
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Workspaces
CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    user_id UUID REFERENCES users(id),  -- NULL in single-user mode
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Repositories
CREATE TABLE repositories (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name VARCHAR(255) NOT NULL,
    local_path TEXT NOT NULL,
    remote_url TEXT NOT NULL,
    default_branch VARCHAR(255) NOT NULL DEFAULT 'main',
    vcs_type VARCHAR(50) NOT NULL DEFAULT 'git',
    vcs_provider_type VARCHAR(50) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Repository Groups
CREATE TABLE repository_groups (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name VARCHAR(255),  -- NULL for single-repo groups
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE repository_group_members (
    group_id UUID NOT NULL REFERENCES repository_groups(id),
    repository_id UUID NOT NULL REFERENCES repositories(id),
    PRIMARY KEY (group_id, repository_id)
);
```

### Task Tables

```sql
-- Agent Sessions
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY,
    agent_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    ai_agent_type VARCHAR(50) NOT NULL,
    ai_agent_model VARCHAR(255),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    output_log TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Agent Tasks
CREATE TABLE agent_tasks (
    id UUID PRIMARY KEY,
    ai_agent_type VARCHAR(50),
    ai_agent_model VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE agent_task_base_remotes (
    agent_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    git_remote_dir_path TEXT NOT NULL,
    git_branch_name VARCHAR(255) NOT NULL,
    PRIMARY KEY (agent_task_id, git_remote_dir_path)
);

-- Unit Tasks
CREATE TABLE unit_tasks (
    id UUID PRIMARY KEY,
    repository_group_id UUID NOT NULL REFERENCES repository_groups(id),
    agent_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    branch_name VARCHAR(255),
    linked_pr_url TEXT,
    base_commit VARCHAR(40),
    end_commit VARCHAR(40),
    status VARCHAR(50) NOT NULL DEFAULT 'in_progress',
    prompt TEXT NOT NULL,
    title VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE unit_task_auto_fix_tasks (
    unit_task_id UUID NOT NULL REFERENCES unit_tasks(id),
    agent_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (unit_task_id, agent_task_id)
);

-- Composite Tasks
CREATE TABLE composite_tasks (
    id UUID PRIMARY KEY,
    repository_group_id UUID NOT NULL REFERENCES repository_groups(id),
    planning_task_id UUID NOT NULL REFERENCES agent_tasks(id),
    status VARCHAR(50) NOT NULL DEFAULT 'planning',
    execution_agent_type VARCHAR(50),
    prompt TEXT NOT NULL,
    title VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Composite Task Nodes
CREATE TABLE composite_task_nodes (
    id UUID PRIMARY KEY,
    composite_task_id UUID NOT NULL REFERENCES composite_tasks(id),
    unit_task_id UUID NOT NULL REFERENCES unit_tasks(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE composite_task_node_dependencies (
    node_id UUID NOT NULL REFERENCES composite_task_nodes(id),
    depends_on_node_id UUID NOT NULL REFERENCES composite_task_nodes(id),
    PRIMARY KEY (node_id, depends_on_node_id)
);
```

### Other Tables

```sql
-- Todo Items
CREATE TABLE todo_items (
    id UUID PRIMARY KEY,
    type VARCHAR(50) NOT NULL,  -- 'issue_triage', 'pr_review'
    source VARCHAR(50) NOT NULL DEFAULT 'auto',
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    repository_id UUID NOT NULL REFERENCES repositories(id),
    data JSONB NOT NULL,  -- Type-specific fields
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- TTY Input Requests
CREATE TABLE tty_input_requests (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES unit_tasks(id),
    session_id UUID NOT NULL REFERENCES agent_sessions(id),
    prompt TEXT NOT NULL,
    input_type VARCHAR(50) NOT NULL,
    options JSONB,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    response TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    responded_at TIMESTAMP
);

-- OIDC Auth States
CREATE TABLE auth_states (
    state VARCHAR(255) PRIMARY KEY,
    code_verifier VARCHAR(255) NOT NULL,
    redirect_uri TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL
);

-- Workers (for worker registry)
CREATE TABLE workers (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    endpoint_url TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    last_heartbeat TIMESTAMP NOT NULL DEFAULT NOW(),
    current_task_id UUID REFERENCES unit_tasks(id),
    registered_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

## Worker Management

### Worker Registration

Workers register themselves with the Main Server on startup:

```
Worker starts
    ▼
POST worker.register {
    name: "worker-1",
    endpoint_url: "http://worker-1:54872"
}
    ▼
Server adds worker to registry
    ▼
Worker starts heartbeat loop
```

### Heartbeat

Workers send periodic heartbeats (every 30 seconds):

```
Worker sends heartbeat
    ▼
POST worker.heartbeat {
    worker_id: "...",
    status: "idle" | "busy",
    current_task_id: "..." | null
}
    ▼
Server updates last_heartbeat timestamp
```

If a worker misses 3 consecutive heartbeats (90 seconds), it is marked as `unhealthy` and tasks assigned to it are reassigned.

### Task Assignment

When a new task needs execution:

1. Server finds an available (idle + healthy) worker
2. Server marks worker as `busy` with task assignment
3. Worker receives task via `worker.getTask` polling or WebSocket notification
4. Worker fetches secrets via `worker.getSecrets`
5. Worker executes task and reports progress
6. Worker reports completion via `worker.reportStatus`
7. Server marks worker as `idle`

## Authentication Flow

### Remote Mode (OIDC)

```
Client requests login
    ▼
Server generates auth URL with:
    - PKCE code_verifier/code_challenge
    - State parameter (stored in DB)
    - Redirect URI
    ▼
User authenticates with OIDC provider
    ▼
Provider redirects to callback
    ▼
Server exchanges code for tokens
    ▼
Server validates ID token
    ▼
Server creates/updates user in DB
    ▼
Server issues JWT to client
    ▼
Client uses JWT for subsequent requests
```

### Single-User Mode

Authentication is completely disabled. All requests are treated as authenticated.

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `DATABASE_URL` | PostgreSQL connection URL | Remote mode |
| `DELIDEV_SINGLE_USER_MODE` | Set to `true` for single-user mode | No |
| `DELIDEV_JWT_SECRET` | JWT signing secret | Remote mode |
| `DELIDEV_OIDC_ISSUER_URL` | OIDC provider URL | Remote mode |
| `DELIDEV_OIDC_CLIENT_ID` | OAuth2 client ID | Remote mode |
| `DELIDEV_OIDC_CLIENT_SECRET` | OAuth2 client secret | Remote mode |
| `DELIDEV_SERVER_PORT` | Server port | No (default: 54871) |
| `DELIDEV_LOG_LEVEL` | Log level | No (default: info) |

### Single-User Mode

When `DELIDEV_SINGLE_USER_MODE=true`:
- Uses SQLite instead of PostgreSQL
- Authentication is disabled
- All requests bypass auth middleware
- User table is not used

## Error Handling

| Error Code | Description |
|------------|-------------|
| -32600 | Invalid Request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32001 | Authentication required |
| -32002 | Permission denied |
| -32003 | Resource not found |
| -32004 | Worker unavailable |
| -32005 | Task execution failed |
