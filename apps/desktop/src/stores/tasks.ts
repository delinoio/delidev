import { create } from "zustand";
import type { UnitTask, UnitTaskStatus, CompositeTask, AIAgentType } from "../types";
import * as api from "../api";

interface TasksState {
  // State
  unitTasks: UnitTask[];
  compositeTasks: CompositeTask[];
  tasksByStatus: api.TasksByStatus | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  fetchUnitTasks: (repositoryId?: string) => Promise<void>;
  fetchCompositeTasks: (repositoryId?: string) => Promise<void>;
  fetchTasksByStatus: (workspaceId?: string) => Promise<void>;
  createUnitTask: (params: {
    repositoryGroupId: string;
    prompt: string;
    title?: string;
    branchName?: string;
    agentType?: AIAgentType;
  }) => Promise<UnitTask>;
  createCompositeTask: (params: {
    repositoryGroupId: string;
    prompt: string;
    title?: string;
    planningAgentType?: AIAgentType;
    executionAgentType?: AIAgentType;
  }) => Promise<CompositeTask>;
  updateUnitTaskStatus: (id: string, status: UnitTaskStatus) => Promise<void>;
  deleteUnitTask: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useTasksStore = create<TasksState>((set, get) => ({
  // Initial state
  unitTasks: [],
  compositeTasks: [],
  tasksByStatus: null,
  isLoading: false,
  error: null,

  // Actions
  fetchUnitTasks: async (repositoryId?: string) => {
    try {
      set({ isLoading: true, error: null });
      const unitTasks = await api.listUnitTasks(repositoryId);
      set({ unitTasks, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch unit tasks",
        isLoading: false,
      });
    }
  },

  fetchCompositeTasks: async (repositoryId?: string) => {
    try {
      set({ isLoading: true, error: null });
      const compositeTasks = await api.listCompositeTasks(repositoryId);
      set({ compositeTasks, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error
            ? error.message
            : "Failed to fetch composite tasks",
        isLoading: false,
      });
    }
  },

  fetchTasksByStatus: async (workspaceId?: string) => {
    try {
      set({ isLoading: true, error: null });
      const tasksByStatus = await api.getTasksByStatus(workspaceId);
      set({ tasksByStatus, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch tasks",
        isLoading: false,
      });
    }
  },

  createUnitTask: async (params) => {
    try {
      set({ isLoading: true, error: null });
      const task = await api.createUnitTask(params);
      set((state) => ({
        unitTasks: [task, ...state.unitTasks],
        isLoading: false,
      }));
      // Refresh tasksByStatus to update the dashboard
      get().fetchTasksByStatus();
      return task;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create task";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  createCompositeTask: async (params) => {
    try {
      set({ isLoading: true, error: null });
      const task = await api.createCompositeTask(params);
      set((state) => ({
        compositeTasks: [task, ...state.compositeTasks],
        isLoading: false,
      }));
      // Refresh tasksByStatus to update the dashboard
      get().fetchTasksByStatus();
      return task;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create task";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  updateUnitTaskStatus: async (id: string, status: UnitTaskStatus) => {
    try {
      set({ isLoading: true, error: null });
      await api.updateUnitTaskStatus(id, status);

      // Update local state
      set((state) => ({
        unitTasks: state.unitTasks.map((t) =>
          t.id === id ? { ...t, status } : t
        ),
        isLoading: false,
      }));

      // Refresh tasks by status if loaded
      if (get().tasksByStatus) {
        get().fetchTasksByStatus();
      }
    } catch (error) {
      set({
        error:
          error instanceof Error
            ? error.message
            : "Failed to update task status",
        isLoading: false,
      });
    }
  },

  deleteUnitTask: async (id: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.deleteUnitTask(id);

      // Remove from local state
      set((state) => ({
        unitTasks: state.unitTasks.filter((t) => t.id !== id),
        isLoading: false,
      }));

      // Refresh tasks by status if loaded
      if (get().tasksByStatus) {
        get().fetchTasksByStatus();
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to delete task";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  clearError: () => set({ error: null }),
}));
