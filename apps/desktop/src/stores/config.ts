import { create } from "zustand";
import type { GlobalConfig, VCSProviderType } from "../types";
import * as api from "../api";

interface ConfigState {
  // State
  globalConfig: GlobalConfig | null;
  credentialsStatus: api.CredentialsStatus | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  fetchGlobalConfig: () => Promise<void>;
  updateGlobalConfig: (config: GlobalConfig) => Promise<void>;
  fetchCredentialsStatus: () => Promise<void>;
  setGithubToken: (token: string) => Promise<api.VCSUser>;
  setGitlabToken: (token: string) => Promise<api.VCSUser>;
  setBitbucketCredentials: (
    username: string,
    appPassword: string
  ) => Promise<api.VCSUser>;
  validateCredentials: (provider: VCSProviderType) => Promise<api.VCSUser>;
  clearError: () => void;
}

export const useConfigStore = create<ConfigState>((set, get) => ({
  // Initial state
  globalConfig: null,
  credentialsStatus: null,
  isLoading: false,
  error: null,

  // Actions
  fetchGlobalConfig: async () => {
    try {
      set({ isLoading: true, error: null });
      const globalConfig = await api.getGlobalConfig();
      set({ globalConfig, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch config",
        isLoading: false,
      });
    }
  },

  updateGlobalConfig: async (config: GlobalConfig) => {
    try {
      set({ isLoading: true, error: null });
      await api.updateGlobalConfig(config);
      set({ globalConfig: config, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to update config",
        isLoading: false,
      });
    }
  },

  fetchCredentialsStatus: async () => {
    try {
      set({ isLoading: true, error: null });
      const credentialsStatus = await api.getCredentialsStatus();
      set({ credentialsStatus, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error
            ? error.message
            : "Failed to fetch credentials status",
        isLoading: false,
      });
    }
  },

  setGithubToken: async (token: string) => {
    try {
      set({ isLoading: true, error: null });
      const user = await api.setGithubToken(token);
      // Refresh credentials status
      const credentialsStatus = await api.getCredentialsStatus();
      set({ credentialsStatus, isLoading: false });
      return user;
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to set GitHub token";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  setGitlabToken: async (token: string) => {
    try {
      set({ isLoading: true, error: null });
      const user = await api.setGitlabToken(token);
      const credentialsStatus = await api.getCredentialsStatus();
      set({ credentialsStatus, isLoading: false });
      return user;
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to set GitLab token";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  setBitbucketCredentials: async (username: string, appPassword: string) => {
    try {
      set({ isLoading: true, error: null });
      const user = await api.setBitbucketCredentials(username, appPassword);
      const credentialsStatus = await api.getCredentialsStatus();
      set({ credentialsStatus, isLoading: false });
      return user;
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to set Bitbucket credentials";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  validateCredentials: async (provider: VCSProviderType) => {
    try {
      set({ isLoading: true, error: null });
      const user = await api.validateVcsCredentials(provider);
      set({ isLoading: false });
      return user;
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to validate credentials";
      set({ error: message, isLoading: false });
      throw new Error(message);
    }
  },

  clearError: () => set({ error: null }),
}));
