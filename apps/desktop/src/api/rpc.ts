/**
 * JSON-RPC 2.0 Client for DeliDev Server Communication
 *
 * This module provides a JSON-RPC client that can communicate with the DeliDev
 * main server. In single-process mode, this client is not used - the Tauri
 * invoke commands are used directly instead.
 */

// ========== Types ==========

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: string;
  method: string;
  params: unknown;
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: "2.0";
  id: string;
  result?: T;
  error?: JsonRpcError;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export class RpcError extends Error {
  constructor(
    message: string,
    public code: number,
    public data?: unknown
  ) {
    super(message);
    this.name = "RpcError";
  }

  static fromJsonRpcError(error: JsonRpcError): RpcError {
    return new RpcError(error.message, error.code, error.data);
  }
}

// ========== Configuration ==========

export interface RpcClientConfig {
  /** Server URL (e.g., "http://localhost:8080") */
  serverUrl: string;
  /** Optional auth token */
  authToken?: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
}

// ========== Subscription Types ==========

export type SubscriptionCallback<T> = (data: T) => void;

export interface Subscription {
  unsubscribe: () => void;
}

// ========== WebSocket Types ==========

type WebSocketMessageHandler = (message: JsonRpcResponse) => void;

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

// ========== RPC Client ==========

/**
 * JSON-RPC client for communicating with the DeliDev main server
 */
export class JsonRpcClient {
  private serverUrl: string;
  private authToken: string | null = null;
  private timeout: number;
  private ws: WebSocket | null = null;
  private wsConnecting: boolean = false;
  private wsMessageHandlers: Map<string, WebSocketMessageHandler> = new Map();
  private pendingRequests: Map<string, PendingRequest> = new Map();
  private subscriptions: Map<string, Set<SubscriptionCallback<unknown>>> =
    new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(config: RpcClientConfig) {
    this.serverUrl = config.serverUrl;
    this.authToken = config.authToken ?? null;
    this.timeout = config.timeout ?? 30000;
  }

  /**
   * Sets the authentication token
   */
  setAuthToken(token: string | null): void {
    this.authToken = token;
  }

  /**
   * Gets the current authentication token
   */
  getAuthToken(): string | null {
    return this.authToken;
  }

  /**
   * Makes a JSON-RPC call to the server
   */
  async call<T>(method: string, params: unknown = {}): Promise<T> {
    const id = crypto.randomUUID();
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (this.authToken) {
      headers["Authorization"] = `Bearer ${this.authToken}`;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.serverUrl}/rpc`, {
        method: "POST",
        headers,
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new RpcError(
          `HTTP error: ${response.status} ${response.statusText}`,
          -32600
        );
      }

      const jsonResponse: JsonRpcResponse<T> = await response.json();

      if (jsonResponse.error) {
        throw RpcError.fromJsonRpcError(jsonResponse.error);
      }

      if (jsonResponse.result === undefined) {
        throw new RpcError("Response has neither result nor error", -32603);
      }

      return jsonResponse.result;
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof RpcError) {
        throw error;
      }

      if (error instanceof Error) {
        if (error.name === "AbortError") {
          throw new RpcError("Request timeout", -32000);
        }
        throw new RpcError(error.message, -32603);
      }

      throw new RpcError("Unknown error", -32603);
    }
  }

  // ========== WebSocket Connection ==========

  /**
   * Connects to the WebSocket endpoint for real-time subscriptions
   */
  async connect(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return;
    }

    if (this.wsConnecting) {
      // Wait for existing connection attempt
      return new Promise((resolve, reject) => {
        const checkConnection = setInterval(() => {
          if (this.ws?.readyState === WebSocket.OPEN) {
            clearInterval(checkConnection);
            resolve();
          } else if (!this.wsConnecting) {
            clearInterval(checkConnection);
            reject(new Error("WebSocket connection failed"));
          }
        }, 100);
      });
    }

    this.wsConnecting = true;

    return new Promise((resolve, reject) => {
      const wsUrl = this.serverUrl.replace(/^http/, "ws") + "/ws";
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        this.wsConnecting = false;
        this.reconnectAttempts = 0;
        console.log("[RPC] WebSocket connected");
        resolve();
      };

      this.ws.onerror = (error) => {
        this.wsConnecting = false;
        console.error("[RPC] WebSocket error:", error);
        reject(error);
      };

      this.ws.onclose = () => {
        this.wsConnecting = false;
        console.log("[RPC] WebSocket closed");
        this.handleReconnect();
      };

      this.ws.onmessage = (event) => {
        this.handleWebSocketMessage(event.data);
      };
    });
  }

  /**
   * Disconnects from the WebSocket
   */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscriptions.clear();
    this.pendingRequests.forEach(({ reject, timeout }) => {
      clearTimeout(timeout);
      reject(new Error("Connection closed"));
    });
    this.pendingRequests.clear();
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error("[RPC] Max reconnection attempts reached");
      return;
    }

    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
    this.reconnectAttempts++;

    console.log(`[RPC] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    setTimeout(() => {
      if (!this.ws || this.ws.readyState === WebSocket.CLOSED) {
        this.connect().catch((error) => {
          console.error("[RPC] Reconnection failed:", error);
        });
      }
    }, delay);
  }

