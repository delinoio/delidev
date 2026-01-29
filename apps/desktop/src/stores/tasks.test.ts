import { describe, it, expect, beforeEach, vi } from "vitest";
import { useTasksStore } from "./tasks";
import * as api from "../api";
import { CompositeTaskStatusKey } from "../api";
import { UnitTaskStatus, CompositeTaskStatus } from "../types";
import type { UnitTask, CompositeTask } from "../types";

// Mock the API module
vi.mock("../api", () => ({
  listUnitTasks: vi.fn(),
  listCompositeTasks: vi.fn(),
  getTasksByStatus: vi.fn(),
  createUnitTask: vi.fn(),
  createCompositeTask: vi.fn(),
  updateUnitTaskStatus: vi.fn(),
  deleteUnitTask: vi.fn(),
  CompositeTaskStatusKey: {
    InProgress: "composite_in_progress",
    InReview: "composite_in_review",
    Done: "composite_done",
    Rejected: "composite_rejected",
  },
}));

const mockUnitTask: UnitTask = {
  id: "task-1",
  repository_group_id: "repo-group-1",
  prompt: "Test prompt",
  title: "Test Task",
  status: UnitTaskStatus.InProgress,
  agent_task_id: "agent-task-1",
  branch_name: "feature/test",
  auto_fix_task_ids: [],
  last_execution_failed: false,
  created_at: "2024-01-01T00:00:00Z",
  updated_at: "2024-01-01T00:00:00Z",
};

const mockCompositeTask: CompositeTask = {
  id: "composite-1",
  repository_group_id: "repo-group-1",
  prompt: "Test composite prompt",
  title: "Test Composite Task",
  status: CompositeTaskStatus.Planning,
  planning_task_id: "planning-task-1",
  nodes: [],
  created_at: "2024-01-01T00:00:00Z",
  updated_at: "2024-01-01T00:00:00Z",
};

