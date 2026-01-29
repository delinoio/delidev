# PLAN.yaml Specification

When a user creates a CompositeTask, the planningTask generates a `PLAN-{randomString}.yaml` file that defines the task graph.

The filename format is `PLAN-{randomString}.yaml` where `{randomString}` is a unique identifier to distinguish between multiple plans.

## Structure

```yaml
tasks:
  - id: string          # Unique identifier for this task
    title: string       # Optional: Human-readable task title (defaults to id)
    prompt: string      # Task description for the AI agent
    branchName: string  # Optional: Custom git branch name for this task
    dependsOn: string[] # Optional: IDs of tasks this depends on
```

## Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Y | Unique identifier within this plan |
| title | string | N | Human-readable title for the task (defaults to `id` if not specified) |
| prompt | string | Y | Description of what the AI agent should do |
| branchName | string | N | Custom git branch name for this task (uses template if not specified) |
| dependsOn | string[] | N | List of task IDs that must complete before this task starts |

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

The above example produces the following execution graph:

```
setup-db ─────────────┬──► setup-auth-utils ──┬──► auth-api ──┬──► auth-ui
                      │                       │               │
                      │                       └──► auth-middleware
                      │                                       │
                      └───────────────────────────────────────┴──► tests
```

Parallel execution:
1. `setup-db` starts immediately
2. `setup-auth-utils` starts when `setup-db` completes
3. `auth-api` and `auth-middleware` start in parallel when their dependencies complete
4. `auth-ui` starts when `auth-api` completes
5. `tests` starts when both `auth-api` and `auth-middleware` complete

## Validation Rules

1. **Unique IDs**: Each task must have a unique `id` within the plan
2. **Valid References**: All IDs in `dependsOn` must reference existing task IDs
3. **No Cycles**: The dependency graph must be acyclic (DAG)
4. **Non-empty Prompt**: Each task must have a non-empty `prompt`

## User Approval

Before execution:
1. planningTask generates PLAN-{randomString}.yaml
2. User reviews the plan in the UI
3. User can:
   - **Approve**: Start execution
   - **Edit**: Modify the plan (add/remove/reorder tasks)
   - **Reject**: Cancel the CompositeTask
