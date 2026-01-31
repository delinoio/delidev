# Worker Server

The Worker Server executes AI coding agents in isolated Docker containers. It receives tasks from the Main Server and reports execution progress.

## Role

| Responsibility | Description |
|----------------|-------------|
| AI Agent Execution | Runs Claude Code, OpenCode, Aider, and other AI coding agents |
| Docker Sandboxing | Executes agents in isolated Docker containers |
| Secret Injection | Injects secrets as environment variables |
| Output Normalization | Normalizes agent output via `coding_agents` crate |
| Heartbeat | Reports health status to Main Server |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Worker Server                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   Task Executor                          │   │
│  │   - Receives task from Main Server                       │   │
│  │   - Manages task lifecycle                               │   │
│  └──────────────────────────┬──────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                Docker Manager                            │   │
│  │   - Creates containers for each AgentSession             │   │
│  │   - Builds images from .delidev/setup/Dockerfile         │   │
│  │   - Manages container lifecycle                          │   │
│  └──────────────────────────┬──────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                coding_agents Crate                       │   │
│  │   - Agent abstraction (Claude Code, OpenCode, etc.)      │   │
│  │   - Output parsing and normalization                     │   │
│  │   - TTY input detection                                  │   │
│  └──────────────────────────┬──────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  Git Operations                          │   │
│  │   - Worktree creation                                    │   │
│  │   - Branch management                                    │   │
│  │   - Commit operations                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────┐  ┌─────────────────────────────────┐  │
│  │   Heartbeat Loop    │  │      Secret Handler             │  │
│  │   (to Main Server)  │  │   - Receives secrets from Main  │  │
│  │                     │  │   - Injects as env vars         │  │
│  └─────────────────────┘  └─────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Task Execution Flow

```
1. Task Received from Main Server
              ▼
2. Create Git Worktree
   - Clone from base branch
   - Create task branch
              ▼
3. Get Secrets from Main Server
   - worker.getSecrets RPC call
              ▼
4. Build Docker Image (if needed)
   - From .delidev/setup/Dockerfile
   - Or use default (node:20-slim)
              ▼
5. Start Docker Container
   - Mount worktree as /workspace/{repo}
   - Set HOME=/workspace
   - Inject secrets as env vars
              ▼
6. Run AI Agent
   - Execute agent command (claude, opencode, etc.)
   - Stream output to Main Server
   - Detect TTY input requests
              ▼
7. Wait for Completion or User Input
   - If TTY input needed: notify Main Server, wait for response
   - If completed: collect output
              ▼
8. Report Result to Main Server
   - worker.reportStatus RPC call
              ▼
9. Cleanup
   - Stop container
   - Keep worktree for review (or cleanup if rejected)
```

## Docker Sandboxing

### Container Configuration

Each AgentSession runs in an isolated Docker container:

| Setting | Value |
|---------|-------|
| Network | Host network (for git/API access) |
| User | Non-root user (configurable) |
| Working Directory | `/workspace/{repoName}` |
| Home Directory | `/workspace` |
| Memory Limit | Configurable (default: 8GB) |
| CPU Limit | Configurable (default: no limit) |

### Image Building

The Worker builds Docker images for each repository:

1. **Check for custom Dockerfile**: `.delidev/setup/Dockerfile`
2. **Build image**: If Dockerfile exists, build custom image
3. **Use default**: Otherwise, use `node:20-slim`

### Volume Mounts

| Host Path | Container Path | Purpose |
|-----------|----------------|---------|
| `{worktree_path}` | `/workspace/{repoName}` | Repository code |
| `~/.claude` | `/workspace/.claude` | Claude Code config (if using Claude Code) |
| `~/.opencode` | `/workspace/.opencode` | OpenCode config (if using OpenCode) |

### Environment Variables

Injected into the container:

| Variable | Source |
|----------|--------|
| `HOME` | Set to `/workspace` |
| `TERM` | Set to `xterm-256color` |
| `CLAUDE_CODE_OAUTH_TOKEN` | From secrets (if present) |
| `CLAUDE_CODE_USE_OAUTH` | Set to `1` if OAuth token present |
| `ANTHROPIC_API_KEY` | From secrets (if present) |
| `OPENAI_API_KEY` | From secrets (if present) |
| `GOOGLE_AI_API_KEY` | From secrets (if present) |
| `GEMINI_API_KEY` | Same as GOOGLE_AI_API_KEY |
| `GITHUB_TOKEN` | From secrets (if present) |
| `GH_TOKEN` | Same as GITHUB_TOKEN |

## AI Agent Execution

### Supported Agents

| Agent | Command | Output Format |
|-------|---------|---------------|
| Claude Code | `claude --output-format stream-json` | JSON stream |
| OpenCode | `opencode --output-format json` | JSON |
| Gemini CLI | `gemini` | Text |
| Codex CLI | `codex` | Text |
| Aider | `aider --yes` | Text |
| Amp | `amp` | JSON |

### Output Normalization

The `coding_agents` crate normalizes all agent outputs to a unified format:

