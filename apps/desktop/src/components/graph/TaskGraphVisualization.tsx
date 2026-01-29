import { useMemo, useCallback } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  Position,
  Handle,
  type NodeProps,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import type { CompositeTaskNode } from "../../types";
import { UnitTaskStatus } from "../../types";
import { Badge } from "../ui/badge";

interface TaskNodeData extends Record<string, unknown> {
  label: string;
  prompt?: string;
  status?: UnitTaskStatus;
  hasUnitTask: boolean;
}

function TaskNode({ data }: NodeProps<Node<TaskNodeData>>) {
  const statusColors: Record<UnitTaskStatus, string> = {
    [UnitTaskStatus.InProgress]: "bg-blue-100 border-blue-400 text-blue-800",
    [UnitTaskStatus.InReview]: "bg-yellow-100 border-yellow-400 text-yellow-800",
    [UnitTaskStatus.Approved]: "bg-green-100 border-green-400 text-green-800",
    [UnitTaskStatus.PrOpen]: "bg-purple-100 border-purple-400 text-purple-800",
    [UnitTaskStatus.Done]: "bg-green-100 border-green-400 text-green-800",
    [UnitTaskStatus.Rejected]: "bg-red-100 border-red-400 text-red-800",
  };

  const statusLabels: Record<UnitTaskStatus, string> = {
    [UnitTaskStatus.InProgress]: "In Progress",
    [UnitTaskStatus.InReview]: "In Review",
    [UnitTaskStatus.Approved]: "Approved",
    [UnitTaskStatus.PrOpen]: "PR Open",
    [UnitTaskStatus.Done]: "Done",
    [UnitTaskStatus.Rejected]: "Rejected",
  };

  const borderColor = data.status
    ? statusColors[data.status].split(" ")[1]
    : data.hasUnitTask
    ? "border-gray-300"
    : "border-dashed border-gray-400";

  return (
    <div
      className={`px-4 py-3 rounded-lg border-2 bg-white shadow-sm min-w-[160px] max-w-[240px] ${borderColor}`}
    >
      <Handle type="target" position={Position.Left} className="w-3 h-3 !bg-gray-400" />
      <div className="flex flex-col gap-2">
        <div className="font-medium text-sm text-gray-900 truncate">{data.label}</div>
        {data.prompt && (
          <div className="text-xs text-gray-500 line-clamp-2">{data.prompt}</div>
        )}
        {data.status ? (
          <Badge
            variant="outline"
            className={`text-xs w-fit ${statusColors[data.status]}`}
          >
            {statusLabels[data.status]}
          </Badge>
        ) : (
          <Badge variant="outline" className="text-xs w-fit bg-gray-50 text-gray-600">
            Pending
          </Badge>
        )}
      </div>
      <Handle type="source" position={Position.Right} className="w-3 h-3 !bg-gray-400" />
    </div>
  );
}

const nodeTypes = {
  taskNode: TaskNode,
};

interface PlanTask {
  id: string;
  prompt: string;
  dependsOn?: string[];
}

interface TaskGraphVisualizationProps {
  nodes: CompositeTaskNode[];
  planTasks?: PlanTask[];
  unitTaskStatuses?: Record<string, UnitTaskStatus>;
  unitTaskTitles?: Record<string, string>;
  className?: string;
}

