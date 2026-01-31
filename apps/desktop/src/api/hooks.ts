/**
 * React Query Hooks for DeliDev API
 *
 * This module provides React Query hooks for communicating with the DeliDev
 * server. In single-process mode, these hooks call Tauri invoke commands
 * directly. In client mode, they use the JSON-RPC client.
 *
 * Note: react-query is optional. This module provides a unified API that
 * can work with or without react-query installed.
 */

import { useCallback, useEffect, useState, useMemo } from "react";
import type { UnitTask, CompositeTask, Repository, Workspace, RepositoryGroup, UnitTaskStatus, CompositeTaskStatus } from "../types";
import * as api from "./index";
import {
  getRpcClient,
  RpcMethods,
  type CreateUnitTaskRequest,
  type CreateUnitTaskResponse,
  type GetUnitTaskResponse,
  type ListUnitTasksRequest,
  type ListUnitTasksResponse,
  type StartTaskExecutionResponse,
  type CreateCompositeTaskRequest,
  type CreateCompositeTaskResponse,
  type GetCompositeTaskResponse,
  type AddRepositoryRequest,
  type AddRepositoryByUrlRequest,
  type AddRepositoryResponse,
  type ListRepositoriesResponse,
  type GetExecutionLogsResponse,
  type SendSecretsRequest,
  type SendSecretsResponse,
  type ExecutionLogNotification,
  type NormalizedMessage,
} from "./rpc";

// ========== Mode Configuration ==========

export type ClientMode = "single_process" | "remote";

interface ClientModeConfig {
  mode: ClientMode;
  serverUrl?: string;
  authToken?: string;
}

let clientModeConfig: ClientModeConfig = {
  mode: "single_process",
};

/**
 * Sets the client mode configuration
 */
export function setClientMode(config: ClientModeConfig): void {
  clientModeConfig = config;

  if (config.mode === "remote" && config.serverUrl) {
    const { initRpcClient } = require("./rpc");
    initRpcClient({
      serverUrl: config.serverUrl,
      authToken: config.authToken,
    });
  }
}

/**
 * Gets the current client mode
 */
export function getClientMode(): ClientMode {
  return clientModeConfig.mode;
}

/**
 * Checks if we're in single process mode
 */
export function isSingleProcessMode(): boolean {
  return clientModeConfig.mode === "single_process";
}

// ========== Generic Hook Utilities ==========

