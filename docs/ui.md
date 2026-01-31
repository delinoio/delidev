# UI Design

DeliDev is a desktop application with the following main interfaces.

## Mode Selection

Mode selection screen shown on first start to choose between Local Mode and Server Mode.

**Note**: In development mode (`pnpm dev`), this screen is shown on every start to allow developers to easily test both modes.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Welcome to DeliDev                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Choose how you want to run DeliDev                                        │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ [Monitor Icon]                                                      │   │
│  │                                                                     │   │
│  │ Local Mode                                                          │   │
│  │ Run everything locally on your machine. All processing happens     │   │
│  │ on your computer with no external server required.                  │   │
│  │                                                                     │   │
│  │ • Full privacy - your code never leaves your machine                │   │
│  │ • No network latency                                                │   │
│  │ • Works offline (requires local AI setup)                           │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ [Server Icon]                                                       │   │
│  │                                                                     │   │
│  │ Server Mode                                                         │   │
│  │ Connect to a remote DeliDev server for task execution and          │   │
│  │ coordination.                                                       │   │
│  │                                                                     │   │
│  │ • Centralized task management                                       │   │
│  │ • Team collaboration support                                        │   │
│  │ • Offload computation to server                                     │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ── Server URL Input (shown when Server Mode selected) ──                  │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Server URL                              [ https://...           ]   │   │
│  │ Enter the URL of your DeliDev server                                │   │
│  │                                        [Test Connection]            │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  You can change this setting later in Settings                             │
│                                                          [Continue →]      │
└────────────────────────────────────────────────────────────────────────────┘
```

### Mode Selection Features

| Feature | Description |
|---------|-------------|
| Local Mode | Runs server, worker, and client all in one process (single-process mode) |
| Server Mode | Connects to a remote DeliDev server for distributed execution |
| Connection Test | Validates server URL before proceeding (Server Mode only) |
| Dev Mode Behavior | In development mode, mode selection is shown on every start |
| Persistence | Mode choice is saved and remembered for subsequent starts (production) |

### Development Mode

When running `pnpm dev`, the mode selection screen is always shown at startup. This allows developers to:
- Test Local Mode behavior
- Test Server Mode with different server URLs
- Easily switch between modes during development

To force mode selection in production, add `?force_mode_selection=true` to the URL.

---

## Onboarding

First-time setup wizard shown after mode selection when the app is launched for the first time.

### Step 1: VCS Provider Connection

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Welcome to DeliDev                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Connect your VCS Provider                                     Step 1 of 2 │
│  ─────────────────────────────────────────                                 │
│                                                                            │
│  Select a provider and enter your access token.                            │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Provider                           [ GitHub               ▼]      │   │
│  ├────────────────────────────────────────────────────────────────────┤   │
│  │ Personal Access Token              [ ghp_...               ]      │   │
│  │                                                                    │   │
│  │ Required scopes: repo, read:user, workflow                        │   │
│  │ [Create token on GitHub →]                                        │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ [✓] Connection successful                                          │   │
│  │ Authenticated as: @username                                        │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                    [Skip]       [Next →]   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Step 2: Add First Repository

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Welcome to DeliDev                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Add Your First Repository                                     Step 2 of 2 │
│  ─────────────────────────────────────────                                 │
│                                                                            │
│  Select a local git repository to get started.                             │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                                                                    │   │
│  │                    [+ Select Repository Folder]                    │   │
│  │                                                                    │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Selected: ~/projects/my-app                                        │   │
│  │                                                                    │   │
│  │ Remote: github.com/user/my-app                                     │   │
│  │ Branch: main                                                       │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  💡 You can add more repositories later from Repository Management.        │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                              [← Back]      [Get Started]   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Onboarding Features

| Feature | Description |
|---------|-------------|
| VCS Provider Selection | Dropdown to select GitHub, GitLab, or Bitbucket |
| Token Validation | Real-time validation of access token |
| Help Links | Direct links to create tokens on each provider |
| Skip Option | Users can skip VCS setup and configure later |
| Repository Picker | Native file picker to select local git repositories |

---

## Dashboard

The main view showing task status in a Kanban-style layout.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Dashboard                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────┐│
│  │ In-Progress │ │  In-Review  │ │   PR-Open   │ │    Done     │ │Rejected││
│  ├─────────────┤ ├─────────────┤ ├─────────────┤ ├─────────────┤ ├────────┤│
│  │             │ │             │ │             │ │             │ │        ││
│  │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │ │        ││
│  │ │ Task 1  │ │ │ │ Task 3  │ │ │ │ Task 5  │ │ │ │ Task 7  │ │ │        ││
│  │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │ │        ││
│  │             │ │             │ │             │ │             │ │        ││
│  │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │ │        ││
│  │ │ Task 2  │ │ │ │ Task 4  │ │ │ │ Task 6  │ │ │ │ Task 8  │ │ │        ││
│  │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │ │        ││
│  │             │ │             │ │             │ │             │ │        ││
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └────────┘│
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                            TodoItem List                                     │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Issue Triage                                                            ││
│  │ ┌────────────────────────────────────────────────────────────────────┐  ││
│  │ │ [bug] App crashes on startup  │  Suggested: bug, high-priority     │  ││
│  │ └────────────────────────────────────────────────────────────────────┘  ││
│  │                                                                         ││
│  │ PR Review                                                               ││
│  │ ┌────────────────────────────────────────────────────────────────────┐  ││
│  │ │ feat: Add dark mode  │  12 files changed  │  AI: Adds theme toggle │  ││
│  │ └────────────────────────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Kanban Columns

| Column | Description |
|--------|-------------|
| In-Progress | AI is currently working on the task |
| In-Review | AI work complete, awaiting human review |
| Approved | Human approved, ready to merge or create PR |
| PR-Open | PR created on VCS provider, awaiting merge |
| Done | PR merged, task complete |
| Rejected | Task rejected and discarded |

### Task Card

Each task card shows:
- Task title/description
- Repository name
- Current status indicator
- Progress (for CompositeTask: X/Y nodes complete)
- Quick actions (view details, open in VCS provider)
- **Not Executed indicator**: UnitTasks in "In-Progress" status without a branch are shown with a "Not Executed" warning badge and a dashed orange border

### TodoItem List

Shows human tasks that AI can assist with:
- **Issue Triage**: Unclassified issues with AI-suggested labels/assignees
- **PR Review**: PRs needing human review with AI-generated summary

## Chat Interface

Accessible via global hotkey (default: `Option+Z` / `Alt+Z`) even when app is not focused.

```
┌────────────────────────────────────────────┐
│                   Chat                      │
├────────────────────────────────────────────┤
│                                            │
│  ┌──────────────────────────────────────┐  │
│  │ User: Create a new feature to add    │  │
│  │ user authentication                   │  │
│  └──────────────────────────────────────┘  │
│                                            │
│  ┌──────────────────────────────────────┐  │
│  │ Assistant: I'll create a             │  │
│  │ CompositeTask for this. The plan     │  │
│  │ includes:                            │  │
│  │ 1. Database schema for users         │  │
│  │ 2. Auth API endpoints                │  │
│  │ 3. Login/signup UI                   │  │
│  │                                      │  │
│  │ [View PLAN.yaml] [Approve] [Edit]    │  │
│  └──────────────────────────────────────┘  │
│                                            │
├────────────────────────────────────────────┤
│  ┌──────────────────────────────────────┐  │
│  │ Type a message...          [mic] [>] │  │
│  └──────────────────────────────────────┘  │
└────────────────────────────────────────────┘
```

### Features

- **Text Input**: Type messages to interact with AI
- **Voice Input**: Click microphone or use hotkey for voice commands
- **Local AI Agent Execution**: Chat runs AI coding agents locally without Docker
  - Executes directly in user's working directory
  - No container overhead for quick interactions
  - Uses `agent.execution` settings from global config
- **Full Control**: All features accessible via chat
  - Create tasks (UnitTask, CompositeTask)
  - Review and approve tasks
  - Manage repositories
  - Configure settings

### Global Hotkey

- Opens chat window instantly from anywhere
- Configurable via `hotkey.openChat` in global settings
- App runs in background, ready for quick access

## Task Creation

Interface for creating new tasks with file context.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           Create Task                                        │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Repository Group: [ Full Stack App                    ▼]                  │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ 📁 frontend-app  📁 backend-api  📁 shared-libs                   │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Add user authentication to the app                                 │   │
│  │                                                                    │   │
│  │ Focus on @src/auth/login.ts and @src/db/schema.ts                  │   │
│  │                                                      ┌───────────┐ │   │
│  │                                                      │ src/      │ │   │
│  │                                                      │ ├─ auth/  │ │   │
│  │                                                      │ ├─ db/    │ │   │
│  │                                                      │ └─ utils/ │ │   │
│  │                                                      └───────────┘ │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Title & Branch (Optional)                                         │   │
│  │ ┌──────────────────────────────┐ ┌──────────────────────────────┐ │   │
│  │ │ Task Title                   │ │ Branch Name                  │ │   │
│  │ │ [ Add user authentication ] │ │ [ feature/add-user-auth    ] │ │   │
│  │ └──────────────────────────────┘ └──────────────────────────────┘ │   │
│  │ Leave empty for AI-generated suggestions (requires license).      │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Agent: [ Claude Code                                    ▼]                │
│         The AI coding agent to use for this task.                          │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ [✓] Composite mode                                                 │   │
│  │     Creates a CompositeTask with AI-generated plan                 │   │
│  │     Uncheck for simple single-step tasks (UnitTask)                │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                [Cancel]    [Create Task]   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Title & Branch Name

Users can optionally specify a custom task title and branch name. If left empty, AI will automatically generate contextual suggestions when creating the task.

| Field | Description |
|-------|-------------|
| Task Title | Custom task title (auto-generated with AI if empty) |
| Branch Name | Custom git branch name (auto-generated with AI if empty) |

#### Automatic AI Generation

When title or branch name fields are left empty, the system automatically:
1. Validates the user's license key
2. Calls the webapp API endpoint (`/api/generate-title-branch`)
3. Uses OpenRouter to generate contextual title and branch name
4. Falls back to simple prompt-based generation if license is invalid or API fails

**Requirements for AI generation:**
- Valid DeliDev license (validated against Polar.sh API with 5-minute caching)
- OPENROUTER_API_KEY environment variable configured in webapp

**Fallback behavior:**
If AI generation is not available (no license or API error), the title defaults to the first line of the prompt (truncated to 80 characters), and the branch name uses the repository's branch template.

### Repository Group Selection

When a repository group is selected, the repositories in the group are displayed as small tags below the dropdown. This helps users confirm which repositories will be affected by the task.

| Feature | Description |
|---------|-------------|
| Group Dropdown | Select from available repository groups |
| Repository Tags | Shows all repositories in the selected group as small tags |
| Repository Icon | Each tag displays a folder icon with the repository name |

### File Mention (@)

Type `@` to reference files in your task description, similar to Claude Code.

| Feature | Description |
|---------|-------------|
| Autocomplete | Typing `@` shows file/folder autocomplete dropdown |
| Fuzzy Search | Matches partial file names (e.g., `@login` finds `src/auth/login.ts`) |
| Multiple Files | Reference multiple files with multiple `@` mentions |
| Pass-through | Task description with `@` mentions passed as-is to AI agent |

### Agent Selection

Select which AI coding agent to use for task execution.

| Agent | Description |
|-------|-------------|
| Claude Code | Anthropic's official CLI agent using Claude models |
| OpenCode | Alternative open-source agent supporting multiple providers |

The default agent is loaded from system settings (`Settings > Agent - Execution > Agent Type`).

### Composite Mode Checkbox

| State | Task Type | Description |
|-------|-----------|-------------|
| ✓ Checked (default) | CompositeTask | AI generates a plan (PLAN.yaml) for multi-step tasks |
| ☐ Unchecked | UnitTask | Direct execution for simple single-step tasks |

### UnitTask Auto-Execution

When creating a UnitTask (Composite mode unchecked):
1. Task is created and user is navigated to the task detail page
2. System checks for Docker/Podman availability
3. If available: Execution starts automatically
4. If unavailable: Error toast notification is shown ("Docker/Podman is not available. Please start your container runtime to execute tasks.")

The user can manually start execution later from the task detail page if the initial auto-execution failed.

### Access

Task creation can be initiated via:
- Chat interface: Describe task and confirm creation
- Dashboard: "New Task" button
- Keyboard shortcut: Configurable

## Review Interface

Built-in diff viewer for reviewing AI-generated code before PR creation.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          Code Review                                        │
├────────────────────────────────────────────────────────────────────────────┤
│  Task: Add user authentication                                              │
│  Branch: feature/auth                                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Files Changed (5)                    │  src/auth/login.ts                 │
│  ┌─────────────────────────────────┐  │  ────────────────────────────────── │
│  │ [✓] src/auth/login.ts    ✓     │  │   1  + import { hash } from 'bcrypt'│
│  │ [ ] src/auth/signup.ts   (1)   │  │   2  +                              │
│  │ [✓] src/db/schema.ts     ✓     │  │   3  + export async function login( │
│  │ [ ] src/routes/auth.ts         │  │   4  +   email: string,             │
│  │ [ ] tests/auth.test.ts         │  │   5  +   password: string           │
│  └─────────────────────────────────┘  │   6  + ) {                          │
│                                       │   7  +   const user = await findUser│
│  2/5 viewed                          │   8  +   if (!user) {               │
│                                       │   9  +     throw new Error('Not fou │
│                                       │  10  +   }                          │
│                                       │  ...                                │
│                                       │                                     │
│                                       │  [Mark as viewed] [Open in Editor]  │
├────────────────────────────────────────────────────────────────────────────┤
│  Comments on this file (1):                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │ Line 7 (new): Consider adding rate limiting here    [Edit] [Delete]   ││
│  └────────────────────────────────────────────────────────────────────────┘│
│  [+ Add comment]                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  2 files viewed  │  1 comment                                               │
│  [Submit Review]  [Request Changes]  [Reject]  [Commit]  [Create PR]       │
└────────────────────────────────────────────────────────────────────────────┘
```

### Features

- **File Tree**: List of all changed files with collapsible sidebar
  - **Viewed Checkbox**: Mark files as viewed (similar to GitHub)
  - **Comment Count**: Badge showing number of inline comments per file
  - **View Status**: Checkmark indicator for reviewed files
  - **Progress Counter**: Shows "X/Y viewed" summary
- **Diff Viewer**: Side-by-side or unified diff view
  - **Mark as Viewed Button**: Toggle viewed status from the file header
  - **Open in Editor**: Open the file in your configured external editor
- **Inline Comments**: Add comments on specific files
  - **Add Comment**: Start a new comment with optional line reference
  - **Edit/Delete**: Modify or remove existing comments
  - **Comments Summary**: View all comments for the current file
- **Review Submission Dialog**: Submit review with action selection
  - **Approve**: Approve and proceed with the changes
  - **Request Changes**: Request modifications before proceeding
  - **Comment**: Leave feedback without approving or requesting changes
  - **Review Body**: Optional overall comment for the review
  - **Inline Comments Summary**: Shows all inline comments to be submitted
- **Actions**:
  - **Submit Review**: Open the review submission dialog
  - **Commit to Repository**: Merge changes directly into the current branch of the main repository
    - **Merge Strategy Selector**: Choose between merge strategies:
      - **Merge commit** (default): Create a merge commit preserving all commits
      - **Squash and merge**: Combine all commits into a single commit
      - **Rebase and merge**: Reapply commits on top of the target branch
  - **Create PR**: Create PR on VCS provider with the changes
  - **Request Changes**: Send feedback to AI for rework
  - **Reject**: Discard the task entirely

### Review State Management

The review interface maintains state per task:

| State | Description |
|-------|-------------|
| Viewed Files | Set of file paths that have been marked as viewed |
| Inline Comments | Comments attached to specific files with line references |
| Review Body | Overall comment for the review submission |
| Review Action | Selected action (Approve, Request Changes, Comment) |

This state persists while reviewing but is cleared when:
- The review is submitted
- The task is rejected
- Changes are requested (resets the task to in-progress)

### Purpose

This interface serves as the "AI slop prevention" gate:
- Review AI-generated code quality
- Track review progress with viewed files
- Provide structured feedback with inline comments
- Catch potential issues before they reach VCS provider
- Submit formal reviews like GitHub's PR review system

## Settings Interface

Configuration management interface with separate tabs for global and workspace settings.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                            Settings                                         │
├────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────────┐                                     │
│  │   Global    │  │    Workspace     │                                     │
│  └─────────────┘  └──────────────────┘                                     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Global Settings (~/.delidev/config.toml)                                  │
│  ─────────────────────────────────────────                                 │
│                                                                            │
│  Learning                                                                  │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Auto-learn from reviews                              [ ] Disabled  │   │
│  │ Automatically extract learning points from VCS provider reviews    │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Hotkey                                                                    │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Open Chat                                     [ Cmd+Shift+D    ]   │   │
│  │ Global hotkey to open chat window                                  │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Agent - Planning                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Agent Type                              [ Claude Code       ▼]    │   │
│  │ AI Model                                [ claude-sonnet-4   ▼]    │   │
│  │ Used for CompositeTask planning                                   │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Agent - Execution                                                         │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Agent Type                              [ Claude Code       ▼]    │   │
│  │ AI Model                                [ claude-sonnet-4   ▼]    │   │
│  │ Used for UnitTask execution and auto-fix                          │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Agent - Chat                                                              │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Agent Type                              [ Claude Code       ▼]    │   │
│  │ AI Model                                [ claude-sonnet-4   ▼]    │   │
│  │ Used for chat interface interactions                              │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                [Cancel]         [Save]     │
└────────────────────────────────────────────────────────────────────────────┘
```

```
┌────────────────────────────────────────────────────────────────────────────┐
│                            Settings                                         │
├────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────────┐                                     │
│  │   Global    │  │    Workspace     │  ← Selected                         │
│  └─────────────┘  └──────────────────┘                                     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Workspace Settings (.delidev/config.toml)                                 │
│  Repository: my-project                                                    │
│  ─────────────────────────────────────────                                 │
│                                                                            │
│  Docker                                                                    │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Base Image                              [ node:20-slim         ]   │   │
│  │ Docker image for agent sandbox                                     │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Automation                                                                │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Auto-fix review comments                             [✓] Enabled   │   │
│  │ Automatically apply review comments from VCS provider              │   │
│  ├────────────────────────────────────────────────────────────────────┤   │
│  │ Auto-fix CI failures                                 [✓] Enabled   │   │
│  │ Automatically fix CI failures                                      │   │
│  ├────────────────────────────────────────────────────────────────────┤   │
│  │ Max auto-fix attempts                                    [ 3  ]    │   │
│  │ Maximum number of auto-fix attempts before manual intervention     │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Learning (Override)                                                       │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ Auto-learn from reviews                      [ ] Use global (Off)  │   │
│  │                                              ( ) Override: On      │   │
│  │                                              ( ) Override: Off     │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                [Cancel]         [Save]     │
└────────────────────────────────────────────────────────────────────────────┘
```

### Tabs

| Tab | Description |
|-----|-------------|
| Global | User-wide settings stored in `~/.delidev/config.toml` |
| Workspace | Repository-specific settings stored in `.delidev/config.toml` |

### Global Settings

Settings that apply across all repositories:

| Setting | Type | Description |
|---------|------|-------------|
| Auto-learn from reviews | Toggle | Automatically learn from VCS provider reviews |
| Open Chat hotkey | Hotkey input | Global hotkey to open chat window |
| Planning Agent Type | Dropdown | AI agent for CompositeTask planning |
| Planning AI Model | Dropdown | Model for planning tasks |
| Execution Agent Type | Dropdown | AI agent for UnitTask and auto-fix |
| Execution AI Model | Dropdown | Model for execution tasks |
| Chat Agent Type | Dropdown | AI agent for chat interface |
| Chat AI Model | Dropdown | Model for chat interactions |

### Workspace Settings

Settings specific to the current repository:

| Setting | Type | Description |
|---------|------|-------------|
| Base Image | Text input | Docker image for agent sandbox |
| Auto-fix review comments | Toggle | Automatically apply review comments |
| Auto-fix CI failures | Toggle | Automatically fix CI failures |
| Max auto-fix attempts | Number input | Maximum auto-fix attempts |
| Auto-learn from reviews | Radio (inherit/override) | Override global learning setting |

### Inheritance Behavior

- Workspace settings can override global settings
- When "Use global" is selected, the global value is used
- When "Override" is selected, the workspace-specific value takes precedence
- Visual indicator shows when a setting overrides the global value

### Access

The Settings interface can be accessed via:
- Menu bar: Settings menu item
- Chat interface: "Open settings" command
- Keyboard shortcut: Configurable

## Repository Management

Interface for adding and managing repositories.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Repository Management                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Registered Repositories                                                   │
│  ─────────────────────────────────────────                                 │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ 📁 my-project          │ ~/projects/my-project    │ [Settings] [✕] │   │
│  ├────────────────────────────────────────────────────────────────────┤   │
│  │ 📁 another-repo        │ ~/work/another-repo      │ [Settings] [✕] │   │
│  ├────────────────────────────────────────────────────────────────────┤   │
│  │ 📁 frontend-app        │ ~/code/frontend-app      │ [Settings] [✕] │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│                                           [+ Add Repositories]             │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Adding Repositories

When clicking the "Add Repositories" button:

1. **File Picker Dialog** opens for folder selection
2. **Multiple Selection** is supported - users can select multiple folders at once
3. Selected folders are validated as git repositories
4. Valid repositories are added to the list

```
┌─────────────────────────────────────────────────────────────┐
│                    Select Repositories                       │
├─────────────────────────────────────────────────────────────┤
│  ◀ ▶  ~/projects                                     [⌂]   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ☑ 📁 project-a                                             │
│  ☑ 📁 project-b                                             │
│  ☐ 📁 notes              (not a git repository)             │
│  ☑ 📁 project-c                                             │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  3 folders selected                                         │
│                                           [Cancel]  [Add]   │
└─────────────────────────────────────────────────────────────┘
```

### Features

| Feature | Description |
|---------|-------------|
| File Picker Dialog | Native OS file picker for folder selection |
| Multiple Selection | Select multiple folders at once to add |
| Git Validation | Only valid git repositories can be added |
| Quick Actions | Per-repository settings and removal buttons |

### Repository Actions

| Action | Description |
|--------|-------------|
| Settings | Open workspace settings for this repository |
| Remove (✕) | Unregister the repository from DeliDev |

### Access

The Repository Management interface can be accessed via:
- Menu bar: Repositories menu item
- Chat interface: "Manage repositories" command
- Dashboard: Repository selector dropdown → "Manage..."

## Repository Groups Management

Interface for creating and managing repository groups for multi-repository tasks.

**URL**: `/repository-groups`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Repository Groups                                     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Create groups of repositories for multi-repository tasks.                 │
│                                                           [+ Create Group]  │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ 🗂 Full Stack App      │ 3 repositories     │ [Edit] [Manage] [✕] │   │
│  │                                                                    │   │
│  │ 📁 frontend-app  📁 backend-api  📁 shared-libs                   │   │
│  │                                                                    │   │
│  │                                      [+ Manage Repositories]       │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │ 🗂 Microservices       │ 5 repositories     │ [Edit] [Manage] [✕] │   │
│  │                                                                    │   │
│  │ 📁 auth-service  📁 user-service  📁 order-service  ...           │   │
│  │                                                                    │   │
│  │                                      [+ Manage Repositories]       │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Creating a Repository Group

When clicking the "Create Group" button:

```
┌─────────────────────────────────────────────────────────────┐
│                Create Repository Group                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Create a group of repositories for multi-repository tasks.  │
│                                                               │
│  Group Name                                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ My Repository Group                                     │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  Repositories                                                 │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ ☑ 📁 frontend-app           ~/projects/frontend-app    │ │
│  │ ☑ 📁 backend-api            ~/projects/backend-api     │ │
│  │ ☐ 📁 docs                   ~/projects/docs            │ │
│  │ ☑ 📁 shared-libs            ~/projects/shared-libs     │ │
│  └─────────────────────────────────────────────────────────┘ │
│  3 repositories selected                                      │
│                                                               │
│                                       [Cancel]    [Create]    │
└─────────────────────────────────────────────────────────────┘
```

### Features

| Feature | Description |
|---------|-------------|
| Create Group | Create a new repository group with a name and selected repositories |
| Edit Group | Rename an existing repository group |
| Manage Repositories | Add or remove repositories from a group |
| Delete Group | Remove a repository group (does not delete the repositories) |

### Group Types

| Type | Description |
|------|-------------|
| Single-repo groups | Automatically created when tasks target a single repository. Not shown in this UI. |
| Multi-repo groups | Named groups with multiple repositories. Created and managed in this UI. |

### Repository Group Actions

| Action | Description |
|--------|-------------|
| Edit | Change the group name |
| Manage Repositories | Add or remove repositories from the group |
| Delete (✕) | Remove the repository group |

### Use Case

Repository groups are used when creating tasks (UnitTask or CompositeTask) that need to operate across multiple repositories simultaneously. For example:
- Updating a shared library and all its consumers
- Refactoring code that spans frontend and backend repositories
- Making coordinated changes across a microservices architecture

### Access

The Repository Groups interface can be accessed via:
- Sidebar: "Repository Groups" navigation item
- Chat interface: "Manage repository groups" command

## Desktop Notifications

The app sends desktop notifications when AI agents require user attention.

### Notification Triggers

| Trigger | Description |
|---------|-------------|
| Approval Request | AI agent requests approval for a task or plan |
| User Question | AI agent asks a question to the user |
| TTY Input Request | AI agent requires user input during execution |
| Review Ready | AI work is complete and ready for human review |

### Notification Behavior

```
┌──────────────────────────────────────────────┐
│ 🤖 DeliDev                              [×]  │
├──────────────────────────────────────────────┤
│                                              │
│  AI agent needs your approval               │
│                                              │
│  Task: Add user authentication               │
│  Click to view details                       │
│                                              │
└──────────────────────────────────────────────┘
```

### Click Action

When a notification is clicked:
1. The app window is brought to focus
2. Navigation occurs to the relevant task detail page:
   - **UnitTask**: `/unit-tasks/{id}`
   - **CompositeTask**: `/composite-tasks/{id}`

### Features

| Feature | Description |
|---------|-------------|
| Native OS Integration | Uses system notification APIs (macOS, Windows, Linux) |
| Deep Linking | Clicking notification navigates directly to task detail |
| Background Support | Notifications work even when app is minimized or unfocused |
| Action Types | Uses Tauri notification action types for click handling |

### Implementation

The notification click handling is implemented using a custom native notification module that provides platform-specific click callback support:

1. **Frontend (React)**: The `useNotificationClickHandler` hook in `src/hooks/useNotificationClickHandler.ts`:
   - Listens for `notification-clicked` Tauri events emitted by the backend
   - Extracts task context from the event payload (task_type and task_id)
   - Focuses the window and navigates to the appropriate task detail page

2. **Backend (Rust)**: The notification system consists of two modules:
   - **`NativeNotificationService`** in `src-tauri/src/services/native_notification.rs`:
     - Platform-specific notification with click handler support
     - **Windows**: Uses `tauri-winrt-notification` with `on_activated` callback
     - **Linux**: Uses `notify-rust` with D-Bus action support and `wait_for_action`
     - **macOS**: Uses `osascript` for display (native click handling TODO)
     - Emits `notification-clicked` Tauri events when notifications are clicked
   - **`NotificationService`** in `src-tauri/src/services/notification.rs`:
     - High-level API for sending task notifications
     - Wraps the native service and provides task-specific notification methods

## Task Detail Pages

Detailed view for individual tasks, accessible via URL routing.

### UnitTask Detail Page

**URL**: `/unit-tasks/{id}`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          UnitTask Details                                    │
├────────────────────────────────────────────────────────────────────────────┤
│  Task: Add user authentication                                               │
│  Status: [In Review]                    Repository: my-project               │
│  Created: 2024-01-15 10:30              Branch: feature/auth                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ AI Agent Request                                              [!]   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  The AI agent is requesting approval:                               │   │
│  │                                                                     │   │
│  │  "I've completed the authentication implementation.                 │   │
│  │   Should I proceed with creating the PR?"                           │   │
│  │                                                                     │   │
│  │                              [Deny]    [Approve]                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Agent Session Log                                                         │
│  ─────────────────────────────────────────                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [10:30:15] Starting agent session...                                │   │
│  │ [10:30:20] Analyzing codebase structure                             │   │
│  │ ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │ │ 🤖 SubAgent: Explore codebase        (3 messages)         [▶]  │ │   │
│  │ └─────────────────────────────────────────────────────────────────┘ │   │
│  │ [10:35:42] Creating auth module                                     │   │
│  │ [10:40:18] Writing tests                                            │   │
│  │ [10:45:30] Requesting user approval...                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  [View Diff]        [Request Changes]        [Reject]                      │
└────────────────────────────────────────────────────────────────────────────┘
```

### CompositeTask Detail Page

**URL**: `/composite-tasks/{id}`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        CompositeTask Details                                 │
├────────────────────────────────────────────────────────────────────────────┤
│  Task: Build e-commerce checkout system                                      │
│  Status: [Pending Approval]             Repository: my-shop                  │
│  Created: 2024-01-15 09:00                                                   │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Plan Approval Required                                        [!]   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  The AI has generated a plan for this task.                         │   │
│  │  Please review and approve to proceed.                              │   │
│  │                                                                     │   │
│  │  [View PLAN.yaml]                     [Reject]    [Approve Plan]    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Task Graph (Interactive - powered by xyflow/react)                        │
│  ─────────────────────────────────────────                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │   ┌─────────────┐                                                   │   │
│  │   │ setup-db    │──────┐                                            │   │
│  │   │ [Pending]   │      │                                            │   │
│  │   └─────────────┘      │    ┌──────────────┐    ┌─────────────┐     │   │
│  │                        ├───►│ api-endpoints│───►│  frontend   │     │   │
│  │   ┌─────────────┐      │    │  [Pending]   │    │  [Pending]  │     │   │
│  │   │ setup-auth  │──────┘    └──────────────┘    └─────────────┘     │   │
│  │   │ [Pending]   │                                                   │   │
│  │   └─────────────┘                                                   │   │
│  │                                                                     │   │
│  │                    [Zoom Controls] [MiniMap]                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  Sub-Tasks                                                                 │
│  ─────────────────────────────────────────                                 │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │ 1. setup-db       │ Set up database schema      │ [Pending]   [→]  │    │
│  ├────────────────────────────────────────────────────────────────────┤    │
│  │ 2. setup-auth     │ Set up authentication       │ [Pending]   [→]  │    │
│  ├────────────────────────────────────────────────────────────────────┤    │
│  │ 3. api-endpoints  │ Implement API endpoints     │ [Pending]   [→]  │    │
│  ├────────────────────────────────────────────────────────────────────┤    │
│  │ 4. frontend       │ Implement frontend          │ [Pending]   [→]  │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  Progress: 0/4 tasks complete                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

#### Task Graph Visualization

The task graph is rendered using `@xyflow/react` and provides an interactive visualization of task dependencies:

| Feature | Description |
|---------|-------------|
| Nodes | Each task is displayed as a node with its ID, prompt preview, and status |
| Edges | Animated edges show dependencies between tasks with arrow markers |
| Status Colors | Nodes are color-coded based on their unit task status (Pending, In Progress, In Review, Done, Rejected) |
| Zoom Controls | Users can zoom in/out and fit the view to see all tasks |
| MiniMap | A minimap provides an overview for larger task graphs |
| Collapsible | The graph section can be collapsed/expanded via the header |

The graph automatically positions nodes using topological sorting to ensure dependencies flow left-to-right.

### Features

| Feature | Description |
|---------|-------------|
| Deep Linking | Direct URL access to any task via `/unit-tasks/{id}` or `/composite-tasks/{id}` |
| Approval UI | Inline approval/denial interface for AI agent requests |
| TTY Input Dialog | Web form interface for answering agent questions during execution |
| Live Updates | Real-time status updates as AI works |
| Task Graph | Visual representation of task dependencies (CompositeTask) |
| Session Log | Scrollable log of AI agent activity |
| Collapsible Execution Logs | Execution Progress card is collapsible; collapsed by default for in-review tasks |
| Collapsible SubAgents | SubAgent (Task tool) logs are grouped and collapsible to reduce noise |
| Stop Execution | Stop button to cancel running AI agent execution (UnitTask) |
| Git Diff Viewer | Shows git diff of changes made by AI agent when task is in review (uses `@pierre/diffs`) |
| Branch Rename | Edit button to rename the branch for UnitTasks before execution starts |
| Token Usage Card | Displays aggregated token usage statistics (cost, duration, turns, sessions) for completed tasks |

## TTY Input Dialog

When an AI agent requires user input during execution, a dialog appears in the Task Detail page.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         TTY Input Request                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Agent Question                                               [!]   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  The AI agent is asking:                                            │   │
│  │                                                                     │   │
│  │  "Which authentication method should I implement?"                  │   │
│  │                                                                     │   │
│  │  ┌───────────────────────────────────────────────────────────────┐  │   │
│  │  │ Options:                                                      │  │   │
│  │  │                                                               │  │   │
│  │  │  ○ JWT with refresh tokens                                    │  │   │
│  │  │  ○ Session-based authentication                               │  │   │
│  │  │  ○ OAuth 2.0 with social providers                            │  │   │
│  │  │                                                               │  │   │
│  │  │ Or provide a custom response:                                 │  │   │
│  │  │ ┌─────────────────────────────────────────────────────────┐   │  │   │
│  │  │ │                                                         │   │  │   │
│  │  │ └─────────────────────────────────────────────────────────┘   │  │   │
│  │  └───────────────────────────────────────────────────────────────┘  │   │
│  │                                                                     │   │
│  │                                    [Cancel]    [Submit Response]    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Input Types

| Type | Description | UI Component |
|------|-------------|--------------|
| Text | Free-form text input | Text area |
| Confirm | Yes/No confirmation | Two buttons |
| Select | Selection from options | Radio button list |

### Behavior

1. **Notification**: Desktop notification is shown when input is requested
2. **Focus**: Clicking notification brings app to focus and scrolls to dialog
3. **Blocking**: Agent execution is paused until user responds
4. **Timeout**: Requests can optionally timeout after a configurable period
5. **Cancel**: User can cancel to stop the current execution

### Integration

The TTY Input Dialog integrates with the existing Task Detail page:
- Appears above the Agent Session Log when a request is pending
- Highlighted with a distinct border to draw attention
- Automatically dismissed after response is submitted

### Navigation

- Click on a sub-task `[→]` button to navigate to its UnitTask detail page
- Breadcrumb navigation: Dashboard → CompositeTask → UnitTask
- Back button returns to previous view

---

## Keyboard Shortcuts

Complete list of keyboard shortcuts available in DeliDev.

### Global Shortcuts

These shortcuts work even when the app is not focused.

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Option+Z` (macOS) / `Alt+Z` (Win/Linux) | Open Chat | Opens the chat interface from anywhere |

### Application Shortcuts

These shortcuts work when the app is focused.

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Cmd+N` / `Ctrl+N` | New Task | Opens the task creation dialog |
| `Cmd+,` / `Ctrl+,` | Settings | Opens the settings interface |
| `Cmd+K` / `Ctrl+K` | Command Palette | Opens the command palette |
| `Cmd+1` / `Ctrl+1` | Dashboard | Navigate to dashboard |
| `Cmd+2` / `Ctrl+2` | Repositories | Navigate to repository management |
| `Escape` | Close Dialog | Closes the current dialog or panel |

### Review Interface Shortcuts

| Shortcut | Action | Description |
|----------|--------|-------------|
| `J` / `K` | Navigate Files | Move up/down in file list |
| `Enter` | Open File | Open selected file in diff viewer |
| `Cmd+Enter` / `Ctrl+Enter` | Approve | Approve and create PR |
| `Cmd+Shift+R` / `Ctrl+Shift+R` | Request Changes | Send feedback for rework |

### Task Detail Shortcuts

| Shortcut | Action | Description |
|----------|--------|-------------|
| `A` | Approve | Approve the pending request |
| `D` | Deny | Deny the pending request |
| `L` | Toggle Log | Show/hide agent session log |
| `S` | Stop Execution | Stop the running AI agent execution |

### Tab Navigation Shortcuts

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Ctrl+T` / `Cmd+T` | New Tab | Opens a new dashboard tab |
| `Ctrl+W` / `Cmd+W` | Close Tab | Closes current tab (when only one tab, redirects to dashboard and clears history) |
| `Ctrl+Tab` / `Cmd+Tab` | Next Tab | Switch to the next tab |
| `Ctrl+Shift+Tab` / `Cmd+Shift+Tab` | Previous Tab | Switch to the previous tab |
| `Ctrl+1-9` / `Cmd+1-9` | Switch Tab | Switch to tab by index (9 = last tab) |
| `Ctrl+Click` / `Cmd+Click` | Open in New Tab | Opens task in a new tab |
| `Middle Click` | Close Tab | Closes the clicked tab |

### Customization

Global shortcuts can be customized via Settings → Global → Hotkey.

---

## Multi-Tab Interface

DeliDev supports a multi-tab interface for managing multiple tasks simultaneously.

### Tab Bar

The tab bar appears at the top of the main content area when multiple tabs are open.

```
┌────────────────────────────────────────────────────────────────────────────┐
│  [Dashboard]  [Task: Add auth ×]  [Task: Fix bug ×]                        │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  (Tab content displayed here)                                              │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Tab Features

| Feature | Description |
|---------|-------------|
| Ctrl/Cmd+Click | Opens any task card in a new tab |
| Middle Click | Closes the clicked tab |
| Tab Title | Displays the task title, auto-updated when task loads |
| Close Button | Appears on hover to close individual tabs |
| Tab Icons | Different icons for UnitTask, CompositeTask, Settings, etc. |

### Tab Types

| Type | Icon | Description |
|------|------|-------------|
| Dashboard | Home | Main dashboard view |
| UnitTask | ListTodo | Individual unit task details |
| CompositeTask | Layers | Composite task with sub-tasks |
| Repositories | FolderGit2 | Repository management |
| Settings | Settings | Global or repository settings |
| Chat | MessageSquare | Chat interface |

### Tab Behavior

- **Single Tab Mode**: When only one tab is open, the tab bar is hidden
- **Regular Navigation**: Normal clicks replace the current tab's content
- **New Tab Navigation**: Ctrl/Cmd+Click opens content in a new tab
- **Tab Persistence**: Navigating within a tab updates that tab's content
- **Tab Close**: Cannot close the last remaining tab