export function TaskGraphVisualization({
  nodes,
  planTasks,
  unitTaskStatuses = {},
  unitTaskTitles = {},
  className,
}: TaskGraphVisualizationProps) {
  const { flowNodes, flowEdges } = useMemo(() => {
    const taskData = planTasks || nodes.map((n) => ({
      id: n.id,
      prompt: "",
      dependsOn: n.depends_on,
    }));

    // Build adjacency lists
    const nodeMap = new Map<string, PlanTask>();
    const inDegree = new Map<string, number>();
    const children = new Map<string, string[]>();

    for (const task of taskData) {
      nodeMap.set(task.id, task);
      inDegree.set(task.id, task.dependsOn?.length || 0);
      children.set(task.id, []);
    }

    for (const task of taskData) {
      for (const dep of task.dependsOn || []) {
        const c = children.get(dep) || [];
        c.push(task.id);
        children.set(dep, c);
      }
    }

    // Assign layers using topological sort
    const layers: string[][] = [];
    const nodeLayer = new Map<string, number>();
    const queue: string[] = [];

    for (const [id, degree] of inDegree.entries()) {
      if (degree === 0) {
        queue.push(id);
        nodeLayer.set(id, 0);
      }
    }

    while (queue.length > 0) {
      const current = queue.shift()!;
      const layer = nodeLayer.get(current)!;

      if (!layers[layer]) {
        layers[layer] = [];
      }
      layers[layer].push(current);

      for (const child of children.get(current) || []) {
        const newDegree = (inDegree.get(child) || 0) - 1;
        inDegree.set(child, newDegree);

        if (newDegree === 0) {
          // Child layer is max of all parent layers + 1
          const parentLayers = nodeMap.get(child)?.dependsOn?.map((p) => nodeLayer.get(p) || 0) || [];
          const childLayer = Math.max(...parentLayers) + 1;
          nodeLayer.set(child, childLayer);
          queue.push(child);
        }
      }
    }

    // Position nodes
    const horizontalSpacing = 280;
    const verticalSpacing = 100;

    const flowNodes: Node<TaskNodeData>[] = [];

    for (let layerIdx = 0; layerIdx < layers.length; layerIdx++) {
      const layerNodes = layers[layerIdx];
      const layerHeight = layerNodes.length * verticalSpacing;
      const startY = -layerHeight / 2 + verticalSpacing / 2;

      for (let nodeIdx = 0; nodeIdx < layerNodes.length; nodeIdx++) {
        const taskId = layerNodes[nodeIdx];
        const task = nodeMap.get(taskId)!;
        const compositeNode = nodes.find((n) => n.id === taskId);

        // Get the title from unitTaskTitles if available, otherwise fallback to taskId
        const nodeLabel = compositeNode?.unit_task_id
          ? unitTaskTitles[compositeNode.unit_task_id] ?? taskId
          : taskId;

        flowNodes.push({
          id: taskId,
          type: "taskNode",
          position: {
            x: layerIdx * horizontalSpacing,
            y: startY + nodeIdx * verticalSpacing,
          },
          data: {
            label: nodeLabel,
            prompt: task.prompt,
            status: compositeNode?.unit_task_id
              ? unitTaskStatuses[compositeNode.unit_task_id]
              : undefined,
            hasUnitTask: !!compositeNode?.unit_task_id,
          },
        });
      }
    }

    // Create edges
    const flowEdges: Edge[] = [];
    for (const task of taskData) {
      for (const dep of task.dependsOn || []) {
        flowEdges.push({
          id: `${dep}-${task.id}`,
          source: dep,
          target: task.id,
          type: "smoothstep",
          animated: true,
          markerEnd: {
            type: MarkerType.ArrowClosed,
            width: 16,
            height: 16,
            color: "#9ca3af",
          },
          style: {
            stroke: "#9ca3af",
            strokeWidth: 2,
          },
        });
      }
    }

    return { flowNodes, flowEdges };
  }, [nodes, planTasks, unitTaskStatuses, unitTaskTitles]);

  const onInit = useCallback(() => {
    // The flow is initialized
  }, []);

  if (flowNodes.length === 0) {
    return (
      <div className={`flex items-center justify-center h-64 bg-gray-50 rounded-lg border ${className}`}>
        <p className="text-muted-foreground">No tasks to display</p>
      </div>
    );
  }

  return (
    <div className={`h-[400px] bg-gray-50 rounded-lg border ${className}`}>
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        onInit={onInit}
        fitView
        fitViewOptions={{
          padding: 0.2,
          minZoom: 0.5,
          maxZoom: 1.5,
        }}
        defaultViewport={{ x: 0, y: 0, zoom: 1 }}
        minZoom={0.25}
        maxZoom={2}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
      >
        <Background color="#e5e7eb" gap={16} />
        <Controls showInteractive={false} />
        <MiniMap
          nodeStrokeColor="#9ca3af"
          nodeColor="#ffffff"
          nodeBorderRadius={8}
          className="!bg-white !border-gray-200"
        />
      </ReactFlow>
    </div>
  );
}
