import { create } from "zustand";

// Dashboard tab ID constant
export const DASHBOARD_TAB_ID = "tab-dashboard";

// Tab types
export enum TabType {
  Dashboard = "dashboard",
  UnitTask = "unit_task",
  CompositeTask = "composite_task",
  Repositories = "repositories",
  RepositoryGroups = "repository_groups",
  Settings = "settings",
  Chat = "chat",
  RepositorySettings = "repository_settings",
}

export interface Tab {
  id: string;
  type: TabType;
  title: string;
  path: string;
  // For task tabs, store the task ID for potential data refresh
  taskId?: string;
}

interface TabsState {
  // State
  tabs: Tab[];
  activeTabId: string | null;

  // Actions
  openTab: (tab: Omit<Tab, "id">) => string;
  closeTab: (tabId: string) => void;
  setActiveTab: (tabId: string) => void;
  updateTabTitle: (tabId: string, title: string) => void;
  updateActiveTabPath: (path: string) => void;
  closeAllTabs: () => void;
  closeOtherTabs: (tabId: string) => void;
  getTabByPath: (path: string) => Tab | undefined;
}

// Generate unique tab ID
const generateTabId = (): string => {
  return `tab-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
};

// Parse path to determine tab type
export const getTabTypeFromPath = (path: string): TabType => {
  if (path === "/" || path === "") {
    return TabType.Dashboard;
  }
  if (path.startsWith("/unit-tasks/")) {
    return TabType.UnitTask;
  }
  if (path.startsWith("/composite-tasks/")) {
    return TabType.CompositeTask;
  }
  if (path === "/repositories") {
    return TabType.Repositories;
  }
  if (path === "/repository-groups") {
    return TabType.RepositoryGroups;
  }
  if (path.startsWith("/settings/repository/")) {
    return TabType.RepositorySettings;
  }
  if (path.startsWith("/settings")) {
    return TabType.Settings;
  }
  if (path === "/chat") {
    return TabType.Chat;
  }
  return TabType.Dashboard;
};

// Get default title for a tab type
export const getDefaultTitleForType = (type: TabType): string => {
  switch (type) {
    case TabType.Dashboard:
      return "Dashboard";
    case TabType.UnitTask:
      return "Task";
    case TabType.CompositeTask:
      return "Composite Task";
    case TabType.Repositories:
      return "Repositories";
    case TabType.RepositoryGroups:
      return "Repository Groups";
    case TabType.Settings:
      return "Settings";
    case TabType.Chat:
      return "Chat";
    case TabType.RepositorySettings:
      return "Repository Settings";
    default:
      return "Tab";
  }
};

export const useTabsStore = create<TabsState>((set, get) => ({
  // Initial state - start with dashboard tab
  tabs: [
    {
      id: DASHBOARD_TAB_ID,
      type: TabType.Dashboard,
      title: "Dashboard",
      path: "/",
    },
  ],
  activeTabId: DASHBOARD_TAB_ID,

  // Actions
  openTab: (tab) => {
    const { tabs } = get();

    // Create new tab (allow multiple tabs with the same path)
    const newTab: Tab = {
      ...tab,
      id: generateTabId(),
    };

    set({
      tabs: [...tabs, newTab],
      activeTabId: newTab.id,
    });

    return newTab.id;
  },

  closeTab: (tabId) => {
    const { tabs, activeTabId } = get();

    // Don't close if it's the only tab
    if (tabs.length <= 1) {
      return;
    }

    const tabIndex = tabs.findIndex((t) => t.id === tabId);
    if (tabIndex === -1) {
      return;
    }

    const newTabs = tabs.filter((t) => t.id !== tabId);

    // If closing the active tab, switch to adjacent tab
    let newActiveTabId = activeTabId;
    if (activeTabId === tabId) {
      // Prefer the tab to the right, otherwise the tab to the left
      const newActiveIndex = Math.min(tabIndex, newTabs.length - 1);
      newActiveTabId = newTabs[newActiveIndex]?.id || null;
    }

    set({
      tabs: newTabs,
      activeTabId: newActiveTabId,
    });
  },

  setActiveTab: (tabId) => {
    const { tabs } = get();
    if (tabs.some((t) => t.id === tabId)) {
      set({ activeTabId: tabId });
    }
  },

  updateTabTitle: (tabId, title) => {
    const { tabs } = get();
    set({
      tabs: tabs.map((t) => (t.id === tabId ? { ...t, title } : t)),
    });
  },

  updateActiveTabPath: (path) => {
    const { activeTabId, tabs } = get();
    if (!activeTabId) return;

    const type = getTabTypeFromPath(path);
    const title = getDefaultTitleForType(type);

    set({
      tabs: tabs.map((t) =>
        t.id === activeTabId ? { ...t, path, type, title } : t
      ),
    });
  },

  closeAllTabs: () => {
    // Reset to just the dashboard tab
    set({
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
  },

  closeOtherTabs: (tabId) => {
    const { tabs } = get();
    const tabToKeep = tabs.find((t) => t.id === tabId);
    if (!tabToKeep) {
      return;
    }

    set({
      tabs: [tabToKeep],
      activeTabId: tabId,
    });
  },

  getTabByPath: (path) => {
    const { tabs } = get();
    return tabs.find((t) => t.path === path);
  },
}));