interface QueryResult<T> {
  data: T | undefined;
  isLoading: boolean;
  isError: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

interface MutationResult<TData, TVariables> {
  mutate: (variables: TVariables) => Promise<TData>;
  mutateAsync: (variables: TVariables) => Promise<TData>;
  isLoading: boolean;
  isError: boolean;
  error: Error | null;
  data: TData | undefined;
  reset: () => void;
}

/**
 * A simple query hook that works without react-query
 */
function useSimpleQuery<T>(
  queryKey: string[],
  queryFn: () => Promise<T>,
  options?: { enabled?: boolean }
): QueryResult<T> {
  const [data, setData] = useState<T | undefined>(undefined);
  const [isLoading, setIsLoading] = useState(true);
  const [isError, setIsError] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const enabled = options?.enabled ?? true;

  const refetch = useCallback(async () => {
    if (!enabled) return;

    setIsLoading(true);
    setIsError(false);
    setError(null);

    try {
      const result = await queryFn();
      setData(result);
    } catch (e) {
      setIsError(true);
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setIsLoading(false);
    }
  }, [queryFn, enabled]);

  useEffect(() => {
    refetch();
  }, [queryKey.join(","), enabled]);

  return { data, isLoading, isError, error, refetch };
}

/**
 * A simple mutation hook that works without react-query
 */
function useSimpleMutation<TData, TVariables>(
  mutationFn: (variables: TVariables) => Promise<TData>
): MutationResult<TData, TVariables> {
  const [data, setData] = useState<TData | undefined>(undefined);
  const [isLoading, setIsLoading] = useState(false);
  const [isError, setIsError] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const mutateAsync = useCallback(
    async (variables: TVariables): Promise<TData> => {
      setIsLoading(true);
      setIsError(false);
      setError(null);

      try {
        const result = await mutationFn(variables);
        setData(result);
        return result;
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        setIsError(true);
        setError(err);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [mutationFn]
  );

  const mutate = useCallback(
    (variables: TVariables) => {
      mutateAsync(variables).catch(() => {
        // Error is already captured in state
      });
    },
    [mutateAsync]
  ) as (variables: TVariables) => Promise<TData>;

  const reset = useCallback(() => {
    setData(undefined);
    setIsLoading(false);
    setIsError(false);
    setError(null);
  }, []);

  return { mutate, mutateAsync, isLoading, isError, error, data, reset };
}

// ========== Task Hooks ==========

/**
 * Fetches a list of unit tasks
 */
export function useUnitTasks(filter?: { repositoryGroupId?: string; status?: UnitTaskStatus }) {
  const queryFn = useCallback(async (): Promise<UnitTask[]> => {
    if (isSingleProcessMode()) {
      return api.listUnitTasks(filter?.repositoryGroupId);
    } else {
      const client = getRpcClient();
      const params: ListUnitTasksRequest = {};
      if (filter?.repositoryGroupId) {
        params.repositoryGroupId = filter.repositoryGroupId;
      }
      if (filter?.status) {
        params.status = filter.status;
      }
      const response = await client.call<ListUnitTasksResponse>(
        RpcMethods.LIST_UNIT_TASKS,
        params
      );
      return response.tasks;
    }
  }, [filter?.repositoryGroupId, filter?.status]);

  return useSimpleQuery(
    ["unitTasks", filter?.repositoryGroupId ?? "", filter?.status ?? ""],
    queryFn
  );
}

/**
 * Fetches a single unit task
 */
export function useUnitTask(id: string) {
  const queryFn = useCallback(async (): Promise<UnitTask | null> => {
    if (isSingleProcessMode()) {
      return api.getUnitTask(id);
    } else {
      const client = getRpcClient();
      const response = await client.call<GetUnitTaskResponse>(
        RpcMethods.GET_UNIT_TASK,
        { id }
      );
      return response.task;
    }
  }, [id]);

  return useSimpleQuery(["unitTask", id], queryFn, { enabled: !!id });
}

/**
 * Creates a new unit task
 */
export function useCreateUnitTask() {
  type CreateTaskParams = {
    repositoryGroupId: string;
    prompt: string;
    title?: string;
    branchName?: string;
  };

  const mutationFn = useCallback(async (params: CreateTaskParams): Promise<UnitTask> => {
    if (isSingleProcessMode()) {
      return api.createUnitTask(params);
    } else {
      const client = getRpcClient();
      const request: CreateUnitTaskRequest = {
        repositoryGroupId: params.repositoryGroupId,
        title: params.title ?? "",
        prompt: params.prompt,
        branchName: params.branchName,
      };
      const response = await client.call<CreateUnitTaskResponse>(
        RpcMethods.CREATE_UNIT_TASK,
        request
      );
      return response.task;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Updates a unit task's status
 */
export function useUpdateUnitTaskStatus() {
  type UpdateParams = { id: string; status: UnitTaskStatus };

  const mutationFn = useCallback(async (params: UpdateParams): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.updateUnitTaskStatus(params.id, params.status);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.UPDATE_UNIT_TASK_STATUS, params);
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Deletes a unit task
 */
export function useDeleteUnitTask() {
  const mutationFn = useCallback(async (id: string): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.deleteUnitTask(id);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.DELETE_UNIT_TASK, { id });
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Starts task execution
 */
export function useStartTaskExecution() {
  type StartResult = { started: boolean; sessionId?: string };

  const mutationFn = useCallback(async (taskId: string): Promise<StartResult> => {
    if (isSingleProcessMode()) {
      await api.startTaskExecution(taskId);
      return { started: true };
    } else {
      const client = getRpcClient();
      return client.call<StartTaskExecutionResponse>(
        RpcMethods.START_TASK_EXECUTION,
        { taskId }
      );
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Stops task execution
 */
export function useStopTaskExecution() {
  const mutationFn = useCallback(async (taskId: string): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.stopTaskExecution(taskId);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.STOP_TASK_EXECUTION, { taskId });
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

// ========== Composite Task Hooks ==========

/**
 * Fetches a list of composite tasks
 */
export function useCompositeTasks(repositoryId?: string) {
  const queryFn = useCallback(async (): Promise<CompositeTask[]> => {
    if (isSingleProcessMode()) {
      return api.listCompositeTasks(repositoryId);
    } else {
      const client = getRpcClient();
      const response = await client.call<{ tasks: CompositeTask[] }>(
        RpcMethods.LIST_COMPOSITE_TASKS,
        { repositoryId }
      );
      return response.tasks;
    }
  }, [repositoryId]);

  return useSimpleQuery(["compositeTasks", repositoryId ?? ""], queryFn);
}

/**
 * Fetches a single composite task
 */
export function useCompositeTask(id: string) {
  const queryFn = useCallback(async (): Promise<CompositeTask | null> => {
    if (isSingleProcessMode()) {
      return api.getCompositeTask(id);
    } else {
      const client = getRpcClient();
      const response = await client.call<GetCompositeTaskResponse>(
        RpcMethods.GET_COMPOSITE_TASK,
        { id }
      );
      return response.task;
    }
  }, [id]);

  return useSimpleQuery(["compositeTask", id], queryFn, { enabled: !!id });
}

/**
 * Creates a new composite task
 */
export function useCreateCompositeTask() {
  type CreateParams = {
    repositoryGroupId: string;
    prompt: string;
    title?: string;
  };

  const mutationFn = useCallback(async (params: CreateParams): Promise<CompositeTask> => {
    if (isSingleProcessMode()) {
      return api.createCompositeTask(params);
    } else {
      const client = getRpcClient();
      const request: CreateCompositeTaskRequest = {
        repositoryGroupId: params.repositoryGroupId,
        title: params.title ?? "",
        prompt: params.prompt,
      };
      const response = await client.call<CreateCompositeTaskResponse>(
        RpcMethods.CREATE_COMPOSITE_TASK,
        request
      );
      return response.task;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Approves a composite task plan
 */
export function useApproveCompositePlan() {
  const mutationFn = useCallback(async (id: string): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.approveCompositeTaskPlan(id);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.APPROVE_COMPOSITE_PLAN, { id });
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Rejects a composite task plan
 */
export function useRejectCompositePlan() {
  type RejectParams = { id: string; reason?: string };

  const mutationFn = useCallback(async (params: RejectParams): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.rejectCompositeTaskPlan(params.id);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.REJECT_COMPOSITE_PLAN, params);
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

// ========== Repository Hooks ==========

/**
 * Fetches a list of repositories
 */
export function useRepositories() {
  const queryFn = useCallback(async (): Promise<Repository[]> => {
    if (isSingleProcessMode()) {
      return api.listRepositories();
    } else {
      const client = getRpcClient();
      const response = await client.call<ListRepositoriesResponse>(
        RpcMethods.LIST_REPOSITORIES,
        {}
      );
      return response.repositories;
    }
  }, []);

  return useSimpleQuery(["repositories"], queryFn);
}

/**
 * Fetches a single repository
 */
export function useRepository(id: string) {
  const queryFn = useCallback(async (): Promise<Repository | null> => {
    if (isSingleProcessMode()) {
      return api.getRepository(id);
    } else {
      const client = getRpcClient();
      const response = await client.call<{ repository: Repository | null }>(
        RpcMethods.GET_REPOSITORY,
        { id }
      );
      return response.repository;
    }
  }, [id]);

  return useSimpleQuery(["repository", id], queryFn, { enabled: !!id });
}

/**
 * Adds a new repository
 */
export function useAddRepository() {
  const mutationFn = useCallback(async (path: string): Promise<Repository> => {
    if (isSingleProcessMode()) {
      return api.addRepository(path);
    } else {
      const client = getRpcClient();
      const request: AddRepositoryRequest = {
        name: path.split("/").pop() ?? path,
        remoteUrl: "",
        localPath: path,
      };
      const response = await client.call<AddRepositoryResponse>(
        RpcMethods.ADD_REPOSITORY,
        request
      );
      return response.repository;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Adds a repository by URL (for server mode)
 */
export function useAddRepositoryByUrl() {
  type AddParams = { remoteUrl: string; defaultBranch?: string };

  const mutationFn = useCallback(async (params: AddParams): Promise<Repository> => {
    if (isSingleProcessMode()) {
      return api.addRepositoryByUrl(params.remoteUrl, params.defaultBranch);
    } else {
      const client = getRpcClient();
      const request: AddRepositoryByUrlRequest = {
        remoteUrl: params.remoteUrl,
        defaultBranch: params.defaultBranch,
      };
      const response = await client.call<AddRepositoryResponse>(
        RpcMethods.ADD_REPOSITORY_BY_URL,
        request
      );
      return response.repository;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

/**
 * Removes a repository
 */
export function useRemoveRepository() {
  const mutationFn = useCallback(async (id: string): Promise<void> => {
    if (isSingleProcessMode()) {
      return api.removeRepository(id);
    } else {
      const client = getRpcClient();
      await client.call(RpcMethods.REMOVE_REPOSITORY, { id });
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

// ========== Workspace Hooks ==========

/**
 * Fetches a list of workspaces
 */
export function useWorkspaces() {
  const queryFn = useCallback(async (): Promise<Workspace[]> => {
    if (isSingleProcessMode()) {
      return api.listWorkspaces();
    } else {
      const client = getRpcClient();
      const response = await client.call<{ workspaces: Workspace[] }>(
        RpcMethods.LIST_WORKSPACES,
        {}
      );
      return response.workspaces;
    }
  }, []);

  return useSimpleQuery(["workspaces"], queryFn);
}

/**
 * Creates a new workspace
 */
export function useCreateWorkspace() {
  type CreateParams = { name: string; description?: string };

  const mutationFn = useCallback(async (params: CreateParams): Promise<Workspace> => {
    if (isSingleProcessMode()) {
      return api.createWorkspace(params.name, params.description);
    } else {
      const client = getRpcClient();
      const response = await client.call<{ workspace: Workspace }>(
        RpcMethods.CREATE_WORKSPACE,
        params
      );
      return response.workspace;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

// ========== Repository Group Hooks ==========

/**
 * Fetches a list of repository groups
 */
export function useRepositoryGroups(workspaceId?: string) {
  const queryFn = useCallback(async (): Promise<RepositoryGroup[]> => {
    if (isSingleProcessMode()) {
      return api.listRepositoryGroups(workspaceId);
    } else {
      const client = getRpcClient();
      const response = await client.call<{ groups: RepositoryGroup[] }>(
        RpcMethods.LIST_REPOSITORY_GROUPS,
        { workspaceId }
      );
      return response.groups;
    }
  }, [workspaceId]);

  return useSimpleQuery(["repositoryGroups", workspaceId ?? ""], queryFn);
}

/**
 * Fetches a single repository group
 */
export function useRepositoryGroup(id: string) {
  const queryFn = useCallback(async (): Promise<RepositoryGroup | null> => {
    if (isSingleProcessMode()) {
      return api.getRepositoryGroup(id);
    } else {
      const client = getRpcClient();
      const response = await client.call<{ group: RepositoryGroup | null }>(
        RpcMethods.GET_REPOSITORY_GROUP,
        { id }
      );
      return response.group;
    }
  }, [id]);

  return useSimpleQuery(["repositoryGroup", id], queryFn, { enabled: !!id });
}

// ========== Execution Log Hooks ==========

/**
 * Fetches execution logs for a session
 */
export function useExecutionLogs(sessionId: string) {
  const queryFn = useCallback(async (): Promise<NormalizedMessage[]> => {
    if (isSingleProcessMode()) {
      const logs = await api.getExecutionLogs(sessionId);
      // Convert to normalized message format
      return logs.map((log) => ({
        type: "text" as const,
        timestamp: log.timestamp,
        content: log.message,
      }));
    } else {
      const client = getRpcClient();
      const response = await client.call<GetExecutionLogsResponse>(
        RpcMethods.GET_EXECUTION_LOGS,
        { sessionId }
      );
      return response.logs;
    }
  }, [sessionId]);

  return useSimpleQuery(["executionLogs", sessionId], queryFn, { enabled: !!sessionId });
}

/**
 * Subscribes to real-time execution logs
 */
export function useExecutionLogsSubscription(taskId: string) {
  const [logs, setLogs] = useState<NormalizedMessage[]>([]);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    if (!taskId) return;

    if (isSingleProcessMode()) {
      // In single process mode, use Tauri events
      let unsubscribe: (() => void) | undefined;

      api.onClaudeStream((event) => {
        if (event.task_id === taskId) {
          const message: NormalizedMessage = {
            type: "text",
            timestamp: event.timestamp,
            content: JSON.stringify(event.message),
          };
          setLogs((prev) => [...prev, message]);
        }
      }).then((unsub) => {
        unsubscribe = unsub;
        setIsConnected(true);
      });

      return () => {
        if (unsubscribe) {
          unsubscribe();
        }
        setIsConnected(false);
      };
    } else {
      // In remote mode, use WebSocket subscription
      const client = getRpcClient();
      const subscription = client.subscribeExecutionLogs<ExecutionLogNotification>(
        taskId,
        (notification) => {
          setLogs((prev) => [...prev, notification.message]);
        }
      );

      setIsConnected(true);

      return () => {
        subscription.unsubscribe();
        setIsConnected(false);
      };
    }
  }, [taskId]);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  return { logs, isConnected, clearLogs };
}

// ========== Secrets Hook ==========

/**
 * Sends secrets to the server for task execution
 */
export function useSendSecrets() {
  type SendParams = { taskId: string; secrets: Record<string, string> };

  const mutationFn = useCallback(async (params: SendParams): Promise<boolean> => {
    if (isSingleProcessMode()) {
      // In single process mode, secrets are retrieved from local keychain
      // No need to send them explicitly
      return true;
    } else {
      const client = getRpcClient();
      const request: SendSecretsRequest = {
        taskId: params.taskId,
        secrets: params.secrets,
      };
      const response = await client.call<SendSecretsResponse>(
        RpcMethods.SEND_SECRETS,
        request
      );
      return response.accepted;
    }
  }, []);

  return useSimpleMutation(mutationFn);
}

// ========== Combined Task Execution Hook ==========

/**
 * A combined hook for starting task execution with secrets
 */
export function useStartTaskWithSecrets() {
  const startExecution = useStartTaskExecution();
  const sendSecrets = useSendSecrets();

  type StartWithSecretsParams = {
    taskId: string;
    secrets?: Record<string, string>;
  };

  const mutationFn = useCallback(
    async (params: StartWithSecretsParams): Promise<{ started: boolean; sessionId?: string }> => {
      // Send secrets first (if not in single process mode)
      if (!isSingleProcessMode() && params.secrets) {
        const accepted = await sendSecrets.mutateAsync({
          taskId: params.taskId,
          secrets: params.secrets,
        });

        if (!accepted) {
          throw new Error("Secrets not accepted by server");
        }
      }

      // Then start execution
      return startExecution.mutateAsync(params.taskId);
    },
    [sendSecrets, startExecution]
  );

  return useSimpleMutation(mutationFn);
}

// ========== Tasks by Status Hook ==========

/**
 * Fetches all tasks organized by status (for Kanban board)
 */
export function useTasksByStatus(workspaceId?: string) {
  const queryFn = useCallback(async () => {
    if (isSingleProcessMode()) {
      return api.getTasksByStatus(workspaceId);
    } else {
      // In remote mode, we need to fetch unit tasks and composite tasks separately
      // and organize them by status
      const client = getRpcClient();

      const [unitTasksResponse, compositeTasksResponse] = await Promise.all([
        client.call<ListUnitTasksResponse>(RpcMethods.LIST_UNIT_TASKS, {}),
        client.call<{ tasks: CompositeTask[] }>(RpcMethods.LIST_COMPOSITE_TASKS, {}),
      ]);

      // Organize by status (this mirrors the backend logic)
      const result: api.TasksByStatus = {
        in_progress: [],
        in_review: [],
        approved: [],
        pr_open: [],
        done: [],
        rejected: [],
        composite_in_progress: [],
        composite_in_review: [],
        composite_done: [],
        composite_rejected: [],
      };

      for (const task of unitTasksResponse.tasks) {
        const key = task.status as keyof typeof result;
        if (result[key] && Array.isArray(result[key])) {
          (result[key] as UnitTask[]).push(task);
        }
      }

      for (const task of compositeTasksResponse.tasks) {
        switch (task.status) {
          case "planning":
          case "in_progress":
            result.composite_in_progress.push(task);
            break;
          case "pending_approval":
            result.composite_in_review.push(task);
            break;
          case "done":
            result.composite_done.push(task);
            break;
          case "rejected":
            result.composite_rejected.push(task);
            break;
        }
      }

      return result;
    }
  }, [workspaceId]);

  return useSimpleQuery(["tasksByStatus", workspaceId ?? ""], queryFn);
}
