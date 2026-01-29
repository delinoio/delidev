import { create } from "zustand";
import type { Workspace } from "../types";
import * as api from "../api";

interface WorkspacesState {
  // State
  workspaces: Workspace[];
  selectedWorkspaceId: string | null;
  isLoading: boolean;
  hasFetched: boolean;
  error: string | null;

  // Computed
  selectedWorkspace: () => Workspace | undefined;

  // Actions
  fetchWorkspaces: () => Promise<void>;
  createWorkspace: (name: string, description?: string) => Promise<Workspace>;
  updateWorkspace: (id: string, name: string, description?: string) => Promise<Workspace>;
  deleteWorkspace: (id: string) => Promise<void>;
  selectWorkspace: (id: string | null) => void;
  getDefaultWorkspace: () => Promise<Workspace>;
  clearError: () => void;
}

export const useWorkspacesStore = create<WorkspacesState>((set, get) => ({
  // Initial state
  workspaces: [],
  selectedWorkspaceId: null,
  isLoading: false,
  hasFetched: false,
  error: null,

  // Computed
  selectedWorkspace: () => {
    const { workspaces, selectedWorkspaceId } = get();
    return workspaces.find((w) => w.id === selectedWorkspaceId);
  },

  // Actions
  fetchWorkspaces: async () => {
    try {
      set({ isLoading: true, error: null });
      const workspaces = await api.listWorkspaces();
      set({ workspaces, isLoading: false, hasFetched: true });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch workspaces",
        isLoading: false,
        hasFetched: true,
      });
    }
  },

  createWorkspace: async (name: string, description?: string) => {
    try {
      set({ isLoading: true, error: null });
      const workspace = await api.createWorkspace(name, description);
      set((state) => ({
        workspaces: [...state.workspaces, workspace],
        isLoading: false,
      }));
      return workspace;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create workspace";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  updateWorkspace: async (id: string, name: string, description?: string) => {
    try {
      set({ isLoading: true, error: null });
      const workspace = await api.updateWorkspace(id, name, description);
      set((state) => ({
        workspaces: state.workspaces.map((w) =>
          w.id === id ? workspace : w
        ),
        isLoading: false,
      }));
      return workspace;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to update workspace";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  deleteWorkspace: async (id: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.deleteWorkspace(id);
      set((state) => ({
        workspaces: state.workspaces.filter((w) => w.id !== id),
        selectedWorkspaceId:
          state.selectedWorkspaceId === id ? null : state.selectedWorkspaceId,
        isLoading: false,
      }));
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to delete workspace",
        isLoading: false,
      });
    }
  },

  selectWorkspace: (id: string | null) => {
    set({ selectedWorkspaceId: id });
  },

  getDefaultWorkspace: async () => {
    try {
      set({ isLoading: true, error: null });
      const workspace = await api.getDefaultWorkspace();

      // Add to list if not already there
      set((state) => {
        const exists = state.workspaces.some((w) => w.id === workspace.id);
        return {
          workspaces: exists ? state.workspaces : [...state.workspaces, workspace],
          selectedWorkspaceId: state.selectedWorkspaceId ?? workspace.id,
          isLoading: false,
        };
      });

      return workspace;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to get default workspace";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  clearError: () => set({ error: null }),
}));
