import React from "react";
import { useNavigate } from "react-router-dom";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Badge } from "../ui/badge";
import { ExternalLink, GitBranch, AlertCircle, XCircle, FolderGit2 } from "lucide-react";
import type { UnitTask, UnitTaskStatus, Repository } from "../../types";
import { UnitTaskStatus as Status } from "../../types";
import { useRepositoriesStore } from "../../stores/repositories";
import { useRepositoryGroupsStore } from "../../stores/repositoryGroups";
import { useTabNavigation } from "../../hooks/useTabNavigation";
import { TabType } from "../../stores/tabs";

interface TaskCardProps {
  task: UnitTask;
  repository?: Repository;
}

const statusColors: Record<UnitTaskStatus, "default" | "secondary" | "success" | "warning" | "destructive" | "info"> = {
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

function TaskCardComponent({ task, repository: repositoryProp }: TaskCardProps) {
  const { repositories } = useRepositoriesStore();
  const { groups } = useRepositoryGroupsStore();
  const navigate = useNavigate();
  const { handleClick } = useTabNavigation();
  // Use prop if provided, otherwise look up via repository group
  const repository = repositoryProp ?? (() => {
    const group = groups.find((g) => g.id === task.repository_group_id);
    if (group && group.repository_ids.length > 0) {
      return repositories.find((r) => r.id === group.repository_ids[0]);
    }
    return undefined;
  })();

  // A task is considered not executed if it's in_progress but has no branch_name
  const isNotExecuted = task.status === Status.InProgress && !task.branch_name;
  // A task has failed execution if it's in_progress and the last execution failed
  const isExecutionFailed = task.status === Status.InProgress && task.last_execution_failed;

  const taskPath = `/unit-tasks/${task.id}`;

  const handleCardClick = (e: React.MouseEvent) => {
    // Check for Ctrl/Cmd+Click to open in new tab
    const handled = handleClick(e, {
      path: taskPath,
      title: task.title,
      type: TabType.UnitTask,
      taskId: task.id,
    });

    if (!handled) {
      // Regular click - navigate normally
      navigate(taskPath);
    }
  };

  return (
    <Card
      className={`cursor-pointer transition-shadow hover:shadow-md ${isNotExecuted ? "border-orange-400 border-dashed" : ""} ${isExecutionFailed ? "border-red-500 border-dashed" : ""}`}
      onClick={handleCardClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <CardTitle className="text-sm font-medium line-clamp-2">
            {task.title}
          </CardTitle>
          <div className="flex items-center gap-1 shrink-0">
            {isExecutionFailed && (
              <Badge variant="destructive" className="gap-1">
                <XCircle className="h-3 w-3" />
                Execution Failed
              </Badge>
            )}
            {!isExecutionFailed && isNotExecuted && (
              <Badge variant="warning" className="gap-1">
                <AlertCircle className="h-3 w-3" />
                Not Executed
              </Badge>
            )}
            {!isExecutionFailed && !isNotExecuted && (
              <Badge variant={statusColors[task.status]}>
                {statusLabels[task.status]}
              </Badge>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {repository && (
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <FolderGit2 className="h-3 w-3" />
            <span className="truncate">{repository.name}</span>
          </div>
        )}
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          {task.branch_name && (
            <span className="flex items-center gap-1">
              <GitBranch className="h-3 w-3" />
              {task.branch_name}
            </span>
          )}
          {task.linked_pr_url && (
            <a
              href={task.linked_pr_url}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 hover:text-foreground"
              onClick={(e) => e.stopPropagation()}
            >
              <ExternalLink className="h-3 w-3" />
              PR
            </a>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export const TaskCard = React.memo(TaskCardComponent);
