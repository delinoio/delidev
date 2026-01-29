import { useMemo, memo } from "react";
import { TaskCard } from "./TaskCard";
import { CompositeTaskCard } from "./CompositeTaskCard";
import { UnitTaskStatus, type UnitTask, type CompositeTask, type Repository } from "../../types";
import { CompositeTaskStatusKey, type TasksByStatus } from "../../api";
import { useRepositoriesStore } from "../../stores/repositories";
import { useRepositoryGroupsStore } from "../../stores/repositoryGroups";

interface KanbanBoardProps {
  tasksByStatus: TasksByStatus;
}

const columns: { status: UnitTaskStatus; label: string; unitKey: UnitTaskStatus; compositeKey?: CompositeTaskStatusKey }[] = [
  { status: UnitTaskStatus.InProgress, label: "In Progress", unitKey: UnitTaskStatus.InProgress, compositeKey: CompositeTaskStatusKey.InProgress },
  { status: UnitTaskStatus.InReview, label: "In Review", unitKey: UnitTaskStatus.InReview, compositeKey: CompositeTaskStatusKey.InReview },
  { status: UnitTaskStatus.Approved, label: "Approved", unitKey: UnitTaskStatus.Approved },
  { status: UnitTaskStatus.PrOpen, label: "PR Open", unitKey: UnitTaskStatus.PrOpen },
  { status: UnitTaskStatus.Done, label: "Done", unitKey: UnitTaskStatus.Done, compositeKey: CompositeTaskStatusKey.Done },
  { status: UnitTaskStatus.Rejected, label: "Rejected", unitKey: UnitTaskStatus.Rejected, compositeKey: CompositeTaskStatusKey.Rejected },
];

type TaskItem =
  | { type: "unit"; task: UnitTask }
  | { type: "composite"; task: CompositeTask };

interface KanbanColumnProps {
  status: UnitTaskStatus;
  label: string;
  unitTasks: UnitTask[];
  compositeTasks: CompositeTask[];
  repoMap: Map<string, Repository>;
}

const KanbanColumn = memo(function KanbanColumn({
  status,
  label,
  unitTasks,
  compositeTasks,
  repoMap,
}: KanbanColumnProps) {
  const totalCount = unitTasks.length + compositeTasks.length;

  // Memoize the combined and sorted task list
  const allTasks = useMemo(() => {
    const combined: TaskItem[] = [
      ...unitTasks.map((task): TaskItem => ({ type: "unit", task })),
      ...compositeTasks.map((task): TaskItem => ({ type: "composite", task })),
    ];
    return combined.sort(
      (a, b) =>
        new Date(b.task.created_at).getTime() - new Date(a.task.created_at).getTime()
    );
  }, [unitTasks, compositeTasks]);

  return (
    <div className="flex-shrink-0 w-72 bg-muted/50 rounded-lg p-3">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-medium text-sm">{label}</h3>
        <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
          {totalCount}
        </span>
      </div>
      <div className="space-y-3">
        {allTasks.map((item) =>
          item.type === "composite" ? (
            <CompositeTaskCard key={item.task.id} task={item.task} repository={repoMap.get(item.task.repository_group_id)} />
          ) : (
            <TaskCard key={item.task.id} task={item.task} repository={repoMap.get(item.task.repository_group_id)} />
          )
        )}
        {totalCount === 0 && (
          <p className="text-xs text-muted-foreground text-center py-8">
            No tasks
          </p>
        )}
      </div>
    </div>
  );
});

export function KanbanBoard({ tasksByStatus }: KanbanBoardProps) {
  const { repositories } = useRepositoriesStore();
  const { groups } = useRepositoryGroupsStore();

  // Pre-compute repository lookup maps for O(1) access
  const repoByIdMap = useMemo(() => {
    return new Map(repositories.map((r) => [r.id, r]));
  }, [repositories]);

  // Map from repository_group_id to the first Repository in the group
  const repoMap = useMemo(() => {
    const map = new Map<string, Repository>();
    for (const group of groups) {
      if (group.repository_ids.length > 0) {
        const repo = repoByIdMap.get(group.repository_ids[0]);
        if (repo) {
          map.set(group.id, repo);
        }
      }
    }
    return map;
  }, [groups, repoByIdMap]);

  return (
    <div className="flex gap-4 overflow-x-auto pb-4">
      {columns.map(({ status, label, unitKey, compositeKey }) => {
        const unitTasks = tasksByStatus[unitKey] || [];
        const compositeTasks = compositeKey ? tasksByStatus[compositeKey] || [] : [];

        return (
          <KanbanColumn
            key={status}
            status={status}
            label={label}
            unitTasks={unitTasks}
            compositeTasks={compositeTasks}
            repoMap={repoMap}
          />
        );
      })}
    </div>
  );
}
