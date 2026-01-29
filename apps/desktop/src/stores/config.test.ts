import { describe, it, expect, beforeEach, vi } from "vitest";
import { useConfigStore } from "./config";
import * as api from "../api";
import { AIAgentType, ContainerRuntime, EditorType, VCSProviderType } from "../types";
import type { GlobalConfig } from "../types";

// Mock the API module
vi.mock("../api", () => ({
  getGlobalConfig: vi.fn(),
  updateGlobalConfig: vi.fn(),
  getCredentialsStatus: vi.fn(),
  setGithubToken: vi.fn(),
  setGitlabToken: vi.fn(),
  setBitbucketCredentials: vi.fn(),
  validateVcsCredentials: vi.fn(),
}));

const mockGlobalConfig: GlobalConfig = {
  learning: { auto_learn_from_reviews: true },
  hotkey: { open_chat: "ctrl+shift+c" },
  notification: {
    enabled: true,
    approval_request: true,
    user_question: true,
    review_ready: true,
  },
  agent: {
    planning: { type: AIAgentType.ClaudeCode, model: "claude-opus-4-20250514" },
    execution: { type: AIAgentType.ClaudeCode, model: "claude-sonnet-4-20250514" },
    chat: { type: AIAgentType.ClaudeCode, model: "claude-sonnet-4-20250514" },
  },
  container: {
    runtime: ContainerRuntime.Docker,
    use_container: true,
  },
  editor: { editor_type: EditorType.Vscode },
  concurrency: { max_concurrent_sessions: 4 },
};

const mockCredentialsStatus: api.CredentialsStatus = {
  github_configured: true,
  gitlab_configured: false,
  bitbucket_configured: false,
};

const mockVCSUser: api.VCSUser = {
  username: "testuser",
  name: "Test User",
  avatar_url: "https://example.com/avatar.png",
};

