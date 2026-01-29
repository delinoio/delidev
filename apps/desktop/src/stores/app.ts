import { create } from "zustand";
import * as api from "../api";

interface AppState {
  // State
  isInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  dockerAvailable: boolean;
  appInfo: api.AppInfo | null;

  // Actions
  initialize: () => Promise<void>;
  clearError: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  isInitialized: false,
  isLoading: true,
  error: null,
  dockerAvailable: false,
  appInfo: null,

  // Actions
  initialize: async () => {
    try {
      set({ isLoading: true, error: null });

      const [appInfo, dockerAvailable] = await Promise.all([
        api.getAppInfo(),
        api.checkDocker().catch(() => false),
      ]);

      set({
        appInfo,
        dockerAvailable,
        isInitialized: true,
        isLoading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to initialize",
        isLoading: false,
      });
    }
  },

  clearError: () => set({ error: null }),
}));
