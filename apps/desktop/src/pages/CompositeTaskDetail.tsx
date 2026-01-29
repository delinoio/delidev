import { useEffect, useState, useRef, useMemo } from "react";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import {
  Loader2,
  ArrowLeft,
  ArrowRight,
  Check,
  X,
  FileCode,
  Terminal,
  ChevronDown,
  ChevronRight,
  GitBranch,
  Code,
  Trash2,
  Pencil,
} from "lucide-react";
import { CompositeTaskStatus, UnitTaskStatus, type CompositeTask, type AgentTask, type UnitTask } from "../types";
import * as api from "../api";
import type { ExecutionLog } from "../api";
import { StreamRenderer, type StreamEntry } from "../components/execution";
import { TaskGraphVisualization } from "../components/graph";
import { useTabsStore } from "../stores/tabs";
import { CollapsibleText } from "../components/ui/collapsible-text";
import { Textarea } from "../components/ui/textarea";
import yaml from "js-yaml";

interface PlanTask {
  id: string;
  prompt: string;
  dependsOn?: string[];
}

interface PlanYaml {
  tasks: PlanTask[];
}

const statusColors: Record<CompositeTaskStatus, "default" | "secondary" | "success" | "warning" | "destructive" | "info"> = {
  planning: "info",
  pending_approval: "warning",
  in_progress: "info",
  done: "success",
  rejected: "destructive",
};

const statusLabels: Record<CompositeTaskStatus, string> = {
  planning: "Planning",
  pending_approval: "Pending Approval",
  in_progress: "In Progress",
  done: "Done",
  rejected: "Rejected",
};

