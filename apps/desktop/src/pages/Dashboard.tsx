import { useEffect } from "react";
import { KanbanBoard } from "../components/tasks/KanbanBoard";
import { useTasksStore } from "../stores/tasks";
import { useRepositoriesStore } from "../stores/repositories";
import { useWorkspacesStore } from "../stores/workspaces";
import { useRepositoryGroupsStore } from "../stores/repositoryGroups";
import { Loader2 } from "lucide-react";
import { UnitTaskStatus } from "../types";
import { CompositeTaskStatusKey, onTaskStatusChanged } from "../api";

export function Dashboard() {
  const { tasksByStatus, fetchTasksByStatus, isLoading, error } = useTasksStore();
  const { fetchRepositories, hasFetched } = useRepositoriesStore();
  const { selectedWorkspaceId } = useWorkspacesStore();
  const { fetchGroups, hasFetched: groupsHasFetched } = useRepositoryGroupsStore();

  useEffect(() => {
    fetchTasksByStatus(selectedWorkspaceId ?? undefined);
    if (!hasFetched) {
      fetchRepositories();
    }
    if (!groupsHasFetched) {
      fetchGroups(selectedWorkspaceId ?? undefined);
    }
  }, [fetchTasksByStatus, fetchRepositories, hasFetched, selectedWorkspaceId, fetchGroups, groupsHasFetched]);

  // Listen for task status changes to automatically move items on the dashboard
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    let isMounted = true;

    const setupListener = async () => {
      try {
        const unlisten = await onTaskStatusChanged(() => {
          if (isMounted) {
            fetchTasksByStatus(selectedWorkspaceId ?? undefined);
          }
        });
        if (isMounted) {
          unlistenFn = unlisten;
        } else {
          // Component unmounted during setup, clean up immediately
          unlisten();
        }
      } catch (error) {
        console.error("Failed to set up task status listener:", error);
      }
    };

    setupListener();

    return () => {
      isMounted = false;
      unlistenFn?.();
    };
  }, [fetchTasksByStatus, selectedWorkspaceId]);

  if (isLoading && !tasksByStatus) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
        <p className="text-sm text-destructive">{error}</p>
      </div>
    );
  }

  const tasks = tasksByStatus || {
    [UnitTaskStatus.InProgress]: [],
    [UnitTaskStatus.InReview]: [],
    [UnitTaskStatus.Approved]: [],
    [UnitTaskStatus.PrOpen]: [],
    [UnitTaskStatus.Done]: [],
    [UnitTaskStatus.Rejected]: [],
    [CompositeTaskStatusKey.InProgress]: [],
    [CompositeTaskStatusKey.InReview]: [],
    [CompositeTaskStatusKey.Done]: [],
    [CompositeTaskStatusKey.Rejected]: [],
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-muted-foreground">
          Manage your AI agent tasks across all repositories.
        </p>
      </div>

      <KanbanBoard tasksByStatus={tasks} />
    </div>
  );
}
