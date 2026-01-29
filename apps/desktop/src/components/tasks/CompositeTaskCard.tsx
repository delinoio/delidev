import React from "react";
import { useNavigate } from "react-router-dom";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Badge } from "../ui/badge";
import { Layers, FolderGit2 } from "lucide-react";
import type { CompositeTask, CompositeTaskStatus, Repository } from "../../types";
import { useRepositoriesStore } from "../../stores/repositories";
import { useRepositoryGroupsStore } from "../../stores/repositoryGroups";
import { useTabNavigation } from "../../hooks/useTabNavigation";
import { TabType } from "../../stores/tabs";

interface CompositeTaskCardProps {
  task: CompositeTask;
  repository?: Repository;
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

function CompositeTaskCardComponent({ task, repository: repositoryProp }: CompositeTaskCardProps) {
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

  const taskPath = `/composite-tasks/${task.id}`;

  const handleCardClick = (e: React.MouseEvent) => {
    // Check for Ctrl/Cmd+Click to open in new tab
    const handled = handleClick(e, {
      path: taskPath,
      title: task.title,
      type: TabType.CompositeTask,
      taskId: task.id,
    });

    if (!handled) {
      // Regular click - navigate normally
      navigate(taskPath);
    }
  };

  return (
    <Card
      className="cursor-pointer transition-shadow hover:shadow-md border-l-4 border-l-primary"
      onClick={handleCardClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2">
            <Layers className="h-4 w-4 text-primary shrink-0" />
            <CardTitle className="text-sm font-medium line-clamp-2">
              {task.title}
            </CardTitle>
          </div>
          <Badge variant={statusColors[task.status]} className="shrink-0">
            {statusLabels[task.status]}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {repository && (
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <FolderGit2 className="h-3 w-3" />
            <span className="truncate">{repository.name}</span>
          </div>
        )}
        {task.nodes.length > 0 && (
          <div className="text-xs text-muted-foreground">
            {task.nodes.length} subtask{task.nodes.length !== 1 ? "s" : ""}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export const CompositeTaskCard = React.memo(CompositeTaskCardComponent);