```rust
pub enum NormalizedEvent {
    // Text output
    TextOutput { content: String, stream: bool },
    ErrorOutput { content: String },

    // Tool usage
    ToolUse { tool_name: String, input: serde_json::Value },
    ToolResult { tool_name: String, output: serde_json::Value, is_error: bool },

    // File operations
    FileChange { path: String, change_type: FileChangeType, content: Option<String> },

    // Commands
    CommandExecution { command: String, exit_code: Option<i32>, output: Option<String> },

    // User interaction
    AskUserQuestion { question: String, options: Option<Vec<String>> },
    UserResponse { response: String },

    // Session lifecycle
    SessionStart { agent_type: AIAgentType, model: Option<String> },
    SessionEnd { success: bool, error: Option<String> },

    // Thinking/reasoning
    Thinking { content: String },
}

pub enum FileChangeType {
    Create,
    Modify,
    Delete,
    Rename { from: String },
}
```

### TTY Input Detection

The Worker detects when agents request user input:

**Claude Code**: Detects `AskUserQuestion` tool use in stream-json output:
```json
{"type": "tool_use", "name": "AskUserQuestion", "input": {"question": "..."}}
```

**OpenCode**: Detects question events:
```json
{"type": "question", "content": "...", "options": [...]}
```

When detected:
1. Worker pauses agent execution (holds stdin)
2. Creates TtyInputRequest in Main Server
3. Waits for response via RPC callback
4. Writes response to agent stdin
5. Resumes execution

## Secrets Handling

### Secret Retrieval

When a task starts, the Worker retrieves secrets from Main Server:

```
Worker                             Main Server
   │                                    │
   ├─── worker.getSecrets ─────────────►│
   │    { task_id: "..." }              │
   │                                    │
   │◄── Secrets response ───────────────┤
   │    { secrets: {...} }              │
   │                                    │
```

### Secret Injection

Secrets are injected as environment variables when starting the Docker container:

```rust
// Pseudocode
let mut env_vars = HashMap::new();

if let Some(token) = secrets.get("CLAUDE_CODE_OAUTH_TOKEN") {
    env_vars.insert("CLAUDE_CODE_OAUTH_TOKEN", token);
    env_vars.insert("CLAUDE_CODE_USE_OAUTH", "1");
}

if let Some(key) = secrets.get("ANTHROPIC_API_KEY") {
    env_vars.insert("ANTHROPIC_API_KEY", key);
}

// ... other secrets

docker.create_container(ContainerConfig {
    env: env_vars,
    // ...
});
```

### Security

- Secrets are never written to disk
- Secrets are cleared from memory after container stops
- Container environment is isolated
- Secrets are passed via Docker API, not command line

## Heartbeat System

### Heartbeat Loop

The Worker sends heartbeats every 30 seconds:

```
Worker                             Main Server
   │                                    │
   ├─── worker.heartbeat ──────────────►│
   │    {                               │
   │      worker_id: "...",             │
   │      status: "idle" | "busy",      │
   │      current_task_id: "..." | null │
   │    }                               │
   │                                    │
   │◄── Heartbeat ACK ──────────────────┤
   │                                    │
```

### Health Status

| Status | Description |
|--------|-------------|
| `idle` | Ready to accept tasks |
| `busy` | Currently executing a task |
| `unhealthy` | Missed 3+ heartbeats (set by Main Server) |

### Failure Recovery

If the Worker crashes during task execution:

1. Main Server detects missed heartbeats (after 90 seconds)
2. Main Server marks Worker as `unhealthy`
3. Main Server reassigns task to another Worker
4. Original Worker's container is eventually cleaned up

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DELIDEV_MAIN_SERVER_URL` | Main Server URL | Required |
| `DELIDEV_WORKER_NAME` | Worker identifier | Hostname |
| `DELIDEV_WORKER_PORT` | Worker port | 54872 |
| `DELIDEV_DOCKER_SOCKET` | Docker socket path | `/var/run/docker.sock` |
| `DELIDEV_CONTAINER_MEMORY_LIMIT` | Container memory limit | 8g |
| `DELIDEV_CONTAINER_CPU_LIMIT` | Container CPU limit | No limit |
| `DELIDEV_LOG_LEVEL` | Log level | info |

### Container Runtime

The Worker supports both Docker and Podman:

| Runtime | Socket Path |
|---------|-------------|
| Docker | `/var/run/docker.sock` |
| Podman | `/run/user/{uid}/podman/podman.sock` |

Set `DELIDEV_CONTAINER_RUNTIME=podman` to use Podman.

## Error Handling

### Container Errors

| Error | Action |
|-------|--------|
| Container failed to start | Report failure to Main Server, cleanup |
| Agent crashed | Collect logs, report failure |
| Timeout | Kill container, report timeout failure |
| Out of memory | Kill container, report OOM failure |

### Network Errors

| Error | Action |
|-------|--------|
| Main Server unreachable | Retry with exponential backoff |
| Heartbeat failed | Continue retrying until connection restored |
| Task retrieval failed | Wait for next heartbeat cycle |

### Recovery

The Worker is designed to be stateless:
- All task state is stored in Main Server
- Worker can be restarted without losing progress
- Main Server handles task reassignment on Worker failure
