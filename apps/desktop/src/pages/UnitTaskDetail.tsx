import { useEffect, useState, useRef } from "react";
import { useNavigate, Link } from "react-router-dom";
import { useTabParams } from "../hooks";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import {
  Loader2,
  ArrowLeft,
  GitBranch,
  ExternalLink,
  Check,
  X,
  RotateCcw,
  Play,
  Terminal,
  RefreshCw,
  CheckCircle2,
  XCircle,
  Circle,
  Square,
  FileCode,
  FolderGit2,
  Trash2,
  ChevronDown,
  ChevronRight,
  Pencil,
  Layers,
} from "lucide-react";
import { UnitTaskStatus, type UnitTask, type AgentTask } from "../types";
import * as api from "../api";
import type { MergeStrategy } from "../api";
import { useRepositoriesStore } from "../stores/repositories";
import { useRepositoryGroupsStore } from "../stores/repositoryGroups";
import { useTabsStore } from "../stores/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import type { ExecutionLog, ClaudeStreamEvent, TtyInputRequest } from "../api";
import { StreamRenderer, type StreamEntry, TtyInputDialog } from "../components/execution";
import { DiffViewer, ReviewSubmitDialog } from "../components/diff";
import { RequestChangesDialog, TokenUsageCard } from "../components/tasks";
import { Select } from "../components/ui/select";
import { Input } from "../components/ui/input";
import { CollapsibleText } from "../components/ui/collapsible-text";
import { useReviewStore, ReviewAction, type InlineComment } from "../stores";
import { MessageSquare } from "lucide-react";

const statusColors: Record<
  UnitTaskStatus,
  "default" | "secondary" | "success" | "warning" | "destructive" | "info"
> = {
  in_progress: "info",
  in_review: "warning",
  approved: "success",
  pr_open: "secondary",
  done: "success",
  rejected: "destructive",
};

const statusLabels: Record<UnitTaskStatus, string> = {
  in_progress: "In Progress",
  in_review: "In Review",
  approved: "Approved",
  pr_open: "PR Open",
  done: "Done",
  rejected: "Rejected",
};

const phaseLabels: Record<string, string> = {
  starting: "Initializing",
  worktree: "Creating Git Worktree",
  container: "Creating Docker Container",
  setup: "Running Setup Commands",
  executing: "Running Agent",
  completed: "Completed",
  failed: "Failed",
  cleanup: "Cleaning Up",
};

const phaseOrder = [
  "starting",
  "worktree",
  "container",
  "setup",
  "executing",
  "cleanup",
];

