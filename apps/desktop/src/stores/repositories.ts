import { create } from "zustand";
import type { Repository } from "../types";
import * as api from "../api";

interface RepositoriesState {
  // State
  repositories: Repository[];
  selectedRepositoryId: string | null;
  isLoading: boolean;
  hasFetched: boolean;
  error: string | null;

  // Computed
  selectedRepository: () => Repository | undefined;

  // Actions
  fetchRepositories: () => Promise<void>;
  addRepository: (path: string) => Promise<Repository>;
  addRepositoryByUrl: (remoteUrl: string, defaultBranch?: string) => Promise<Repository>;
  removeRepository: (id: string) => Promise<void>;
  selectRepository: (id: string | null) => void;
  clearError: () => void;
}

export const useRepositoriesStore = create<RepositoriesState>((set, get) => ({
  // Initial state
  repositories: [],
  selectedRepositoryId: null,
  isLoading: false,
  hasFetched: false,
  error: null,

  // Computed
  selectedRepository: () => {
    const { repositories, selectedRepositoryId } = get();
    return repositories.find((r) => r.id === selectedRepositoryId);
  },

  // Actions
  fetchRepositories: async () => {
    try {
      set({ isLoading: true, error: null });
      const repositories = await api.listRepositories();
      set({ repositories, isLoading: false, hasFetched: true });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch repositories",
        isLoading: false,
        hasFetched: true,
      });
    }
  },

  addRepository: async (path: string) => {
    try {
      set({ isLoading: true, error: null });
      const repository = await api.addRepository(path);
      set((state) => ({
        repositories: [...state.repositories, repository],
        isLoading: false,
      }));
      return repository;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to add repository";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  addRepositoryByUrl: async (remoteUrl: string, defaultBranch?: string) => {
    try {
      set({ isLoading: true, error: null });
      const repository = await api.addRepositoryByUrl(remoteUrl, defaultBranch);
      set((state) => ({
        repositories: [...state.repositories, repository],
        isLoading: false,
      }));
      return repository;
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to add repository";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  removeRepository: async (id: string) => {
    try {
      set({ isLoading: true, error: null });
      await api.removeRepository(id);
      set((state) => ({
        repositories: state.repositories.filter((r) => r.id !== id),
        selectedRepositoryId:
          state.selectedRepositoryId === id ? null : state.selectedRepositoryId,
        isLoading: false,
      }));
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to remove repository",
        isLoading: false,
      });
    }
  },

  selectRepository: (id: string | null) => {
    set({ selectedRepositoryId: id });
  },

  clearError: () => set({ error: null }),
}));
