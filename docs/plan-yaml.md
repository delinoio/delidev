# PLAN.yaml Specification

When a user creates a CompositeTask, the planning agent generates a `PLAN-{randomString}.yaml` file that defines the task graph for execution.

## File Location

- **Filename**: `PLAN-{randomString}.yaml` where `{randomString}` is a unique identifier
- **Location**: Repository root directory (copied from planning worktree after generation)

## Structure

```yaml
tasks:
  - id: string          # Unique identifier for this task
    title: string       # Optional: Human-readable task title
    prompt: string      # Task description for the AI agent
    branchName: string  # Optional: Custom git branch name
    dependsOn: string[] # Optional: IDs of dependent tasks
```

## Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier within this plan |
| `title` | string | No | Human-readable title (defaults to `id`) |
| `prompt` | string | Yes | Description of what the AI agent should do |
| `branchName` | string | No | Custom git branch name (uses template if not specified) |
| `dependsOn` | string[] | No | List of task IDs that must complete first |

## Example

```yaml
tasks:
  - id: "setup-db"
    title: "Setup Database Schema"
    prompt: "Create database schema for user authentication including users, sessions, and password_reset_tokens tables"
    branchName: "feature/auth-database"

  - id: "setup-auth-utils"
    title: "Implement Auth Utilities"
    prompt: "Implement authentication utilities: password hashing, JWT token generation, and session management"
    branchName: "feature/auth-utils"
    dependsOn: ["setup-db"]

  - id: "auth-api"
    title: "REST API Endpoints"
    prompt: "Implement REST API endpoints for login, signup, logout, and password reset"
    dependsOn: ["setup-db", "setup-auth-utils"]

  - id: "auth-middleware"
    title: "Authentication Middleware"
    prompt: "Create authentication middleware for protecting routes"
    dependsOn: ["setup-auth-utils"]

  - id: "auth-ui"
    title: "Login/Signup UI"
    prompt: "Implement login and signup UI components with form validation"
    dependsOn: ["auth-api"]

  - id: "tests"
    title: "Auth Tests"
    prompt: "Write unit and integration tests for authentication system"
    dependsOn: ["auth-api", "auth-middleware"]
```

## Execution Graph

The example above produces the following execution graph:

```
setup-db ─────────────┬──► setup-auth-utils ──┬──► auth-api ──┬──► auth-ui
                      │                       │               │
                      │                       └──► auth-middleware
                      │                                       │
                      └───────────────────────────────────────┴──► tests
```

### Parallel Execution

Tasks execute in parallel when their dependencies are satisfied:

1. `setup-db` starts immediately (no dependencies)
2. `setup-auth-utils` starts when `setup-db` completes
3. `auth-api` and `auth-middleware` start in parallel when their dependencies complete
4. `auth-ui` starts when `auth-api` completes
5. `tests` starts when both `auth-api` and `auth-middleware` complete

### Execution Timeline

```
Time 0: setup-db starts
        │
Time 1: setup-db completes, setup-auth-utils starts
        │
Time 2: setup-auth-utils completes
        ├── auth-api starts
        └── auth-middleware starts (parallel)
        │
Time 3: auth-api completes, auth-ui starts
        │
Time 4: auth-middleware completes
        └── (waiting for auth-api if not done)
        │
Time 5: tests starts (both dependencies complete)
        │
Time 6: auth-ui completes
        │
Time 7: tests completes → CompositeTask done
```

## Validation Rules

### Required Rules

1. **Unique IDs**: Each task must have a unique `id` within the plan
2. **Valid References**: All IDs in `dependsOn` must reference existing task IDs
3. **No Cycles**: The dependency graph must be acyclic (DAG)
4. **Non-empty Prompt**: Each task must have a non-empty `prompt`

### Validation Errors

| Error | Description |
|-------|-------------|
| `DuplicateTaskId` | Two tasks have the same ID |
| `InvalidDependency` | `dependsOn` references a non-existent task ID |
| `CyclicDependency` | Circular dependency detected |
| `EmptyPrompt` | Task has empty or missing prompt |

## User Approval Flow

Before execution begins:

```
1. User creates CompositeTask
        ▼
2. Planning agent generates PLAN-{random}.yaml
        ▼
3. Plan copied to repository root
        ▼
4. User reviews plan in UI
        ├── View task graph visualization
        ├── Read task descriptions
        └── Check dependencies
        ▼
5. User decision:
   ├── Approve → Start execution
   ├── Edit → Modify plan manually
   └── Reject → Cancel CompositeTask
```

### UI Visualization

The plan is rendered as an interactive graph:

- **Nodes**: Each task shown with ID, title, and prompt preview
- **Edges**: Arrows showing dependencies
- **Status**: Color-coded by execution state
- **Zoom**: Pan and zoom for large graphs

## Branch Naming

### Default Behavior

If `branchName` is not specified, the system uses the branch template from repository settings:

```toml
# .delidev/config.toml
[branch]
template = "feature/${taskId}-${slug}"
```

### Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `${taskId}` | UnitTask ID | `abc123` |
| `${slug}` | URL-safe task title | `setup-database` |

### Priority

1. **Task-specific**: `branchName` in PLAN.yaml
2. **Repository template**: `.delidev/config.toml`
3. **Default**: `delidev/${taskId}`

## Best Practices

### Task Granularity

- **Too broad**: "Implement entire authentication system" → Hard to review, long execution
- **Too narrow**: "Add import statement" → Excessive overhead
- **Good**: "Implement login API endpoint with tests" → Focused, reviewable

### Dependencies

- Only add dependencies when truly necessary
- Independent tasks should run in parallel
- Avoid linear chains when parallelization is possible

### Prompts

- Be specific about requirements
- Reference existing patterns in the codebase
- Include acceptance criteria when applicable

## Generated vs Manual

| Source | Use Case |
|--------|----------|
| AI-generated | Complex tasks, initial planning |
| Manual | Simple workflows, precise control |

Users can edit AI-generated plans before approval or create plans manually for repetitive workflows.