export function CompositeTaskDetail() {
  const { id } = useTabParams<{ id: string }>();
  const navigate = useNavigate();
  const [task, setTask] = useState<CompositeTask | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showUpdatePlanDialog, setShowUpdatePlanDialog] = useState(false);
  const [updatePlanPrompt, setUpdatePlanPrompt] = useState("");
  const [isUpdatingPlan, setIsUpdatingPlan] = useState(false);

  // Planning progress state
  const [planningAgentTask, setPlanningAgentTask] = useState<AgentTask | null>(null);
  const [executionLogs, setExecutionLogs] = useState<ExecutionLog[]>([]);
  const [streamEntries, setStreamEntries] = useState<StreamEntry[]>([]);
  const [isPlanningExpanded, setIsPlanningExpanded] = useState(true);
  const [isTaskGraphExpanded, setIsTaskGraphExpanded] = useState(true);
  const [isPlanYamlExpanded, setIsPlanYamlExpanded] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Plan YAML content state
  const [planYamlContent, setPlanYamlContent] = useState<string | null>(null);
  const [isPlanLoading, setIsPlanLoading] = useState(false);
  const [planYamlParseError, setPlanYamlParseError] = useState<string | null>(null);

  // Unit task statuses for the graph visualization
  const [unitTaskStatuses, setUnitTaskStatuses] = useState<Record<string, UnitTaskStatus>>({});

  // Unit tasks for displaying titles
  const [unitTasks, setUnitTasks] = useState<Record<string, UnitTask>>({});

  // Parse plan YAML to get tasks for graph visualization
  const planTasks = useMemo<PlanTask[]>(() => {
    if (!planYamlContent) {
      setPlanYamlParseError(null);
      return [];
    }
    try {
      const parsed = yaml.load(planYamlContent) as PlanYaml;
      if (parsed?.tasks && Array.isArray(parsed.tasks)) {
        setPlanYamlParseError(null);
        return parsed.tasks.map((t) => ({
          id: t.id,
          prompt: t.prompt,
          dependsOn: t.dependsOn,
        }));
      }
      setPlanYamlParseError("Invalid plan format: missing 'tasks' array");
      return [];
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : "Unknown parsing error";
      console.error("Failed to parse plan YAML:", e);
      setPlanYamlParseError(`Failed to parse YAML: ${errorMessage}`);
      return [];
    }
  }, [planYamlContent]);

  // Reset all task-specific state when the task id changes
  useEffect(() => {
    // Clear task state
    setTask(null);
    setError(null);

    // Clear planning state
    setPlanningAgentTask(null);
    setExecutionLogs([]);
    setStreamEntries([]);

    // Clear plan content state
    setPlanYamlContent(null);
    setPlanYamlParseError(null);

    // Clear unit task statuses and unit tasks
    setUnitTaskStatuses({});
    setUnitTasks({});
  }, [id]);

  useEffect(() => {
    if (id) {
      loadTask(id);
    }
  }, [id]);

  // Update tab title when task loads
  useEffect(() => {
    if (task?.title) {
      // Using getState() to access the store directly - not a stale closure issue
      const { tabs } = useTabsStore.getState();
      const currentPath = `/composite-tasks/${task.id}`;
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
      const result = await api.getCompositeTask(taskId);
      if (result) {
        setTask(result);

        // Load planning agent task if in planning or pending approval state
        if (result.status === CompositeTaskStatus.Planning ||
            result.status === CompositeTaskStatus.PendingApproval) {
          try {
            const agentTaskResult = await api.getAgentTask(result.planning_task_id);
            if (agentTaskResult) {
              setPlanningAgentTask(agentTaskResult);

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
                }

                // Load execution logs
                try {
                  const historicalLogs = await api.getHistoricalExecutionLogs(latestSession.id);
                  if (historicalLogs && historicalLogs.length > 0) {
                    setExecutionLogs(historicalLogs);
                  }
                } catch (err) {
                  console.error("Failed to load historical execution logs:", err);
                }
              }
            }
          } catch (err) {
            console.error("Failed to load planning agent task:", err);
          }

        }

        // Load plan YAML content - prefer persisted content from database
        if (result.plan_yaml_content) {
          // Use persisted content from the task directly
          setPlanYamlContent(result.plan_yaml_content);
        } else if (result.status === CompositeTaskStatus.PendingApproval && result.plan_file_path) {
          // Fallback: load from file via API only during pending approval
          setIsPlanLoading(true);
          try {
            const planContent = await api.getCompositeTaskPlan(taskId);
            setPlanYamlContent(planContent);
          } catch (err) {
            console.error("Failed to load plan content:", err);
          } finally {
            setIsPlanLoading(false);
          }
        }

        // Load unit tasks for graph visualization and displaying titles
        if (result.nodes && result.nodes.length > 0) {
          const statusMap: Record<string, UnitTaskStatus> = {};
          const taskMap: Record<string, UnitTask> = {};
          const unitTaskPromises = result.nodes
            .filter((node) => node.unit_task_id)
            .map((node) => api.getUnitTask(node.unit_task_id));

          try {
            const loadedUnitTasks = await Promise.all(unitTaskPromises);
            for (const unitTask of loadedUnitTasks) {
              if (unitTask) {
                statusMap[unitTask.id] = unitTask.status;
                taskMap[unitTask.id] = unitTask;
              }
            }
            setUnitTaskStatuses(statusMap);
            setUnitTasks(taskMap);
          } catch (err) {
            console.error("Failed to load unit tasks:", err);
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

  const handleStatusUpdate = async (newStatus: CompositeTaskStatus) => {
    if (!task) return;
    try {
      setIsUpdating(true);
      await api.updateCompositeTaskStatus(task.id, newStatus);
      setTask({ ...task, status: newStatus });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update status");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleApprovePlan = async () => {
    if (!task) return;
    try {
      setIsUpdating(true);
      await api.approveCompositeTaskPlan(task.id);
      // Reload task to get updated nodes and status
      await loadTask(task.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to approve plan");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleRejectPlan = async () => {
    if (!task) return;
    try {
      setIsUpdating(true);
      await api.rejectCompositeTaskPlan(task.id);
      // Reload task to get updated status
      await loadTask(task.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to reject plan");
    } finally {
      setIsUpdating(false);
    }
  };

  const handleUpdatePlan = async () => {
    if (!task || !updatePlanPrompt.trim()) return;
    try {
      setIsUpdatingPlan(true);
      // Clear existing stream entries to show fresh update progress
      setStreamEntries([]);
      setExecutionLogs([]);
      // Close the dialog before starting the update
      setShowUpdatePlanDialog(false);
      await api.updateCompositeTaskPlan(task.id, updatePlanPrompt.trim());
      // Reload task to get updated plan content
      await loadTask(task.id);
      // Clear the prompt
      setUpdatePlanPrompt("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update plan");
    } finally {
      setIsUpdatingPlan(false);
    }
  };

  const handleDelete = async () => {
    if (!task) return;
    try {
      setIsDeleting(true);
      await api.deleteCompositeTask(task.id);
      // Navigate back after successful deletion
      navigate(-1);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete task");
      setShowDeleteConfirm(false);
    } finally {
      setIsDeleting(false);
    }
  };

  // Set up event listeners for planning progress
  useEffect(() => {
    // Listen during Planning and PendingApproval states
    if (!task || (task.status !== CompositeTaskStatus.Planning && task.status !== CompositeTaskStatus.PendingApproval)) {
      return;
    }

    const registeredListeners: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      const [streamUnlisten, logUnlisten, statusUnlisten] = await Promise.all([
        // Listen for Claude stream events (structured JSON)
        // Note: Use task.id (CompositeTask ID) as that's what the backend emits events with
        api.onClaudeStream((event) => {
          if (event.task_id === task.id) {
            const entry: StreamEntry = {
              id: crypto.randomUUID(),
              timestamp: event.timestamp,
              message: event.message,
            };
            setStreamEntries((prev) => [...prev, entry]);
          }
        }),
        // Listen for execution logs (fallback for non-JSON output)
        // Note: Use task.id (CompositeTask ID) as that's what the backend emits events with
        api.onExecutionLog((event) => {
          if (event.task_id === task.id) {
            setExecutionLogs((prev) => [...prev, event.log]);
          }
        }),
        // Listen for status changes
        api.onTaskStatusChanged(async (event) => {
          if (event.task_id === task.id) {
            // Reload task to get updated status
            await loadTask(task.id);
          }
        }),
      ]);

      if (!isMounted) {
        streamUnlisten();
        logUnlisten();
        statusUnlisten();
        return;
      }

      registeredListeners.push(streamUnlisten, logUnlisten, statusUnlisten);
    };

    setupListeners().catch((error) => {
      console.error("Failed to set up event listeners:", error);
    });

    return () => {
      isMounted = false;
      registeredListeners.forEach((unlisten) => unlisten());
    };
  }, [task?.id, task?.status, task?.planning_task_id]);

  // Set up event listeners for unit task status changes
  // This auto-updates the page when a unit task within this composite task completes
  useEffect(() => {
    if (!task || task.status !== CompositeTaskStatus.InProgress) {
      return;
    }

    // Get all unit task IDs from the composite task's nodes
    const unitTaskIds = new Set(
      task.nodes
        .filter((node) => node.unit_task_id)
        .map((node) => node.unit_task_id)
    );

    // No unit tasks to monitor
    if (unitTaskIds.size === 0) {
      return;
    }

    let isMounted = true;
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      unlistenFn = await api.onTaskStatusChanged(async (event) => {
        if (!isMounted) return;

        // Check if the event is for one of our unit tasks or the composite task itself
        if (unitTaskIds.has(event.task_id) || event.task_id === task.id) {
          // Reload unit tasks for the graph visualization
          const statusMap: Record<string, UnitTaskStatus> = {};
          const taskMap: Record<string, UnitTask> = {};
          const unitTaskPromises = task.nodes
            .filter((node) => node.unit_task_id)
            .map((node) => api.getUnitTask(node.unit_task_id));

          try {
            const loadedUnitTasks = await Promise.all(unitTaskPromises);
            for (const unitTask of loadedUnitTasks) {
              if (unitTask) {
                statusMap[unitTask.id] = unitTask.status;
                taskMap[unitTask.id] = unitTask;
              }
            }
            if (isMounted) {
              setUnitTaskStatuses(statusMap);
              setUnitTasks(taskMap);
            }
          } catch (err) {
            console.error("Failed to reload unit tasks:", err);
          }

          // If the composite task status changed, reload the full task
          if (event.task_id === task.id) {
            await loadTask(task.id);
          }
        }
      });
    };

    setupListener().catch((error) => {
      console.error("Failed to set up unit task status listener:", error);
    });

    return () => {
      isMounted = false;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [task?.id, task?.status, task?.nodes]);

  // Auto-scroll logs only when user is at the bottom
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    // Check if user is at or near the bottom (within 50px threshold)
    const isAtBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight < 50;

    if (isAtBottom) {
      logsEndRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  }, [executionLogs, streamEntries]);

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

  const completedNodes = task.nodes.filter(
    (node) => node.unit_task_id
  ).length;
  const totalNodes = task.nodes.length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold line-clamp-1">{task.prompt}</h1>
            <Badge variant={statusColors[task.status]} className="shrink-0">
              {statusLabels[task.status]}
            </Badge>
          </div>
          <p className="text-muted-foreground mt-1">CompositeTask</p>
        </div>
      </div>

      {/* Task Info */}
      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Full Prompt
              </p>
              <CollapsibleText text={task.prompt} className="mt-1" />
            </div>
            {task.plan_file_path && (
              <div>
                <p className="text-sm font-medium text-muted-foreground">
                  Plan File
                </p>
                <p className="mt-1 flex items-center gap-2 font-mono text-sm">
                  <FileCode className="h-4 w-4" />
                  {task.plan_file_path}
                </p>
              </div>
            )}
            <div>
              <p className="text-sm font-medium text-muted-foreground">
                Progress
              </p>
              <p className="mt-1">
                {completedNodes}/{totalNodes} tasks completed
              </p>
            </div>
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
            {task.status === CompositeTaskStatus.PendingApproval && (
              <div className="rounded-lg border p-4 bg-warning/10 border-warning/50">
                <h4 className="font-medium mb-2">Plan Approval Required</h4>
                <p className="text-sm text-muted-foreground mb-4">
                  The AI has generated a plan for this task. Review the plan and task graph below, then approve to
                  start execution.
                </p>
                <div className="flex gap-2 flex-wrap">
                  <Button
                    variant="destructive"
                    onClick={handleRejectPlan}
                    disabled={isUpdating || isUpdatingPlan}
                  >
                    <X className="h-4 w-4" />
                    Reject
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => setShowUpdatePlanDialog(true)}
                    disabled={isUpdating || isUpdatingPlan}
                  >
                    {isUpdatingPlan ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Pencil className="h-4 w-4" />
                    )}
                    Update Plan
                  </Button>
                  <Button
                    onClick={handleApprovePlan}
                    disabled={isUpdating || isUpdatingPlan}
                  >
                    {isUpdating ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Check className="h-4 w-4" />
                    )}
                    Approve Plan
                  </Button>
                </div>
              </div>
            )}

            {task.status === CompositeTaskStatus.Planning && (
              <div className="rounded-lg border p-4">
                <h4 className="font-medium mb-2">Planning</h4>
                <p className="text-sm text-muted-foreground">
                  AI is generating a plan for this task. Please wait...
                </p>
              </div>
            )}

            {task.status === CompositeTaskStatus.InProgress && (
              <div className="rounded-lg border p-4 bg-info/10">
                <h4 className="font-medium mb-2">In Progress</h4>
                <p className="text-sm text-muted-foreground">
                  Tasks are being executed. Progress: {completedNodes}/
                  {totalNodes}
                </p>
              </div>
            )}

            {task.status === CompositeTaskStatus.Done && (
              <div className="rounded-lg border p-4 bg-green-50 border-green-200">
                <h4 className="font-medium text-green-800 mb-2">Completed</h4>
                <p className="text-sm text-green-700">
                  All tasks have been completed.
                </p>
              </div>
            )}

            {task.status === CompositeTaskStatus.Rejected && (
              <div className="rounded-lg border p-4 bg-destructive/10 border-destructive/50">
                <h4 className="font-medium mb-2">Rejected</h4>
                <p className="text-sm text-muted-foreground">
                  This task was rejected and discarded.
                </p>
              </div>
            )}

            {/* Delete Button */}
            <div className="border-t pt-4 mt-4">
              <Button
                variant="destructive"
                onClick={() => setShowDeleteConfirm(true)}
                disabled={isDeleting || isUpdating}
              >
                <Trash2 className="h-4 w-4" />
                Delete Task
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Composite Task</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this composite task? This will also delete all {task.nodes.length} unit tasks that belong to it. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowDeleteConfirm(false)}
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

      {/* Update Plan Dialog */}
      <Dialog open={showUpdatePlanDialog} onOpenChange={setShowUpdatePlanDialog}>
        <DialogContent className="sm:max-w-[525px]">
          <DialogHeader>
            <DialogTitle>Update Plan</DialogTitle>
            <DialogDescription>
              Describe how you want to modify the current plan. The AI will regenerate the plan based on your feedback.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Textarea
              placeholder="e.g., Add a task for writing tests, merge the first two tasks together, add more detail to the database migration task..."
              value={updatePlanPrompt}
              onChange={(e) => setUpdatePlanPrompt(e.target.value)}
              className="min-h-[120px] resize-y"
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowUpdatePlanDialog(false);
                setUpdatePlanPrompt("");
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleUpdatePlan}
              disabled={!updatePlanPrompt.trim()}
            >
              <Pencil className="h-4 w-4" />
              Update Plan
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Task Graph Visualization - Show plan tasks from YAML when pending approval, or actual nodes otherwise */}
      {(task.nodes.length > 0 || planTasks.length > 0) && (
        <Card>
          <CardHeader
            className="cursor-pointer select-none"
            onClick={() => setIsTaskGraphExpanded(!isTaskGraphExpanded)}
          >
            <CardTitle className="flex items-center gap-2">
              <GitBranch className="h-5 w-5" />
              Task Graph
              <span className="ml-auto">
                {isTaskGraphExpanded ? (
                  <ChevronDown className="h-5 w-5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-5 w-5 text-muted-foreground" />
                )}
              </span>
            </CardTitle>
            <CardDescription>
              {task.status === CompositeTaskStatus.PendingApproval
                ? "Preview of planned task dependencies and execution flow."
                : "Visual representation of task dependencies and execution flow."}
            </CardDescription>
          </CardHeader>
          {isTaskGraphExpanded && (
            <CardContent>
              {planYamlParseError ? (
                <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
                  <p className="text-sm text-destructive">{planYamlParseError}</p>
                </div>
              ) : (
                <TaskGraphVisualization
                  nodes={task.nodes}
                  planTasks={planTasks.length > 0 ? planTasks : undefined}
                  unitTaskStatuses={unitTaskStatuses}
                  unitTaskTitles={Object.fromEntries(
                    Object.entries(unitTasks).map(([id, t]) => [id, t.title])
                  )}
                />
              )}
            </CardContent>
          )}
        </Card>
      )}

      {/* Plan YAML Content - Show when plan content exists */}
      {(planYamlContent || isPlanLoading) && (
        <Card>
          <CardHeader
            className="cursor-pointer select-none"
            onClick={() => setIsPlanYamlExpanded(!isPlanYamlExpanded)}
          >
            <CardTitle className="flex items-center gap-2">
              <Code className="h-5 w-5" />
              Plan YAML
              <span className="ml-auto">
                {isPlanYamlExpanded ? (
                  <ChevronDown className="h-5 w-5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-5 w-5 text-muted-foreground" />
                )}
              </span>
            </CardTitle>
            <CardDescription>
              {task.status === CompositeTaskStatus.PendingApproval
                ? "Generated plan file content. Review the tasks before approval."
                : "Generated plan file content."}
            </CardDescription>
          </CardHeader>
          {isPlanYamlExpanded && (
            <CardContent>
              {isPlanLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  <span className="ml-2 text-muted-foreground">Loading plan...</span>
                </div>
              ) : planYamlContent ? (
                <pre className="rounded-lg bg-zinc-950 p-4 overflow-x-auto font-mono text-sm text-zinc-300">
                  {planYamlContent}
                </pre>
              ) : (
                <p className="text-muted-foreground text-sm">
                  Plan file not found. It may have been removed or the planning process failed.
                </p>
              )}
            </CardContent>
          )}
        </Card>
      )}

      {/* Planning Progress - Show when in planning state or has stream entries */}
      {(task.status === CompositeTaskStatus.Planning ||
        task.status === CompositeTaskStatus.PendingApproval ||
        streamEntries.length > 0 ||
        executionLogs.length > 0) && (
        <Card>
          <CardHeader
            className="cursor-pointer select-none"
            onClick={() => setIsPlanningExpanded(!isPlanningExpanded)}
          >
            <CardTitle className="flex items-center gap-2">
              <Terminal className="h-5 w-5" />
              Planning Progress
              <span className="ml-auto">
                {isPlanningExpanded ? (
                  <ChevronDown className="h-5 w-5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-5 w-5 text-muted-foreground" />
                )}
              </span>
            </CardTitle>
            <CardDescription>
              {task.status === CompositeTaskStatus.Planning
                ? "AI agent is generating a plan..."
                : task.status === CompositeTaskStatus.PendingApproval
                ? "Planning completed. Review the plan below."
                : "Planning phase output"}
            </CardDescription>
          </CardHeader>
          {isPlanningExpanded && (
            <CardContent className="space-y-4">
              {/* Stream Entries (structured Claude output) */}
              {streamEntries.length > 0 && (
                <div ref={scrollContainerRef} className="max-h-[600px] overflow-y-auto">
                  <StreamRenderer entries={streamEntries} taskStatus="in_progress" />
                  <div ref={logsEndRef} />
                </div>
              )}

              {/* Fallback Logs (non-JSON output) */}
              {executionLogs.length > 0 && streamEntries.length === 0 && (
                <div ref={scrollContainerRef} className="rounded-lg bg-zinc-950 p-4 max-h-96 overflow-y-auto font-mono text-sm">
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

              {/* Loading indicator when no logs yet */}
              {streamEntries.length === 0 && executionLogs.length === 0 && task.status === CompositeTaskStatus.Planning && (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  <span className="ml-2 text-muted-foreground">Waiting for planning output...</span>
                </div>
              )}
            </CardContent>
          )}
        </Card>
      )}

      {/* Task Nodes */}
      {task.nodes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Sub-Tasks</CardTitle>
            <CardDescription>
              Individual tasks in this composite task.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {task.nodes.map((node, index) => {
                const unitTask = node.unit_task_id ? unitTasks[node.unit_task_id] : null;
                return (
                  <div
                    key={node.id}
                    className="flex items-center justify-between rounded-lg border p-3"
                  >
                    <div className="flex items-center gap-3">
                      <span className="text-sm font-mono text-muted-foreground">
                        {index + 1}.
                      </span>
                      <span className="font-medium">{unitTask?.title ?? node.id}</span>
                      {node.depends_on.length > 0 && (
                        <span className="text-xs text-muted-foreground">
                          (depends on: {node.depends_on.map((depId) => {
                            // Find the unit task for this dependency
                            const depNode = task.nodes.find((n) => n.id === depId);
                            const depUnitTask = depNode?.unit_task_id ? unitTasks[depNode.unit_task_id] : null;
                            return depUnitTask?.title ?? depId;
                          }).join(", ")})
                        </span>
                      )}
                    </div>
                    {node.unit_task_id && (
                      <Link to={`/unit-tasks/${node.unit_task_id}`}>
                        <Button variant="ghost" size="sm">
                          View
                          <ArrowRight className="h-4 w-4" />
                        </Button>
                      </Link>
                    )}
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