  private handleWebSocketMessage(data: string): void {
    try {
      const response: JsonRpcResponse = JSON.parse(data);

      // Check if this is a response to a pending request
      if (response.id && this.pendingRequests.has(response.id)) {
        const pending = this.pendingRequests.get(response.id)!;
        this.pendingRequests.delete(response.id);
        clearTimeout(pending.timeout);

        if (response.error) {
          pending.reject(RpcError.fromJsonRpcError(response.error));
        } else {
          pending.resolve(response.result);
        }
        return;
      }

      // Handle notification/subscription messages
      const handlers = this.wsMessageHandlers.get(response.id);
      if (handlers) {
        handlers(response);
      }
    } catch (error) {
      console.error("[RPC] Failed to parse WebSocket message:", error);
    }
  }

  /**
   * Makes a JSON-RPC call over WebSocket
   */
  async callWs<T>(method: string, params: unknown = {}): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      await this.connect();
    }

    const id = crypto.randomUUID();
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };

    return new Promise<T>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new RpcError("Request timeout", -32000));
      }, this.timeout);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout,
      });

      this.ws!.send(JSON.stringify(request));
    });
  }

  // ========== Subscriptions ==========

  /**
   * Subscribes to execution logs for a task
   */
  subscribeExecutionLogs<T>(
    taskId: string,
    callback: SubscriptionCallback<T>
  ): Subscription {
    const subscriptionKey = `executionLogs:${taskId}`;

    if (!this.subscriptions.has(subscriptionKey)) {
      this.subscriptions.set(subscriptionKey, new Set());

      // Send subscription request
      this.callWs("subscribeExecutionLogs", { taskId }).catch((error) => {
        console.error("[RPC] Failed to subscribe to execution logs:", error);
      });
    }

    const callbacks = this.subscriptions.get(subscriptionKey)!;
    callbacks.add(callback as SubscriptionCallback<unknown>);

    // Register message handler
    this.wsMessageHandlers.set(subscriptionKey, (message) => {
      if (message.result) {
        callbacks.forEach((cb) => cb(message.result));
      }
    });

    return {
      unsubscribe: () => {
        callbacks.delete(callback as SubscriptionCallback<unknown>);
        if (callbacks.size === 0) {
          this.subscriptions.delete(subscriptionKey);
          this.wsMessageHandlers.delete(subscriptionKey);

          // Send unsubscription request
          this.callWs("unsubscribeExecutionLogs", { taskId }).catch(() => {
            // Ignore unsubscribe errors
          });
        }
      },
    };
  }

  /**
   * Subscribes to task status changes
   */
  subscribeTaskStatus<T>(callback: SubscriptionCallback<T>): Subscription {
    const subscriptionKey = "taskStatus";

    if (!this.subscriptions.has(subscriptionKey)) {
      this.subscriptions.set(subscriptionKey, new Set());
    }

    const callbacks = this.subscriptions.get(subscriptionKey)!;
    callbacks.add(callback as SubscriptionCallback<unknown>);

    this.wsMessageHandlers.set(subscriptionKey, (message) => {
      if (message.result) {
        callbacks.forEach((cb) => cb(message.result));
      }
    });

    return {
      unsubscribe: () => {
        callbacks.delete(callback as SubscriptionCallback<unknown>);
        if (callbacks.size === 0) {
          this.subscriptions.delete(subscriptionKey);
          this.wsMessageHandlers.delete(subscriptionKey);
        }
      },
    };
  }
}

// ========== Method Names ==========

/**
 * RPC method names (matching Rust rpc_protocol crate)
 */