describe("useTasksStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useTasksStore.setState({
      unitTasks: [],
      compositeTasks: [],
      tasksByStatus: null,
      isLoading: false,
      error: null,
    });
    // Clear all mocks
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("should have correct initial state", () => {
      const state = useTasksStore.getState();
      expect(state.unitTasks).toEqual([]);
      expect(state.compositeTasks).toEqual([]);
      expect(state.tasksByStatus).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });
  });

  describe("fetchUnitTasks", () => {
    it("should fetch and set unit tasks", async () => {
      vi.mocked(api.listUnitTasks).mockResolvedValue([mockUnitTask]);

      await useTasksStore.getState().fetchUnitTasks();

      const state = useTasksStore.getState();
      expect(state.unitTasks).toEqual([mockUnitTask]);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it("should pass repository ID to API", async () => {
      vi.mocked(api.listUnitTasks).mockResolvedValue([]);

      await useTasksStore.getState().fetchUnitTasks("repo-123");

      expect(api.listUnitTasks).toHaveBeenCalledWith("repo-123");
    });

    it("should set loading state during fetch", async () => {
      let loadingDuringFetch = false;
      vi.mocked(api.listUnitTasks).mockImplementation(async () => {
        loadingDuringFetch = useTasksStore.getState().isLoading;
        return [];
      });

      await useTasksStore.getState().fetchUnitTasks();

      expect(loadingDuringFetch).toBe(true);
      expect(useTasksStore.getState().isLoading).toBe(false);
    });

    it("should handle errors", async () => {
      vi.mocked(api.listUnitTasks).mockRejectedValue(new Error("Network error"));

      await useTasksStore.getState().fetchUnitTasks();

      const state = useTasksStore.getState();
      expect(state.error).toBe("Network error");
      expect(state.isLoading).toBe(false);
    });

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.listUnitTasks).mockRejectedValue("Unknown error");

      await useTasksStore.getState().fetchUnitTasks();

      const state = useTasksStore.getState();
      expect(state.error).toBe("Failed to fetch unit tasks");
    });
  });

  describe("fetchCompositeTasks", () => {
    it("should fetch and set composite tasks", async () => {
      vi.mocked(api.listCompositeTasks).mockResolvedValue([mockCompositeTask]);

      await useTasksStore.getState().fetchCompositeTasks();

      const state = useTasksStore.getState();
      expect(state.compositeTasks).toEqual([mockCompositeTask]);
      expect(state.isLoading).toBe(false);
    });

    it("should handle errors", async () => {
      vi.mocked(api.listCompositeTasks).mockRejectedValue(
        new Error("Fetch failed")
      );

      await useTasksStore.getState().fetchCompositeTasks();

      expect(useTasksStore.getState().error).toBe("Fetch failed");
    });
  });

  describe("fetchTasksByStatus", () => {
    it("should fetch and set tasks by status", async () => {
      const mockTasksByStatus: api.TasksByStatus = {
        [UnitTaskStatus.InProgress]: [mockUnitTask],
        [UnitTaskStatus.InReview]: [],
        [UnitTaskStatus.Approved]: [],
        [UnitTaskStatus.PrOpen]: [],
        [UnitTaskStatus.Done]: [],
        [UnitTaskStatus.Rejected]: [],
        [CompositeTaskStatusKey.InProgress]: [],
        [CompositeTaskStatusKey.InReview]: [],
        [CompositeTaskStatusKey.Done]: [],
        [CompositeTaskStatusKey.Rejected]: [],
      };
      vi.mocked(api.getTasksByStatus).mockResolvedValue(mockTasksByStatus);

      await useTasksStore.getState().fetchTasksByStatus();

      const state = useTasksStore.getState();
      expect(state.tasksByStatus).toEqual(mockTasksByStatus);
    });
  });

  describe("createUnitTask", () => {
    it("should create task and add to list", async () => {
      vi.mocked(api.createUnitTask).mockResolvedValue(mockUnitTask);
      vi.mocked(api.getTasksByStatus).mockResolvedValue({
        [UnitTaskStatus.InProgress]: [mockUnitTask],
        [UnitTaskStatus.InReview]: [],
        [UnitTaskStatus.Approved]: [],
        [UnitTaskStatus.PrOpen]: [],
        [UnitTaskStatus.Done]: [],
        [UnitTaskStatus.Rejected]: [],
        [CompositeTaskStatusKey.InProgress]: [],
        [CompositeTaskStatusKey.InReview]: [],
        [CompositeTaskStatusKey.Done]: [],
        [CompositeTaskStatusKey.Rejected]: [],
      });

      const result = await useTasksStore.getState().createUnitTask({
        repositoryGroupId: "repo-group-1",
        prompt: "Test prompt",
        title: "Test Task",
      });

      expect(result).toEqual(mockUnitTask);
      expect(useTasksStore.getState().unitTasks).toContainEqual(mockUnitTask);
    });

    it("should prepend new task to the list", async () => {
      useTasksStore.setState({
        unitTasks: [{ ...mockUnitTask, id: "existing-task" }],
      });
      vi.mocked(api.createUnitTask).mockResolvedValue(mockUnitTask);
      vi.mocked(api.getTasksByStatus).mockResolvedValue({
        [UnitTaskStatus.InProgress]: [],
        [UnitTaskStatus.InReview]: [],
        [UnitTaskStatus.Approved]: [],
        [UnitTaskStatus.PrOpen]: [],
        [UnitTaskStatus.Done]: [],
        [UnitTaskStatus.Rejected]: [],
        [CompositeTaskStatusKey.InProgress]: [],
        [CompositeTaskStatusKey.InReview]: [],
        [CompositeTaskStatusKey.Done]: [],
        [CompositeTaskStatusKey.Rejected]: [],
      });

      await useTasksStore.getState().createUnitTask({
        repositoryGroupId: "repo-group-1",
        prompt: "Test prompt",
      });

      const state = useTasksStore.getState();
      expect(state.unitTasks[0]).toEqual(mockUnitTask);
      expect(state.unitTasks).toHaveLength(2);
    });

    it("should throw and set error on failure", async () => {
      vi.mocked(api.createUnitTask).mockRejectedValue(
        new Error("Creation failed")
      );

      await expect(
        useTasksStore.getState().createUnitTask({
          repositoryGroupId: "repo-group-1",
          prompt: "Test",
        })
      ).rejects.toThrow("Creation failed");

      expect(useTasksStore.getState().error).toBe("Creation failed");
    });
  });

  describe("createCompositeTask", () => {
    it("should create composite task and add to list", async () => {
      vi.mocked(api.createCompositeTask).mockResolvedValue(mockCompositeTask);
      vi.mocked(api.getTasksByStatus).mockResolvedValue({
        [UnitTaskStatus.InProgress]: [],
        [UnitTaskStatus.InReview]: [],
        [UnitTaskStatus.Approved]: [],
        [UnitTaskStatus.PrOpen]: [],
        [UnitTaskStatus.Done]: [],
        [UnitTaskStatus.Rejected]: [],
        [CompositeTaskStatusKey.InProgress]: [],
        [CompositeTaskStatusKey.InReview]: [],
        [CompositeTaskStatusKey.Done]: [],
        [CompositeTaskStatusKey.Rejected]: [],
      });

      const result = await useTasksStore.getState().createCompositeTask({
        repositoryGroupId: "repo-group-1",
        prompt: "Test prompt",
      });

      expect(result).toEqual(mockCompositeTask);
      expect(useTasksStore.getState().compositeTasks).toContainEqual(
        mockCompositeTask
      );
    });
  });

  describe("updateUnitTaskStatus", () => {
    it("should update task status in local state", async () => {
      useTasksStore.setState({ unitTasks: [mockUnitTask] });
      vi.mocked(api.updateUnitTaskStatus).mockResolvedValue(undefined);

      await useTasksStore.getState().updateUnitTaskStatus("task-1", UnitTaskStatus.Done);

      const updatedTask = useTasksStore
        .getState()
        .unitTasks.find((t) => t.id === "task-1");
      expect(updatedTask?.status).toBe(UnitTaskStatus.Done);
    });

    it("should not modify other tasks", async () => {
      const anotherTask: UnitTask = { ...mockUnitTask, id: "task-2", status: UnitTaskStatus.InReview };
      useTasksStore.setState({ unitTasks: [mockUnitTask, anotherTask] });
      vi.mocked(api.updateUnitTaskStatus).mockResolvedValue(undefined);

      await useTasksStore.getState().updateUnitTaskStatus("task-1", UnitTaskStatus.Done);

      const otherTask = useTasksStore
        .getState()
        .unitTasks.find((t) => t.id === "task-2");
      expect(otherTask?.status).toBe(UnitTaskStatus.InReview);
    });

    it("should refresh tasksByStatus if loaded", async () => {
      useTasksStore.setState({
        unitTasks: [mockUnitTask],
        tasksByStatus: {
          [UnitTaskStatus.InProgress]: [mockUnitTask],
          [UnitTaskStatus.InReview]: [],
          [UnitTaskStatus.Approved]: [],
          [UnitTaskStatus.PrOpen]: [],
          [UnitTaskStatus.Done]: [],
          [UnitTaskStatus.Rejected]: [],
          [CompositeTaskStatusKey.InProgress]: [],
          [CompositeTaskStatusKey.InReview]: [],
          [CompositeTaskStatusKey.Done]: [],
          [CompositeTaskStatusKey.Rejected]: [],
        },
      });
      vi.mocked(api.updateUnitTaskStatus).mockResolvedValue(undefined);
      vi.mocked(api.getTasksByStatus).mockResolvedValue({
        [UnitTaskStatus.InProgress]: [],
        [UnitTaskStatus.InReview]: [],
        [UnitTaskStatus.Approved]: [],
        [UnitTaskStatus.PrOpen]: [],
        [UnitTaskStatus.Done]: [mockUnitTask],
        [UnitTaskStatus.Rejected]: [],
        [CompositeTaskStatusKey.InProgress]: [],
        [CompositeTaskStatusKey.InReview]: [],
        [CompositeTaskStatusKey.Done]: [],
        [CompositeTaskStatusKey.Rejected]: [],
      });

      await useTasksStore.getState().updateUnitTaskStatus("task-1", UnitTaskStatus.Done);

      expect(api.getTasksByStatus).toHaveBeenCalled();
    });

    it("should handle errors", async () => {
      useTasksStore.setState({ unitTasks: [mockUnitTask] });
      vi.mocked(api.updateUnitTaskStatus).mockRejectedValue(
        new Error("Update failed")
      );

      await useTasksStore.getState().updateUnitTaskStatus("task-1", UnitTaskStatus.Done);

      expect(useTasksStore.getState().error).toBe("Update failed");
    });
  });

  describe("deleteUnitTask", () => {
    it("should remove task from local state", async () => {
      useTasksStore.setState({ unitTasks: [mockUnitTask] });
      vi.mocked(api.deleteUnitTask).mockResolvedValue(undefined);

      await useTasksStore.getState().deleteUnitTask("task-1");

      expect(useTasksStore.getState().unitTasks).toHaveLength(0);
    });

    it("should throw on failure", async () => {
      useTasksStore.setState({ unitTasks: [mockUnitTask] });
      vi.mocked(api.deleteUnitTask).mockRejectedValue(
        new Error("Delete failed")
      );

      await expect(
        useTasksStore.getState().deleteUnitTask("task-1")
      ).rejects.toThrow("Delete failed");
    });
  });

  describe("clearError", () => {
    it("should clear error state", () => {
      useTasksStore.setState({ error: "Some error" });

      useTasksStore.getState().clearError();

      expect(useTasksStore.getState().error).toBeNull();
    });
  });
});
