# Local Mode vs Remote Mode

DeliDev supports two execution modes for desktop clients: Local Mode and Remote Mode. Mobile clients only support Remote Mode.

## Mode Comparison

| Aspect | Local Mode | Remote Mode |
|--------|------------|-------------|
| **Architecture** | Single process | Distributed |
| **Server** | Embedded | Remote Main Server |
| **Worker** | Embedded | Remote Worker Server(s) |
| **Database** | SQLite | PostgreSQL |
| **Authentication** | Disabled | JWT + OIDC |
| **Network** | Not required | Required |
| **Secrets** | Direct keychain | Sent via RPC |
| **Docker** | Local machine | Worker machine |
| **Collaboration** | Single user | Multi-user |

## Platform Support

| Platform | Local Mode | Remote Mode |
|----------|------------|-------------|
| Desktop (Windows) | Yes | Yes |
| Desktop (macOS) | Yes | Yes |
| Desktop (Linux) | Yes | Yes |
| Mobile (iOS) | No | Yes |
| Mobile (Android) | No | Yes |

## Local Mode

### When to Use

- **Solo development**: Working alone on projects
- **Offline work**: No internet connection available
- **Privacy**: Code stays on local machine
- **Low latency**: No network round trips
- **Simple setup**: No server configuration needed

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Desktop App (Single Process)                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────┐  ┌─────────────────────┐              │
│  │   Embedded Server   │  │   Embedded Worker   │              │
│  │                     │  │                     │              │
│  │  - Task store       │  │  - AI agent exec    │              │
│  │  - Local SQLite     │  │  - Docker mgmt      │              │
│  │  - No auth          │  │  - Git worktrees    │              │
│  └─────────────────────┘  └─────────────────────┘              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Frontend (WebView)                    │   │
│  │  Tauri invoke() ──► Direct function calls                │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  Local Resources                         │   │
│  │  SQLite │ Keychain │ Docker │ Git Repos                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User creates task
        ▼
Frontend calls Tauri command
        ▼
Embedded Server stores in SQLite
        ▼
Embedded Worker picks up task
        ▼
Worker reads secrets from local keychain
        ▼
Worker runs Docker container locally
        ▼
AI agent executes
        ▼
Results stored in SQLite
        ▼
Frontend updates via Tauri events
```

### Characteristics

| Feature | Behavior |
|---------|----------|
| Startup | Fast, no connection needed |
| Auth | None (trusted local user) |
| Data | SQLite in app data directory |
| Secrets | Read directly from OS keychain |
| Docker | Uses local Docker/Podman |
| Git | Direct access to local repos |
| Concurrency | Single worker (configurable) |

## Remote Mode

### When to Use

- **Team collaboration**: Multiple users sharing tasks
- **Resource offloading**: Execute on powerful servers
- **Mobile access**: View and manage tasks from phone
- **Centralized management**: Single source of truth
- **Scalability**: Multiple workers for parallel execution

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Client (Desktop/Mobile)                     │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Frontend (WebView)                    │   │
│  │  react-query ──► Connect RPC ──► Main Server            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────┐                                       │
│  │  Local Keychain     │  (Secrets sent to server)             │
│  └─────────────────────┘                                       │
│                                                                 │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                    Connect RPC over HTTPS
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Main Server                              │
│                                                                 │
│  ┌───────────┐  ┌───────────┐  ┌───────────────────────────┐   │
│  │  Auth     │  │  Task     │  │  Worker                   │   │
│  │  Module   │  │  Store    │  │  Registry                 │   │
│  └───────────┘  └───────────┘  └───────────────────────────┘   │
│                                                                 │
│  PostgreSQL Database                                            │
│                                                                 │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                                ▼
    ┌───────────────────────────────────────────────────────────┐
    │                    Worker Server(s)                        │
    │                                                            │
    │  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
    │  │  Worker 1  │  │  Worker 2  │  │  Worker N  │           │
    │  │  (Docker)  │  │  (Docker)  │  │  (Docker)  │           │
    │  └────────────┘  └────────────┘  └────────────┘           │
    │                                                            │
    └───────────────────────────────────────────────────────────┘
```

### Data Flow

```
User creates task
        ▼
Frontend sends Connect RPC request
        ▼
Main Server authenticates (JWT)
        ▼
Main Server stores in PostgreSQL
        ▼
Main Server notifies available Worker
        ▼
Client sends secrets to Main Server
        ▼
Main Server relays secrets to Worker
        ▼
Worker runs Docker container
        ▼
AI agent executes
        ▼
Worker reports status to Main Server
        ▼
Main Server broadcasts to clients
        ▼
Frontend updates via WebSocket events
```

### Characteristics