export const RpcMethods = {
  // Task methods
  CREATE_UNIT_TASK: "createUnitTask",
  GET_UNIT_TASK: "getUnitTask",
  LIST_UNIT_TASKS: "listUnitTasks",
  UPDATE_UNIT_TASK_STATUS: "updateUnitTaskStatus",
  DELETE_UNIT_TASK: "deleteUnitTask",
  START_TASK_EXECUTION: "startTaskExecution",
  STOP_TASK_EXECUTION: "stopTaskExecution",

  // Composite task methods
  CREATE_COMPOSITE_TASK: "createCompositeTask",
  GET_COMPOSITE_TASK: "getCompositeTask",
  LIST_COMPOSITE_TASKS: "listCompositeTasks",
  APPROVE_COMPOSITE_PLAN: "approveCompositePlan",
  REJECT_COMPOSITE_PLAN: "rejectCompositePlan",

  // Repository methods
  ADD_REPOSITORY: "addRepository",
  GET_REPOSITORY: "getRepository",
  LIST_REPOSITORIES: "listRepositories",
  REMOVE_REPOSITORY: "removeRepository",

  // Repository group methods
  CREATE_REPOSITORY_GROUP: "createRepositoryGroup",
  GET_REPOSITORY_GROUP: "getRepositoryGroup",
  LIST_REPOSITORY_GROUPS: "listRepositoryGroups",

  // Workspace methods
  CREATE_WORKSPACE: "createWorkspace",
  LIST_WORKSPACES: "listWorkspaces",

  // Execution log methods
  GET_EXECUTION_LOGS: "getExecutionLogs",
  SUBSCRIBE_EXECUTION_LOGS: "subscribeExecutionLogs",
  UNSUBSCRIBE_EXECUTION_LOGS: "unsubscribeExecutionLogs",

  // Secret methods
  SEND_SECRETS: "sendSecrets",
} as const;

// ========== Request/Response Types ==========

export interface CreateUnitTaskRequest {
  repositoryGroupId: string;
  title: string;
  prompt: string;
  branchName?: string;
  agentType?: string;
  model?: string;
}

export interface CreateUnitTaskResponse {
  task: import("../types").UnitTask;
}

export interface GetUnitTaskRequest {
  id: string;
}

export interface GetUnitTaskResponse {
  task: import("../types").UnitTask | null;
}

export interface ListUnitTasksRequest {
  repositoryGroupId?: string;
  status?: import("../types").UnitTaskStatus;
  limit?: number;
  offset?: number;
}

export interface ListUnitTasksResponse {
  tasks: import("../types").UnitTask[];
}

export interface UpdateUnitTaskStatusRequest {
  id: string;
  status: import("../types").UnitTaskStatus;
}

export interface StartTaskExecutionRequest {
  taskId: string;
}

export interface StartTaskExecutionResponse {
  started: boolean;
  sessionId?: string;
}

export interface StopTaskExecutionRequest {
  taskId: string;
}

export interface CreateCompositeTaskRequest {
  repositoryGroupId: string;
  title: string;
  prompt: string;
  executionAgentType?: string;
}

export interface CreateCompositeTaskResponse {
  task: import("../types").CompositeTask;
}

export interface GetCompositeTaskRequest {
  id: string;
}

export interface GetCompositeTaskResponse {
  task: import("../types").CompositeTask | null;
}

export interface ApproveCompositePlanRequest {
  id: string;
}

export interface RejectCompositePlanRequest {
  id: string;
  reason?: string;
}

export interface AddRepositoryRequest {
  name: string;
  remoteUrl: string;
  localPath: string;
  defaultBranch?: string;
  vcsProviderType?: string;
}

export interface AddRepositoryResponse {
  repository: import("../types").Repository;
}

export interface ListRepositoriesRequest {
  workspaceId?: string;
}

export interface ListRepositoriesResponse {
  repositories: import("../types").Repository[];
}

export interface GetExecutionLogsRequest {
  sessionId: string;
  offset?: number;
  limit?: number;
}

export interface NormalizedMessage {
  type: "start" | "text" | "tool_use" | "tool_result" | "user_question" | "complete" | "error";
  timestamp: string;
  content?: string;
  toolName?: string;
  input?: unknown;
  output?: unknown;
  success?: boolean;
  question?: string;
  options?: string[];
  summary?: string;
  message?: string;
}

export interface GetExecutionLogsResponse {
  logs: NormalizedMessage[];
}

export interface SendSecretsRequest {
  taskId: string;
  secrets: Record<string, string>;
}

export interface SendSecretsResponse {
  accepted: boolean;
}

export interface SuccessResponse {
  success: boolean;
}

export interface ExecutionLogNotification {
  taskId: string;
  sessionId: string;
  message: NormalizedMessage;
}

// ========== Singleton Instance ==========

let rpcClientInstance: JsonRpcClient | null = null;

/**
 * Gets or creates the RPC client instance
 */
export function getRpcClient(config?: RpcClientConfig): JsonRpcClient {
  if (!rpcClientInstance && config) {
    rpcClientInstance = new JsonRpcClient(config);
  }

  if (!rpcClientInstance) {
    throw new Error("RPC client not initialized. Call getRpcClient with config first.");
  }

  return rpcClientInstance;
}

/**
 * Initializes the RPC client with the given configuration
 */
export function initRpcClient(config: RpcClientConfig): JsonRpcClient {
  rpcClientInstance = new JsonRpcClient(config);
  return rpcClientInstance;
}

/**
 * Resets the RPC client instance (useful for testing)
 */
export function resetRpcClient(): void {
  if (rpcClientInstance) {
    rpcClientInstance.disconnect();
    rpcClientInstance = null;
  }
}
