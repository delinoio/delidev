import { create } from "zustand";
import type { RepositoryGroup } from "../types";
import * as api from "../api";

interface RepositoryGroupsState {
  // State
  groups: RepositoryGroup[];
  selectedGroupId: string | null;
  isLoading: boolean;
  hasFetched: boolean;
  error: string | null;

  // Computed
  selectedGroup: () => RepositoryGroup | undefined;

  // Actions
  fetchGroups: (workspaceId?: string) => Promise<void>;
  createGroup: (workspaceId: string, name?: string, repositoryIds?: string[]) => Promise<RepositoryGroup>;
  updateGroup: (id: string, name?: string) => Promise<RepositoryGroup>;
  deleteGroup: (id: string) => Promise<void>;
  selectGroup: (id: string | null) => void;
  addRepositoryToGroup: (groupId: string, repositoryId: string) => Promise<void>;
  removeRepositoryFromGroup: (groupId: string, repositoryId: string) => Promise<void>;
  getOrCreateSingleRepoGroup: (workspaceId: string, repositoryId: string) => Promise<string>;
  clearError: () => void;
}

export const useRepositoryGroupsStore = create<RepositoryGroupsState>((set, get) => ({
  // Initial state
  groups: [],
  selectedGroupId: null,
  isLoading: false,
  hasFetched: false,
  error: null,

  // Computed
  selectedGroup: () => {
    const { groups, selectedGroupId } = get();
    return groups.find((g) => g.id === selectedGroupId);
  },

  // Actions
  fetchGroups: async (workspaceId?: string) => {
    try {
      set({ isLoading: true, error: null });
      const groups = await api.listRepositoryGroups(workspaceId);
      set({ groups, isLoading: false, hasFetched: true });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch repository groups",
        isLoading: false,
        hasFetched: true,
      });
    }
  },

  createGroup: async (workspaceId: string, name?: string, repositoryIds?: string[]) => {
    try {
      set({ isLoading: true, error: null });
      const group = await api.createRepositoryGroup(workspaceId, name, repositoryIds);
      set((state) => ({
        groups: [...state.groups, group],
        isLoading: false,
      }));
      return group;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create repository group";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  updateGroup: async (id: string, name?: string) => {
    try {
      set({ isLoading: true, error: null });
      const group = await api.updateRepositoryGroup(id, name);
      set((state) => ({
        groups: state.groups.map((g) =>
          g.id === id ? group : g
        ),
        isLoading: false,
      }));
      return group;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to update repository group";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  deleteGroup: async (id: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.deleteRepositoryGroup(id);
      set((state) => ({
        groups: state.groups.filter((g) => g.id !== id),
        selectedGroupId:
          state.selectedGroupId === id ? null : state.selectedGroupId,
        isLoading: false,
      }));
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to delete repository group",
        isLoading: false,
      });
    }
  },

  selectGroup: (id: string | null) => {
    set({ selectedGroupId: id });
  },

  addRepositoryToGroup: async (groupId: string, repositoryId: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.addRepositoryToGroup(groupId, repositoryId);
      set((state) => ({
        groups: state.groups.map((g) =>
          g.id === groupId
            ? { ...g, repository_ids: [...g.repository_ids, repositoryId] }
            : g
        ),
        isLoading: false,
      }));
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to add repository to group",
        isLoading: false,
      });
    }
  },

  removeRepositoryFromGroup: async (groupId: string, repositoryId: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.removeRepositoryFromGroup(groupId, repositoryId);
      set((state) => ({
        groups: state.groups.map((g) =>
          g.id === groupId
            ? { ...g, repository_ids: g.repository_ids.filter((id) => id !== repositoryId) }
            : g
        ),
        isLoading: false,
      }));
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to remove repository from group",
        isLoading: false,
      });
    }
  },

  getOrCreateSingleRepoGroup: async (workspaceId: string, repositoryId: string) => {
    try {
      set({ isLoading: true, error: null });
      const groupId = await api.getOrCreateSingleRepoGroup(workspaceId, repositoryId);

      // Refresh groups to include the new group
      const groups = await api.listRepositoryGroups(workspaceId);
      set({ groups, isLoading: false });

      return groupId;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to get or create single repo group";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  clearError: () => set({ error: null }),
}));
