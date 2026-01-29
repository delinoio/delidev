import { describe, it, expect, beforeEach } from "vitest";
import {
  useTabsStore,
  TabType,
  getTabTypeFromPath,
  getDefaultTitleForType,
  DASHBOARD_TAB_ID,
} from "./tabs";

describe("getTabTypeFromPath", () => {
  it('should return Dashboard for "/" path', () => {
    expect(getTabTypeFromPath("/")).toBe(TabType.Dashboard);
  });

  it("should return Dashboard for empty path", () => {
    expect(getTabTypeFromPath("")).toBe(TabType.Dashboard);
  });

  it("should return UnitTask for /unit-tasks/* paths", () => {
    expect(getTabTypeFromPath("/unit-tasks/123")).toBe(TabType.UnitTask);
    expect(getTabTypeFromPath("/unit-tasks/abc-def")).toBe(TabType.UnitTask);
  });

  it("should return CompositeTask for /composite-tasks/* paths", () => {
    expect(getTabTypeFromPath("/composite-tasks/123")).toBe(
      TabType.CompositeTask
    );
    expect(getTabTypeFromPath("/composite-tasks/abc-def")).toBe(
      TabType.CompositeTask
    );
  });

  it("should return Repositories for /repositories path", () => {
    expect(getTabTypeFromPath("/repositories")).toBe(TabType.Repositories);
  });

  it("should return RepositorySettings for /settings/repository/* paths", () => {
    expect(getTabTypeFromPath("/settings/repository/123")).toBe(
      TabType.RepositorySettings
    );
    expect(getTabTypeFromPath("/settings/repository/my-repo")).toBe(
      TabType.RepositorySettings
    );
  });

  it("should return Settings for /settings path", () => {
    expect(getTabTypeFromPath("/settings")).toBe(TabType.Settings);
  });

  it("should return Settings for /settings/* paths (not repository)", () => {
    expect(getTabTypeFromPath("/settings/general")).toBe(TabType.Settings);
    expect(getTabTypeFromPath("/settings/tokens")).toBe(TabType.Settings);
  });

  it("should return Chat for /chat path", () => {
    expect(getTabTypeFromPath("/chat")).toBe(TabType.Chat);
  });

  it("should return Dashboard for unknown paths", () => {
    expect(getTabTypeFromPath("/unknown")).toBe(TabType.Dashboard);
    expect(getTabTypeFromPath("/foo/bar")).toBe(TabType.Dashboard);
  });
});

describe("getDefaultTitleForType", () => {
  it("should return correct titles for all tab types", () => {
    expect(getDefaultTitleForType(TabType.Dashboard)).toBe("Dashboard");
    expect(getDefaultTitleForType(TabType.UnitTask)).toBe("Task");
    expect(getDefaultTitleForType(TabType.CompositeTask)).toBe("Composite Task");
    expect(getDefaultTitleForType(TabType.Repositories)).toBe("Repositories");
    expect(getDefaultTitleForType(TabType.Settings)).toBe("Settings");
    expect(getDefaultTitleForType(TabType.Chat)).toBe("Chat");
    expect(getDefaultTitleForType(TabType.RepositorySettings)).toBe(
      "Repository Settings"
    );
  });
});