export function UnitTaskDetail() {
  const { id } = useTabParams<{ id: string }>();
  const navigate = useNavigate();
  const [task, setTask] = useState<UnitTask | null>(null);
  const [agentTask, setAgentTask] = useState<AgentTask | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const { repositories, fetchRepositories, hasFetched } = useRepositoriesStore();
  const { groups, fetchGroups, hasFetched: groupsHasFetched } = useRepositoryGroupsStore();
  const [error, setError] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [dockerAvailable, setDockerAvailable] = useState(false);
  const [isCheckingDocker, setIsCheckingDocker] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isCheckingExecution, setIsCheckingExecution] = useState(false);

  // Diff state
  const [diff, setDiff] = useState<string | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);

  // TTY input state
  const [ttyInputRequest, setTtyInputRequest] = useState<TtyInputRequest | null>(null);
  const [isSubmittingTtyInput, setIsSubmittingTtyInput] = useState(false);

  // Merge strategy state
  const [mergeStrategy, setMergeStrategy] = useState<MergeStrategy>("merge");

  // Review dialog state
  const [isReviewDialogOpen, setIsReviewDialogOpen] = useState(false);

  // Request changes dialog state
  const [isRequestChangesDialogOpen, setIsRequestChangesDialogOpen] = useState(false);

  // Branch editing state
  const [isEditingBranch, setIsEditingBranch] = useState(false);
  const [editedBranchName, setEditedBranchName] = useState("");
  const [isRenamingBranch, setIsRenamingBranch] = useState(false);

  // Review store
  const { getAllComments, getViewedFilesCount, clearTaskReview } = useReviewStore();

  // Execution state
  const [executionLogs, setExecutionLogs] = useState<ExecutionLog[]>([]);
  const [streamEntries, setStreamEntries] = useState<StreamEntry[]>([]);
  const [currentPhase, setCurrentPhase] = useState<string | null>(null);
  const [progressMessage, setProgressMessage] = useState<string>("");
  const [isExecuting, setIsExecuting] = useState(false);
  const [isLogsExpanded, setIsLogsExpanded] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);
  // Track if execution was stopped by user to prevent race conditions with incoming events
  const stoppedByUserRef = useRef(false);

  // Reset all task-specific state when the task id changes
  useEffect(() => {
    // Clear execution state
    setExecutionLogs([]);
    setStreamEntries([]);
    setCurrentPhase(null);
    setProgressMessage("");
    setIsExecuting(false);
    setIsLogsExpanded(true);
    stoppedByUserRef.current = false;

    // Clear task state
    setTask(null);
    setAgentTask(null);
    setError(null);

    // Clear diff state
    setDiff(null);

    // Clear TTY input state
    setTtyInputRequest(null);
  }, [id]);

  useEffect(() => {
    if (id) {
      loadTask(id);
      checkDocker();
    }
    if (!hasFetched) {
      fetchRepositories();
    }
    if (!groupsHasFetched) {
      fetchGroups();
    }
  }, [id, hasFetched, fetchRepositories, groupsHasFetched, fetchGroups]);

  // Re-check Docker availability when window gains focus
  useEffect(() => {
    const handleFocus = () => {
      checkDocker();
    };

    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, []);

  // Set up event listeners when task is in progress
  useEffect(() => {
    if (!task || task.status !== UnitTaskStatus.InProgress) {
      return;
    }

    // Accumulate all registered listeners to avoid race condition during cleanup
    const registeredListeners: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      // Register all listeners using Promise.all to minimize race condition window
      const [streamUnlisten, logUnlisten, progressUnlisten, statusUnlisten, ttyInputUnlisten] =
        await Promise.all([
          // Listen for Claude stream events (structured JSON)
          api.onClaudeStream((event) => {
            if (event.task_id === task.id) {
              // Ignore events if user has stopped execution
              if (stoppedByUserRef.current) return;
              const entry: StreamEntry = {
                id: crypto.randomUUID(),
                timestamp: event.timestamp,
                message: event.message,
              };
              setStreamEntries((prev) => [...prev, entry]);
              setIsExecuting(true);
            }
          }),
          // Listen for execution logs (fallback for non-JSON output)
          api.onExecutionLog((event) => {
            if (event.task_id === task.id) {
              // Ignore events if user has stopped execution
              if (stoppedByUserRef.current) return;
              setExecutionLogs((prev) => [...prev, event.log]);
              setIsExecuting(true);
            }
          }),
          // Listen for progress updates
          api.onExecutionProgress((event) => {
            if (event.task_id === task.id) {
              // Ignore events if user has stopped execution (except for completed/failed which confirm the stop)
              if (stoppedByUserRef.current && event.phase !== "completed" && event.phase !== "failed") {
                return;
              }
              setCurrentPhase(event.phase);
              setProgressMessage(event.message);
              if (!stoppedByUserRef.current) {
                setIsExecuting(true);
              }

              if (event.phase === "completed" || event.phase === "failed") {
                setIsExecuting(false);
                // Clear any pending TTY input requests when execution ends
                setTtyInputRequest(null);
                // Reset the stopped flag when execution actually ends
                stoppedByUserRef.current = false;
              }
            }
          }),
          // Listen for status changes
          api.onTaskStatusChanged((event) => {
            if (event.task_id === task.id) {
              // Reload task to get updated status
              loadTask(task.id);
            }
          }),
          // Listen for TTY input requests
          api.onTtyInputRequest((event) => {
            if (event.request.task_id === task.id) {
              setTtyInputRequest(event.request);
            }
          }),
        ]);

      // Check isMounted once after all listeners are registered
      if (!isMounted) {
        // Clean up all listeners if component unmounted during setup
        streamUnlisten();
        logUnlisten();
        progressUnlisten();
        statusUnlisten();
        ttyInputUnlisten();
        return;
      }

      // Store all listeners for cleanup
      registeredListeners.push(
        streamUnlisten,
        logUnlisten,
        progressUnlisten,
        statusUnlisten,
        ttyInputUnlisten
      );
    };

    setupListeners().catch((error) => {
      console.error("Failed to set up event listeners:", error);
    });

    return () => {
      isMounted = false;
      // Clean up all registered listeners
      registeredListeners.forEach((unlisten) => unlisten());
    };
  }, [task?.id, task?.status]);

  // Auto-scroll logs
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [executionLogs, streamEntries]);

  // Update tab title when task loads
  useEffect(() => {
    if (task?.title) {
      // Using getState() to access the store directly - not a stale closure issue
      const { tabs } = useTabsStore.getState();
      const currentPath = `/unit-tasks/${task.id}`;
      const currentTab = tabs.find((t) => t.path === currentPath);
      if (currentTab) {
        useTabsStore.getState().updateTabTitle(currentTab.id, task.title);
      }
    }
  }, [task?.id, task?.title]);

  const loadTask = async (taskId: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const result = await api.getUnitTask(taskId);
      if (result) {
        setTask(result);

        // Fetch agent task information
        try {
          const agentTaskResult = await api.getAgentTask(result.agent_task_id);
          if (agentTaskResult) {
            setAgentTask(agentTaskResult);

            // Load historical stream messages and execution logs if we have any agent sessions
            if (agentTaskResult.agent_sessions && agentTaskResult.agent_sessions.length > 0) {
              // Get the most recent session (usually the last one)
              const latestSession = agentTaskResult.agent_sessions[agentTaskResult.agent_sessions.length - 1];

              // Load stream messages
              try {
                const historicalMessages = await api.getStreamMessages(latestSession.id);
                if (historicalMessages && historicalMessages.length > 0) {
                  // Convert to StreamEntry format
                  const entries: StreamEntry[] = historicalMessages.map(msg => ({
                    id: msg.id,
                    timestamp: msg.timestamp,
                    message: msg.message,
                  }));
                  setStreamEntries(entries);
                }
              } catch (err) {
                console.error("Failed to load historical stream messages:", err);
                // Don't fail the whole page if loading historical messages fails
              }

              // Load execution logs
              try {
                const historicalLogs = await api.getHistoricalExecutionLogs(latestSession.id);
                if (historicalLogs && historicalLogs.length > 0) {
                  setExecutionLogs(historicalLogs);
                }
              } catch (err) {
                console.error("Failed to load historical execution logs:", err);
                // Don't fail the whole page if loading historical logs fails
              }
            }
          }
        } catch (err) {
          console.error("Failed to load agent task:", err);
          // Don't fail the whole page if agent task loading fails
        }

        // Check if the task is currently executing (e.g., after page refresh)
        if (result.status === UnitTaskStatus.InProgress) {
          setIsCheckingExecution(true);
          try {
            const executing = await api.isTaskExecuting(taskId);
            if (executing) {
              // Reset stopped flag since execution is actually running
              stoppedByUserRef.current = false;
              setIsExecuting(true);
              // Set a generic phase to indicate execution is in progress
              // The actual phase will be updated by event listeners
              if (!currentPhase) {
                setCurrentPhase("executing");
                setProgressMessage("Execution in progress...");
              }
            } else if (result.last_execution_failed) {
              // Task is not currently executing but had a failed execution
              // Set the phase to failed so the UI shows the failure state
              setCurrentPhase("failed");
              setProgressMessage("Execution failed");
            }
          } finally {
            setIsCheckingExecution(false);
          }
        }
      } else {
        setError("Task not found");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load task");
    } finally {
      setIsLoading(false);
    }
  };

  const checkDocker = async () => {
    try {
      setIsCheckingDocker(true);
      const available = await api.isDockerAvailable();
      setDockerAvailable(available);
    } catch {
      setDockerAvailable(false);
    } finally {
      setIsCheckingDocker(false);
    }
  };

  const loadDiff = async () => {
    if (!task) return;
    try {
      setIsDiffLoading(true);
      const result = await api.getTaskDiff(task.id);
      setDiff(result);
    } catch (err) {
      console.error("Failed to load diff:", err);
    } finally {
      setIsDiffLoading(false);
    }
  };

  // Load diff when task is in review, approved, pr_open, or done
  useEffect(() => {
    if (
      task?.status === UnitTaskStatus.InReview ||
      task?.status === UnitTaskStatus.Approved ||
      task?.status === UnitTaskStatus.PrOpen ||
      task?.status === UnitTaskStatus.Done
    ) {
      loadDiff();
    }
  }, [task?.status]);

  // Collapse execution logs by default for in-review or approved tasks
  useEffect(() => {
    if (task?.status === UnitTaskStatus.InReview || task?.status === UnitTaskStatus.Approved) {
      setIsLogsExpanded(false);
    }
  }, [task?.status]);

  const handleStartExecution = async () => {
    if (!task) return;
    try {
      setIsStarting(true);
      setExecutionLogs([]);
      setStreamEntries([]);
      setCurrentPhase(null);
      setProgressMessage("");
      // Reset the stopped flag when starting a new execution
      stoppedByUserRef.current = false;
      await api.startTaskExecution(task.id);
      setIsExecuting(true);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to start execution"
      );
    } finally {
      setIsStarting(false);
    }
  };

  const handleStopExecution = async () => {
    if (!task) return;
    try {
      setIsStopping(true);
      // Set the stopped flag BEFORE calling the API to prevent race conditions
      // with incoming events
      stoppedByUserRef.current = true;
      await api.stopTaskExecution(task.id);
      setIsExecuting(false);
      setCurrentPhase(null);
      setProgressMessage("Execution stopped by user");
    } catch (err) {
      // Reset the flag on error so user can try again
      stoppedByUserRef.current = false;
      setError(
        err instanceof Error ? err.message : "Failed to stop execution"
      );
    } finally {
      setIsStopping(false);
    }
  };

  const handleStatusUpdate = async (newStatus: UnitTaskStatus) => {
    if (!task) return;
    try {
      setIsUpdating(true);
      await api.updateUnitTaskStatus(task.id, newStatus);
      setTask({ ...task, status: newStatus });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update status");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleCreatePr = async () => {
    if (!task) return;
    try {
      setIsUpdating(true);
      setError(null);
      const prUrl = await api.createPrForTask(task.id);
      setTask({ ...task, status: UnitTaskStatus.Done, linked_pr_url: prUrl });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create PR");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleCommitToRepository = async () => {
    if (!task) return;
    try {
      setIsUpdating(true);
      setError(null);
      await api.commitToRepository(task.id, mergeStrategy);
      setTask({ ...task, status: UnitTaskStatus.Done });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to commit to repository");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleDelete = async () => {
    if (!task) return;
    try {
      setIsDeleting(true);
      setError(null);
      await api.deleteUnitTask(task.id);
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete task");
      setIsDeleteDialogOpen(false);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleStartEditBranch = () => {
    if (!task) return;
    setEditedBranchName(task.branch_name || "");
    setIsEditingBranch(true);
  };

  const handleCancelEditBranch = () => {
    setIsEditingBranch(false);
    setEditedBranchName("");
  };

  const handleSaveBranchName = async () => {
    if (!task || !editedBranchName.trim()) return;
    try {
      setIsRenamingBranch(true);
      setError(null);
      await api.renameUnitTaskBranch(task.id, editedBranchName.trim());
      setTask({ ...task, branch_name: editedBranchName.trim() });
      setIsEditingBranch(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to rename branch");
    } finally {
      setIsRenamingBranch(false);
    }
  };

  const handleTtyInputSubmit = async (response: string) => {
    if (!ttyInputRequest) return;
    try {
      setIsSubmittingTtyInput(true);
      await api.submitTtyInput(ttyInputRequest.id, response);
      setTtyInputRequest(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit response");
    } finally {
      setIsSubmittingTtyInput(false);
    }
  };

  const handleTtyInputCancel = async () => {
    if (!ttyInputRequest) return;
    try {
      await api.cancelTtyInput(ttyInputRequest.id);
      setTtyInputRequest(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to cancel request");
    }
  };

  const handleRequestChanges = async (feedback: string) => {
    if (!task) return;

    try {
      setIsUpdating(true);
      setError(null);

      // Call the API to request changes with feedback
      await api.requestUnitTaskChanges(task.id, feedback);

      // Reload the task to get updated prompt and status
      await loadTask(task.id);

      // Clear review state since we're requesting changes
      clearTaskReview(task.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to request changes");
      throw err;
    } finally {
      setIsUpdating(false);
    }
  };

  const handleReviewSubmit = async (
    action: ReviewAction,
    body: string,
    comments: InlineComment[]
  ) => {
    if (!task) return;

    try {
      setIsUpdating(true);
      setError(null);

      // Build the feedback message with inline comments
      let feedbackMessage = body;
      if (comments.length > 0) {
        const commentsSummary = comments
          .map((c) => `- ${c.filePath}:${c.line}: ${c.body}`)
          .join("\n");
        feedbackMessage = feedbackMessage
          ? `${feedbackMessage}\n\n## Inline Comments\n${commentsSummary}`
          : `## Inline Comments\n${commentsSummary}`;
      }

      switch (action) {
        case ReviewAction.Approve:
          // When approving, we can either create a PR or commit directly
          // For now, we show the existing actions and close the dialog
          break;
        case ReviewAction.RequestChanges:
          // Request changes: use the new API to append feedback to prompt
          await api.requestUnitTaskChanges(task.id, feedbackMessage);
          // Reload the task to get updated prompt and status
          await loadTask(task.id);
          // Clear review state since we're requesting changes
          clearTaskReview(task.id);
          break;
        case ReviewAction.Comment:
          // Comment only: keep the status as is, just log the feedback
          // In a real implementation, this would store the comments
          break;
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit review");
      throw err;
    } finally {
      setIsUpdating(false);
    }
  };

  const getPhaseIcon = (phase: string) => {
    if (!currentPhase) return <Circle className="h-4 w-4 text-muted-foreground" />;

    const currentIndex = phaseOrder.indexOf(currentPhase);
    const phaseIndex = phaseOrder.indexOf(phase);

    if (currentPhase === "completed") {
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    }
    if (currentPhase === "failed") {
      if (phaseIndex <= currentIndex) {
        return <XCircle className="h-4 w-4 text-destructive" />;
      }
      return <Circle className="h-4 w-4 text-muted-foreground" />;
    }

    if (phaseIndex < currentIndex) {
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    }
    if (phaseIndex === currentIndex) {
      return <Loader2 className="h-4 w-4 animate-spin text-primary" />;
    }
    return <Circle className="h-4 w-4 text-muted-foreground" />;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !task) {
    return (
      <div className="space-y-4">
        <Button variant="ghost" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">{error || "Task not found"}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold line-clamp-1">{task.title}</h1>
            <Badge variant={statusColors[task.status]} className="shrink-0">
              {statusLabels[task.status]}
            </Badge>
          </div>
          <p className="text-muted-foreground mt-1">UnitTask</p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setIsDeleteDialogOpen(true)}
          className="text-muted-foreground hover:text-destructive"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>

      {/* Delete Confirmation Dialog */}
      <Dialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Task</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this task? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsDeleteDialogOpen(false)}
              disabled={isDeleting}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={isDeleting}
            >
              {isDeleting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4" />
              )}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Task Info */}
      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Repository
              </p>
              <p className="mt-1 flex items-center gap-2">
                <FolderGit2 className="h-4 w-4" />
                {(() => {
                  const group = groups.find((g) => g.id === task.repository_group_id);
                  const repo = group && group.repository_ids.length > 0
                    ? repositories.find((r) => r.id === group.repository_ids[0])
                    : undefined;
                  return repo ? (
                    <Link
                      to={`/repositories/${repo.id}`}
                      className="text-primary hover:underline"
                    >
                      {repo.name}
                    </Link>
                  ) : (
                    <span className="text-muted-foreground">Unknown</span>
                  );
                })()}
              </p>
            </div>
            {task.composite_task_id && (
              <div>
                <p className="text-sm font-medium text-muted-foreground">
                  Part of CompositeTask
                </p>
                <p className="mt-1 flex items-center gap-2">
                  <Layers className="h-4 w-4" />
                  <Link
                    to={`/composite-tasks/${task.composite_task_id}`}
                    className="text-primary hover:underline"
                  >
                    View CompositeTask
                  </Link>
                </p>
              </div>
            )}
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Full Prompt
              </p>
              <CollapsibleText text={task.prompt} className="mt-1" />
            </div>
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Branch
              </p>
              {isEditingBranch ? (
                <div className="mt-1 flex items-center gap-2">
                  <GitBranch className="h-4 w-4 shrink-0" />
                  <Input
                    value={editedBranchName}
                    onChange={(e) => setEditedBranchName(e.target.value)}
                    className="h-8 flex-1"
                    placeholder="Enter branch name"
                    autoFocus
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        handleSaveBranchName();
                      } else if (e.key === "Escape") {
                        handleCancelEditBranch();
                      }
                    }}
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={handleSaveBranchName}
                    disabled={isRenamingBranch || !editedBranchName.trim()}
                  >
                    {isRenamingBranch ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Check className="h-4 w-4" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={handleCancelEditBranch}
                    disabled={isRenamingBranch}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              ) : (
                <div className="mt-1 flex items-center gap-2">
                  <GitBranch className="h-4 w-4" />
                  {task.branch_name ? (
                    <span>{task.branch_name}</span>
                  ) : (
                    <span className="text-muted-foreground italic">Not set</span>
                  )}
                  {task.status === UnitTaskStatus.InProgress && !isExecuting && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={handleStartEditBranch}
                      title="Edit branch name"
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                  )}
                </div>
              )}
            </div>
            {task.linked_pr_url && (
              <div>
                <p className="text-sm font-medium text-muted-foreground">
                  Pull Request
                </p>
                <a
                  href={task.linked_pr_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-1 flex items-center gap-2 text-primary hover:underline"
                >
                  <ExternalLink className="h-4 w-4" />
                  View PR
                </a>
              </div>
            )}
            {agentTask && (agentTask.ai_agent_type || agentTask.ai_agent_model) && (
              <div>
                <p className="text-sm font-medium text-muted-foreground">
                  Agent
                </p>
                <p className="mt-1">
                  {agentTask.ai_agent_type === "claude_code" && "Claude Code"}
                  {agentTask.ai_agent_type === "open_code" && "OpenCode"}
                  {!agentTask.ai_agent_type && "Default Agent"}
                  {agentTask.ai_agent_model && (
                    <span className="text-muted-foreground text-sm ml-1">
                      ({agentTask.ai_agent_model})
                    </span>
                  )}
                </p>
              </div>
            )}
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Created
              </p>
              <p className="mt-1">
                {new Date(task.created_at).toLocaleString()}
              </p>
            </div>
          </CardContent>
        </Card>

        {/* Actions */}
        <Card>
          <CardHeader>
            <CardTitle>Actions</CardTitle>
            <CardDescription>
              Update the task status or take action.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {task.status === UnitTaskStatus.InReview && (
              <div className="rounded-lg border p-4 bg-warning/10 border-warning/50">
                <h4 className="font-medium mb-2">Review Required</h4>
                <p className="text-sm text-muted-foreground mb-4">
                  AI work is complete. Review the changes and decide whether to
                  approve or request changes.
                </p>

                {/* Review Stats */}
                <div className="flex items-center gap-4 mb-4 text-sm text-muted-foreground">
                  <span>
                    {getViewedFilesCount(task.id)} files viewed
                  </span>
                  <span>
                    {getAllComments(task.id).length} comments
                  </span>
                </div>

                <div className="flex flex-wrap gap-2 items-center">
                  <Button
                    variant="outline"
                    onClick={() => setIsRequestChangesDialogOpen(true)}
                    disabled={isUpdating}
                  >
                    <RotateCcw className="h-4 w-4" />
                    Request Changes
                  </Button>
                  <Button
                    variant="destructive"
                    onClick={() => handleStatusUpdate(UnitTaskStatus.Rejected)}
                    disabled={isUpdating}
                  >
                    <X className="h-4 w-4" />
                    Reject
                  </Button>
                  <Button
                    onClick={() => handleStatusUpdate(UnitTaskStatus.Approved)}
                    disabled={isUpdating}
                  >
                    {isUpdating ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Check className="h-4 w-4" />
                    )}
                    Approve
                  </Button>
                </div>
              </div>
            )}

            {task.status === UnitTaskStatus.Approved && (
              <div className="rounded-lg border p-4 bg-green-50 border-green-200">
                <h4 className="font-medium text-green-800 mb-2">Approved</h4>
                <p className="text-sm text-green-700 mb-4">
                  This task has been approved. You can now merge the changes or create a PR.
                </p>

                <div className="flex flex-wrap gap-2 items-center">
                  <div className="flex items-center gap-2">
                    <Select
                      value={mergeStrategy}
                      onChange={(e) => setMergeStrategy(e.target.value as MergeStrategy)}
                      options={[
                        { value: "merge", label: "Merge commit" },
                        { value: "squash", label: "Squash and merge" },
                        { value: "rebase", label: "Rebase and merge" },
                      ]}
                      className="w-44"
                    />
                    <Button
                      onClick={handleCommitToRepository}
                      disabled={isUpdating}
                    >
                      {isUpdating ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <GitBranch className="h-4 w-4" />
                      )}
                      Merge
                    </Button>
                  </div>
                  <Button
                    variant="outline"
                    onClick={handleCreatePr}
                    disabled={isUpdating}
                  >
                    {isUpdating ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <ExternalLink className="h-4 w-4" />
                    )}
                    Create PR
                  </Button>
                </div>
              </div>
            )}

            {task.status === UnitTaskStatus.InProgress && (
              <div className="rounded-lg border p-4">
                <h4 className="font-medium mb-2">In Progress</h4>
                {isCheckingExecution ? (
                  <div className="flex items-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    <p className="text-sm text-muted-foreground">
                      Checking execution status...
                    </p>
                  </div>
                ) : !isExecuting && !currentPhase ? (
                  <>
                    <p className="text-sm text-muted-foreground mb-4">
                      {dockerAvailable
                        ? "Ready to start execution. Click the button below to run the agent."
                        : "Docker/Podman is not available. Please start your container runtime to execute tasks."}
                    </p>
                    <div className="flex gap-2">
                      <Button
                        onClick={handleStartExecution}
                        disabled={!dockerAvailable || isStarting}
                      >
                        {isStarting ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Play className="h-4 w-4" />
                        )}
                        Start Execution
                      </Button>
                      {!dockerAvailable && (
                        <Button
                          variant="outline"
                          onClick={checkDocker}
                          disabled={isCheckingDocker}
                        >
                          {isCheckingDocker ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <RefreshCw className="h-4 w-4" />
                          )}
                          Refresh Status
                        </Button>
                      )}
                    </div>
                  </>
                ) : (
                  <>
                    <p className="text-sm text-muted-foreground mb-4">
                      AI agent is working on this task. See progress below.
                    </p>
                    <Button
                      variant="destructive"
                      onClick={handleStopExecution}
                      disabled={isStopping}
                    >
                      {isStopping ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <Square className="h-4 w-4" />
                      )}
                      Stop Execution
                    </Button>
                  </>
                )}
              </div>
            )}

            {task.status === UnitTaskStatus.PrOpen && (
              <div className="rounded-lg border p-4 bg-secondary/50">
                <h4 className="font-medium mb-2">PR Created</h4>
                <p className="text-sm text-muted-foreground mb-4">
                  Pull request has been created. Waiting for merge.
                </p>
                {task.linked_pr_url && (
                  <Button variant="outline" >
                    <a
                      href={task.linked_pr_url}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="h-4 w-4" />
                      View PR
                    </a>
                  </Button>
                )}
              </div>
            )}

            {task.status === UnitTaskStatus.Done && (
              <div className="rounded-lg border p-4 bg-green-50 border-green-200">
                <h4 className="font-medium text-green-800 mb-2">Completed</h4>
                <p className="text-sm text-green-700">
                  This task has been completed and merged.
                </p>
              </div>
            )}

            {task.status === UnitTaskStatus.Rejected && (
              <div className="rounded-lg border p-4 bg-destructive/10 border-destructive/50">
                <h4 className="font-medium mb-2">Rejected</h4>
                <p className="text-sm text-muted-foreground">
                  This task was rejected and discarded.
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Review Submit Dialog */}
      {task.status === UnitTaskStatus.InReview && (
        <ReviewSubmitDialog
          open={isReviewDialogOpen}
          onOpenChange={setIsReviewDialogOpen}
          taskId={task.id}
          onSubmit={handleReviewSubmit}
        />
      )}

      {/* Request Changes Dialog */}
      {task.status === UnitTaskStatus.InReview && (
        <RequestChangesDialog
          open={isRequestChangesDialogOpen}
          onOpenChange={setIsRequestChangesDialogOpen}
          onSubmit={handleRequestChanges}
        />
      )}

      {/* TTY Input Request - Show when agent is asking a question */}
      {ttyInputRequest && (
        <TtyInputDialog
          request={ttyInputRequest}
          onSubmit={handleTtyInputSubmit}
          onCancel={handleTtyInputCancel}
          isSubmitting={isSubmittingTtyInput}
        />
      )}

      {/* Execution Progress - Show when in progress or has logs */}
      {(task.status === UnitTaskStatus.InProgress ||
        executionLogs.length > 0 ||
        streamEntries.length > 0 ||
        currentPhase) && (
        <Card>
          <CardHeader
            className="cursor-pointer select-none"
            onClick={() => setIsLogsExpanded(!isLogsExpanded)}
          >
            <CardTitle className="flex items-center gap-2">
              <Terminal className="h-5 w-5" />
              Execution Progress
              <span className="ml-auto">
                {isLogsExpanded ? (
                  <ChevronDown className="h-5 w-5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-5 w-5 text-muted-foreground" />
                )}
              </span>
            </CardTitle>
            <CardDescription>
              {isExecuting
                ? "Agent is running..."
                : currentPhase === "completed"
                ? "Execution completed successfully"
                : currentPhase === "failed"
                ? "Execution failed"
                : "Waiting for execution to start"}
            </CardDescription>
          </CardHeader>
          {isLogsExpanded && (
            <CardContent className="space-y-4">
              {/* Progress Steps */}
              {currentPhase && (
                <div className="flex items-center gap-2 flex-wrap">
                  {phaseOrder.map((phase, index) => (
                    <div key={phase} className="flex items-center gap-1">
                      {getPhaseIcon(phase)}
                      <span
                        className={`text-sm ${
                          currentPhase === phase
                            ? "font-medium text-foreground"
                            : "text-muted-foreground"
                        }`}
                      >
                        {phaseLabels[phase]}
                      </span>
                      {index < phaseOrder.length - 1 && (
                        <span className="mx-2 text-muted-foreground">→</span>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {/* Current Status Message */}
              {progressMessage && (
                <div className="rounded-lg bg-muted p-3">
                  <p className="text-sm">{progressMessage}</p>
                </div>
              )}

              {/* Stream Entries (structured Claude output) */}
              {streamEntries.length > 0 && (
                <div className="max-h-[600px] overflow-y-auto">
                  <StreamRenderer entries={streamEntries} taskStatus={task.status} />
                  <div ref={logsEndRef} />
                </div>
              )}

              {/* Fallback Logs (non-JSON output) */}
              {executionLogs.length > 0 && streamEntries.length === 0 && (
                <div className="rounded-lg bg-zinc-950 p-4 max-h-96 overflow-y-auto font-mono text-sm">
                  {executionLogs.map((log) => (
                    <div
                      key={log.id}
                      className={`${
                        log.level === "error"
                          ? "text-red-400"
                          : log.level === "warn"
                          ? "text-yellow-400"
                          : "text-zinc-300"
                      }`}
                    >
                      <span className="text-zinc-500 mr-2">
                        {new Date(log.timestamp).toLocaleTimeString()}
                      </span>
                      <span className="whitespace-pre-wrap">{log.message}</span>
                    </div>
                  ))}
                  <div ref={logsEndRef} />
                </div>
              )}
            </CardContent>
          )}
        </Card>
      )}

      {/* Changes (Git Diff) - Show when in_review, approved, pr_open, or done */}
      {(task.status === UnitTaskStatus.InReview ||
        task.status === UnitTaskStatus.Approved ||
        task.status === UnitTaskStatus.PrOpen ||
        task.status === UnitTaskStatus.Done) && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileCode className="h-5 w-5" />
              Changes
            </CardTitle>
            <CardDescription>
              Git diff of changes made by the AI agent.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {isDiffLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : diff ? (
              <div className="max-h-[600px] overflow-y-auto rounded-lg border p-4">
                <DiffViewer
                  diff={diff}
                  repoPath={(() => {
                    const group = groups.find((g) => g.id === task.repository_group_id);
                    const repo = group && group.repository_ids.length > 0
                      ? repositories.find((r) => r.id === group.repository_ids[0])
                      : undefined;
                    return repo?.local_path;
                  })()}
                  baseCommit={task.base_commit}
                  headCommit={task.end_commit}
                  taskId={task.id}
                  enableReviewFeatures={task.status === UnitTaskStatus.InReview}
                />
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">
                No changes to display. Worktree may have been cleaned up.
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Token Usage */}
      <TokenUsageCard taskId={task.id} taskType="unit" />

      {/* Auto-fix Attempts */}
      {task.auto_fix_task_ids.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Auto-fix Attempts</CardTitle>
            <CardDescription>
              Automatic fixes attempted for this task.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2">
              {task.auto_fix_task_ids.map((taskId, index) => (
                <li
                  key={taskId}
                  className="flex items-center gap-2 text-sm text-muted-foreground"
                >
                  <span className="font-mono">{index + 1}.</span>
                  <span>{taskId}</span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