| Feature | Behavior |
|---------|----------|
| Startup | Requires connection to server |
| Auth | JWT token from OIDC login |
| Data | PostgreSQL on Main Server |
| Secrets | Sent from client to server to worker |
| Docker | Runs on Worker Server machines |
| Git | Workers clone/access repos |
| Concurrency | Multiple workers in parallel |

## Mode Selection

### Desktop

Users select mode on first launch (or in development on every launch):

```
┌────────────────────────────────────────────────────────────────┐
│                     Choose Mode                                 │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  [Monitor Icon]  Local Mode                               │  │
│  │                                                          │  │
│  │  Run everything locally on your machine.                 │  │
│  │  • Full privacy - code stays on your machine             │  │
│  │  • No network latency                                    │  │
│  │  • Works offline                                         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  [Server Icon]  Remote Mode                               │  │
│  │                                                          │  │
│  │  Connect to a remote DeliDev server.                     │  │
│  │  • Team collaboration                                    │  │
│  │  • Offload computation to server                         │  │
│  │  • Access from multiple devices                          │  │
│  │                                                          │  │
│  │  Server URL: [ https://delidev.example.com       ]       │  │
│  │                                   [Test Connection]       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│                                            [Continue →]        │
└────────────────────────────────────────────────────────────────┘
```

### Mobile

Mobile apps automatically use Remote Mode:

```
┌────────────────────────────────────────────────────────────────┐
│                     Connect to Server                           │
│                                                                │
│  Enter your DeliDev server URL:                                │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ https://delidev.example.com                              │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│                    [Test Connection]                           │
│                                                                │
│                                            [Continue →]        │
└────────────────────────────────────────────────────────────────┘
```

## Why Mobile Only Supports Remote Mode

Mobile platforms have fundamental limitations that prevent local mode:

### Technical Limitations

| Limitation | Impact |
|------------|--------|
| **No Docker** | Cannot run containerized AI agents |
| **File System** | Limited/sandboxed access to files |
| **Background Execution** | OS kills long-running tasks |
| **Resource Constraints** | CPU, memory, battery limits |
| **Git Operations** | Require full file system access |

### User Experience

| Factor | Local (if possible) | Remote |
|--------|---------------------|--------|
| Battery | Heavy drain | Minimal impact |
| Storage | Large repos consume space | Only metadata |
| Heat | CPU-intensive | Offloaded |
| Responsiveness | Degraded during execution | Always responsive |

### Practical Usage

Mobile is best suited for:
- Monitoring task progress
- Reviewing code changes
- Approving/rejecting tasks
- Responding to agent questions
- Quick task creation

Heavy lifting happens on remote servers:
- AI agent execution
- Docker container management
- Git operations
- Large file handling

## Switching Modes

### Desktop

Users can switch modes in Settings:

1. Open Settings (`Cmd+,` / `Ctrl+,`)
2. Navigate to "Connection" section
3. Select new mode
4. Enter server URL (if switching to Remote)
5. Restart app

### Data Migration

| Direction | Behavior |
|-----------|----------|
| Local → Remote | Data stays in local SQLite, new data goes to server |
| Remote → Local | Local SQLite starts fresh (or import option) |

**Note**: Tasks are not automatically synced between modes. Export/import functionality may be added in future versions.

## Development

### Environment Variables

```bash
# Force mode (skip selection)
PUBLIC_DEFAULT_MODE=local   # or 'remote'
PUBLIC_SKIP_MODE_SELECTION=true

# Remote mode settings
PUBLIC_REMOTE_SERVER_URL=http://localhost:54871
```

### Scripts

```bash
# Show mode selection (default)
pnpm dev

# Force local mode
pnpm dev:local

# Force remote mode
PUBLIC_REMOTE_SERVER_URL=http://localhost:54871 pnpm dev:remote
```

## Security Considerations

### Local Mode

| Aspect | Status |
|--------|--------|
| Auth | None (trusted local) |
| Secrets | Native keychain |
| Data | Local SQLite |
| Network | None required |
| Isolation | Docker containers |

### Remote Mode

| Aspect | Status |
|--------|--------|
| Auth | JWT + OIDC |
| Secrets | Encrypted transport (TLS) |
| Data | PostgreSQL with auth |
| Network | HTTPS/WSS required |
| Isolation | Docker on worker |

### Secret Handling

| Mode | Secret Flow |
|------|-------------|
| Local | Keychain → Environment → Docker |
| Remote | Keychain → RPC (TLS) → Server (memory) → Worker → Docker |

In Remote Mode, secrets are:
- Read from local keychain on client
- Sent via TLS-encrypted RPC
- Cached in server memory (not persisted)
- Cleared after task completion