describe("useTabsStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useTabsStore.setState({
      tabs: [
        {
          id: DASHBOARD_TAB_ID,
          type: TabType.Dashboard,
          title: "Dashboard",
          path: "/",
        },
      ],
      activeTabId: DASHBOARD_TAB_ID,
    });
  });

  describe("initial state", () => {
    it("should start with a dashboard tab", () => {
      const state = useTabsStore.getState();
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0].type).toBe(TabType.Dashboard);
      expect(state.tabs[0].id).toBe(DASHBOARD_TAB_ID);
      expect(state.activeTabId).toBe(DASHBOARD_TAB_ID);
    });
  });

  describe("openTab", () => {
    it("should add a new tab and set it as active", () => {
      const state = useTabsStore.getState();
      const newTabId = state.openTab({
        type: TabType.UnitTask,
        title: "My Task",
        path: "/unit-tasks/123",
        taskId: "123",
      });

      const updatedState = useTabsStore.getState();
      expect(updatedState.tabs).toHaveLength(2);
      expect(updatedState.activeTabId).toBe(newTabId);
      expect(updatedState.tabs[1].title).toBe("My Task");
      expect(updatedState.tabs[1].taskId).toBe("123");
    });

    it("should generate unique tab IDs", () => {
      const state = useTabsStore.getState();
      const id1 = state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/1",
      });
      const id2 = state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/2",
      });

      expect(id1).not.toBe(id2);
      expect(id1).toMatch(/^tab-\d+-[a-z0-9]+$/);
    });

    it("should allow multiple tabs with the same path", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/123",
      });
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/123",
      });

      const updatedState = useTabsStore.getState();
      expect(updatedState.tabs).toHaveLength(3);
      expect(updatedState.tabs[1].path).toBe("/unit-tasks/123");
      expect(updatedState.tabs[2].path).toBe("/unit-tasks/123");
    });
  });

  describe("closeTab", () => {
    it("should remove the tab", () => {
      const state = useTabsStore.getState();
      const newTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Task",
        path: "/unit-tasks/123",
      });

      expect(useTabsStore.getState().tabs).toHaveLength(2);

      useTabsStore.getState().closeTab(newTabId);

      expect(useTabsStore.getState().tabs).toHaveLength(1);
      expect(useTabsStore.getState().tabs[0].id).toBe(DASHBOARD_TAB_ID);
    });

    it("should not close the last remaining tab", () => {
      const state = useTabsStore.getState();
      state.closeTab(DASHBOARD_TAB_ID);

      expect(useTabsStore.getState().tabs).toHaveLength(1);
      expect(useTabsStore.getState().tabs[0].id).toBe(DASHBOARD_TAB_ID);
    });

    it("should switch to adjacent tab when closing active tab", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/1",
      });
      const thirdTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/2",
      });

      // Close the active (third) tab
      useTabsStore.getState().closeTab(thirdTabId);

      const updatedState = useTabsStore.getState();
      expect(updatedState.tabs).toHaveLength(2);
      // Should switch to the previous tab (index 1)
      expect(updatedState.activeTabId).not.toBe(thirdTabId);
    });

    it("should not change active tab when closing non-active tab", () => {
      const state = useTabsStore.getState();
      const secondTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/1",
      });
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/2",
      });

      const activeTabId = useTabsStore.getState().activeTabId;

      // Close non-active tab
      useTabsStore.getState().closeTab(secondTabId);

      expect(useTabsStore.getState().activeTabId).toBe(activeTabId);
    });

    it("should handle closing non-existent tab gracefully", () => {
      const state = useTabsStore.getState();
      const initialTabs = [...state.tabs];

      state.closeTab("non-existent-id");

      expect(useTabsStore.getState().tabs).toEqual(initialTabs);
    });
  });

  describe("setActiveTab", () => {
    it("should set the active tab", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task",
        path: "/unit-tasks/123",
      });

      useTabsStore.getState().setActiveTab(DASHBOARD_TAB_ID);

      expect(useTabsStore.getState().activeTabId).toBe(DASHBOARD_TAB_ID);
    });

    it("should not set active tab if tab does not exist", () => {
      const state = useTabsStore.getState();
      const initialActiveId = state.activeTabId;

      state.setActiveTab("non-existent-id");

      expect(useTabsStore.getState().activeTabId).toBe(initialActiveId);
    });
  });

  describe("updateTabTitle", () => {
    it("should update the title of a tab", () => {
      const state = useTabsStore.getState();
      const newTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Old Title",
        path: "/unit-tasks/123",
      });

      useTabsStore.getState().updateTabTitle(newTabId, "New Title");

      const updatedTab = useTabsStore
        .getState()
        .tabs.find((t) => t.id === newTabId);
      expect(updatedTab?.title).toBe("New Title");
    });

    it("should not affect other tabs when updating title", () => {
      const state = useTabsStore.getState();
      const originalDashboardTitle = state.tabs[0].title;

      const newTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Task",
        path: "/unit-tasks/123",
      });

      useTabsStore.getState().updateTabTitle(newTabId, "Updated Task");

      expect(useTabsStore.getState().tabs[0].title).toBe(originalDashboardTitle);
    });
  });

  describe("updateActiveTabPath", () => {
    it("should update path, type, and title of active tab", () => {
      const state = useTabsStore.getState();

      state.updateActiveTabPath("/unit-tasks/123");

      const updatedState = useTabsStore.getState();
      const activeTab = updatedState.tabs.find(
        (t) => t.id === updatedState.activeTabId
      );
      expect(activeTab?.path).toBe("/unit-tasks/123");
      expect(activeTab?.type).toBe(TabType.UnitTask);
      expect(activeTab?.title).toBe("Task");
    });

    it("should do nothing if no active tab", () => {
      useTabsStore.setState({ activeTabId: null });

      const tabsBefore = [...useTabsStore.getState().tabs];
      useTabsStore.getState().updateActiveTabPath("/unit-tasks/123");

      expect(useTabsStore.getState().tabs).toEqual(tabsBefore);
    });
  });

  describe("closeAllTabs", () => {
    it("should reset to only dashboard tab", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/1",
      });
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/2",
      });

      expect(useTabsStore.getState().tabs).toHaveLength(3);

      useTabsStore.getState().closeAllTabs();

      const updatedState = useTabsStore.getState();
      expect(updatedState.tabs).toHaveLength(1);
      expect(updatedState.tabs[0].id).toBe(DASHBOARD_TAB_ID);
      expect(updatedState.activeTabId).toBe(DASHBOARD_TAB_ID);
    });
  });

  describe("closeOtherTabs", () => {
    it("should keep only the specified tab", () => {
      const state = useTabsStore.getState();
      const secondTabId = state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/1",
      });
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/2",
      });

      expect(useTabsStore.getState().tabs).toHaveLength(3);

      useTabsStore.getState().closeOtherTabs(secondTabId);

      const updatedState = useTabsStore.getState();
      expect(updatedState.tabs).toHaveLength(1);
      expect(updatedState.tabs[0].id).toBe(secondTabId);
      expect(updatedState.activeTabId).toBe(secondTabId);
    });

    it("should do nothing if tab does not exist", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task",
        path: "/unit-tasks/1",
      });

      const tabsBefore = useTabsStore.getState().tabs.length;

      useTabsStore.getState().closeOtherTabs("non-existent-id");

      expect(useTabsStore.getState().tabs).toHaveLength(tabsBefore);
    });
  });

  describe("getTabByPath", () => {
    it("should find tab by path", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task",
        path: "/unit-tasks/123",
      });

      const tab = useTabsStore.getState().getTabByPath("/unit-tasks/123");

      expect(tab).toBeDefined();
      expect(tab?.title).toBe("Task");
    });

    it("should return undefined for non-existent path", () => {
      const tab = useTabsStore.getState().getTabByPath("/non-existent");

      expect(tab).toBeUndefined();
    });

    it("should return first matching tab when multiple tabs have same path", () => {
      const state = useTabsStore.getState();
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 1",
        path: "/unit-tasks/123",
      });
      state.openTab({
        type: TabType.UnitTask,
        title: "Task 2",
        path: "/unit-tasks/123",
      });

      const tab = useTabsStore.getState().getTabByPath("/unit-tasks/123");

      expect(tab?.title).toBe("Task 1");
    });
  });
});