describe("useConfigStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useConfigStore.setState({
      globalConfig: null,
      credentialsStatus: null,
      isLoading: false,
      error: null,
    });
    // Clear all mocks
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("should have correct initial state", () => {
      const state = useConfigStore.getState();
      expect(state.globalConfig).toBeNull();
      expect(state.credentialsStatus).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });
  });

  describe("fetchGlobalConfig", () => {
    it("should fetch and set global config", async () => {
      vi.mocked(api.getGlobalConfig).mockResolvedValue(mockGlobalConfig);

      await useConfigStore.getState().fetchGlobalConfig();

      const state = useConfigStore.getState();
      expect(state.globalConfig).toEqual(mockGlobalConfig);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it("should set loading state during fetch", async () => {
      let loadingDuringFetch = false;
      vi.mocked(api.getGlobalConfig).mockImplementation(async () => {
        loadingDuringFetch = useConfigStore.getState().isLoading;
        return mockGlobalConfig;
      });

      await useConfigStore.getState().fetchGlobalConfig();

      expect(loadingDuringFetch).toBe(true);
      expect(useConfigStore.getState().isLoading).toBe(false);
    });

    it("should handle errors", async () => {
      vi.mocked(api.getGlobalConfig).mockRejectedValue(
        new Error("Config fetch failed")
      );

      await useConfigStore.getState().fetchGlobalConfig();

      const state = useConfigStore.getState();
      expect(state.error).toBe("Config fetch failed");
      expect(state.isLoading).toBe(false);
    });

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.getGlobalConfig).mockRejectedValue("Unknown error");

      await useConfigStore.getState().fetchGlobalConfig();

      expect(useConfigStore.getState().error).toBe("Failed to fetch config");
    });
  });

  describe("updateGlobalConfig", () => {
    it("should update config", async () => {
      vi.mocked(api.updateGlobalConfig).mockResolvedValue(undefined);

      const updatedConfig = {
        ...mockGlobalConfig,
        learning: { auto_learn_from_reviews: false }
      };
      await useConfigStore.getState().updateGlobalConfig(updatedConfig);

      const state = useConfigStore.getState();
      expect(state.globalConfig).toEqual(updatedConfig);
      expect(state.isLoading).toBe(false);
    });

    it("should handle errors", async () => {
      vi.mocked(api.updateGlobalConfig).mockRejectedValue(
        new Error("Update failed")
      );

      await useConfigStore.getState().updateGlobalConfig(mockGlobalConfig);

      expect(useConfigStore.getState().error).toBe("Update failed");
    });
  });

  describe("fetchCredentialsStatus", () => {
    it("should fetch and set credentials status", async () => {
      vi.mocked(api.getCredentialsStatus).mockResolvedValue(
        mockCredentialsStatus
      );

      await useConfigStore.getState().fetchCredentialsStatus();

      const state = useConfigStore.getState();
      expect(state.credentialsStatus).toEqual(mockCredentialsStatus);
      expect(state.isLoading).toBe(false);
    });

    it("should handle errors", async () => {
      vi.mocked(api.getCredentialsStatus).mockRejectedValue(
        new Error("Credentials fetch failed")
      );

      await useConfigStore.getState().fetchCredentialsStatus();

      expect(useConfigStore.getState().error).toBe("Credentials fetch failed");
    });
  });

  describe("setGithubToken", () => {
    it("should set GitHub token and refresh credentials", async () => {
      vi.mocked(api.setGithubToken).mockResolvedValue(mockVCSUser);
      vi.mocked(api.getCredentialsStatus).mockResolvedValue(
        mockCredentialsStatus
      );

      const result = await useConfigStore.getState().setGithubToken("ghp_xxx");

      expect(result).toEqual(mockVCSUser);
      expect(api.setGithubToken).toHaveBeenCalledWith("ghp_xxx");
      expect(api.getCredentialsStatus).toHaveBeenCalled();
      expect(useConfigStore.getState().credentialsStatus).toEqual(
        mockCredentialsStatus
      );
    });

    it("should throw and set error on failure", async () => {
      vi.mocked(api.setGithubToken).mockRejectedValue(
        new Error("Invalid token")
      );

      await expect(
        useConfigStore.getState().setGithubToken("invalid")
      ).rejects.toThrow("Invalid token");

      expect(useConfigStore.getState().error).toBe("Invalid token");
    });

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.setGithubToken).mockRejectedValue("Unknown error");

      await expect(
        useConfigStore.getState().setGithubToken("token")
      ).rejects.toThrow("Failed to set GitHub token");
    });
  });

  describe("setGitlabToken", () => {
    it("should set GitLab token and refresh credentials", async () => {
      vi.mocked(api.setGitlabToken).mockResolvedValue(mockVCSUser);
      vi.mocked(api.getCredentialsStatus).mockResolvedValue(
        mockCredentialsStatus
      );

      const result = await useConfigStore.getState().setGitlabToken("glpat-xxx");

      expect(result).toEqual(mockVCSUser);
      expect(api.setGitlabToken).toHaveBeenCalledWith("glpat-xxx");
    });

    it("should throw on failure", async () => {
      vi.mocked(api.setGitlabToken).mockRejectedValue(
        new Error("Invalid token")
      );

      await expect(
        useConfigStore.getState().setGitlabToken("invalid")
      ).rejects.toThrow("Invalid token");
    });
  });

  describe("setBitbucketCredentials", () => {
    it("should set Bitbucket credentials and refresh status", async () => {
      vi.mocked(api.setBitbucketCredentials).mockResolvedValue(mockVCSUser);
      vi.mocked(api.getCredentialsStatus).mockResolvedValue(
        mockCredentialsStatus
      );

      const result = await useConfigStore
        .getState()
        .setBitbucketCredentials("user", "app-password");

      expect(result).toEqual(mockVCSUser);
      expect(api.setBitbucketCredentials).toHaveBeenCalledWith(
        "user",
        "app-password"
      );
    });

    it("should throw on failure", async () => {
      vi.mocked(api.setBitbucketCredentials).mockRejectedValue(
        new Error("Invalid credentials")
      );

      await expect(
        useConfigStore.getState().setBitbucketCredentials("user", "wrong")
      ).rejects.toThrow("Invalid credentials");
    });
  });

  describe("validateCredentials", () => {
    it("should validate credentials and return user", async () => {
      vi.mocked(api.validateVcsCredentials).mockResolvedValue(mockVCSUser);

      const result = await useConfigStore
        .getState()
        .validateCredentials(VCSProviderType.GitHub);

      expect(result).toEqual(mockVCSUser);
      expect(api.validateVcsCredentials).toHaveBeenCalledWith("github");
    });

    it("should throw on validation failure", async () => {
      vi.mocked(api.validateVcsCredentials).mockRejectedValue(
        new Error("Credentials expired")
      );

      await expect(
        useConfigStore.getState().validateCredentials("github" as VCSProviderType)
      ).rejects.toThrow("Credentials expired");

      expect(useConfigStore.getState().error).toBe("Credentials expired");
    });
  });

  describe("clearError", () => {
    it("should clear error state", () => {
      useConfigStore.setState({ error: "Some error" });

      useConfigStore.getState().clearError();

      expect(useConfigStore.getState().error).toBeNull();
    });
  });
});
