import { lazy, Suspense, useEffect, useRef, createContext, useMemo } from "react";
import { matchPath } from "react-router-dom";
import { useTabsStore, Tab, TabType, getTabTypeFromPath } from "../../stores/tabs";
import { Loader2 } from "lucide-react";

// Lazy-loaded page components (mirroring App.tsx)
const Dashboard = lazy(() =>
  import("../../pages/Dashboard").then((m) => ({ default: m.Dashboard }))
);
const Repositories = lazy(() =>
  import("../../pages/Repositories").then((m) => ({ default: m.Repositories }))
);
const Settings = lazy(() =>
  import("../../pages/Settings").then((m) => ({ default: m.Settings }))
);
const RepositorySettings = lazy(() =>
  import("../../pages/RepositorySettings").then((m) => ({
    default: m.RepositorySettings,
  }))
);
const Chat = lazy(() =>
  import("../../pages/Chat").then((m) => ({ default: m.Chat }))
);
const UnitTaskDetail = lazy(() =>
  import("../../pages/UnitTaskDetail").then((m) => ({
    default: m.UnitTaskDetail,
  }))
);
const CompositeTaskDetail = lazy(() =>
  import("../../pages/CompositeTaskDetail").then((m) => ({
    default: m.CompositeTaskDetail,
  }))
);
const RepositoryGroups = lazy(() =>
  import("../../pages/RepositoryGroups").then((m) => ({
    default: m.RepositoryGroups,
  }))
);

// Loading fallback component
const PageLoadingFallback = (
  <div className="flex-1 flex items-center justify-center">
    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
  </div>
);

// Route patterns for matching (must match App.tsx routes)
const ROUTE_PATTERNS = [
  { pattern: "/", type: TabType.Dashboard },
  { pattern: "/repositories", type: TabType.Repositories },
  { pattern: "/repository-groups", type: TabType.RepositoryGroups },
  { pattern: "/settings/:tab", type: TabType.Settings },
  { pattern: "/settings/repository/:id", type: TabType.RepositorySettings },
  { pattern: "/chat", type: TabType.Chat },
  { pattern: "/unit-tasks/:id", type: TabType.UnitTask },
  { pattern: "/composite-tasks/:id", type: TabType.CompositeTask },
] as const;

/**
 * Context to provide tab-specific route params to components.
 * This allows components to get params from their tab's path instead of the current URL.
 */
interface TabParamsContextValue {
  params: Record<string, string>;
  tabPath: string;
}

export const TabParamsContext = createContext<TabParamsContextValue | null>(null);

// Extract params from a path using route patterns
function extractParamsFromPath(path: string): Record<string, string> {
  for (const route of ROUTE_PATTERNS) {
    // Need to check repository settings first as it's more specific
    if (route.pattern === "/settings/repository/:id") {
      const match = matchPath(route.pattern, path);
      if (match) {
        return match.params as Record<string, string>;
      }
    }
  }

  for (const route of ROUTE_PATTERNS) {
    if (route.pattern === "/settings/repository/:id") continue; // Already checked
    const match = matchPath(route.pattern, path);
    if (match) {
      return match.params as Record<string, string>;
    }
  }

  return {};
}

// Render the appropriate page component based on tab type and path
function renderPageComponent(tabType: TabType) {
  switch (tabType) {
    case TabType.Dashboard:
      return <Dashboard />;
    case TabType.Repositories:
      return <Repositories />;
    case TabType.RepositoryGroups:
      return <RepositoryGroups />;
    case TabType.Settings:
      return <Settings />;
    case TabType.RepositorySettings:
      return <RepositorySettings />;
    case TabType.Chat:
      return <Chat />;
    case TabType.UnitTask:
      return <UnitTaskDetail />;
    case TabType.CompositeTask:
      return <CompositeTaskDetail />;
    default:
      return <Dashboard />;
  }
}

interface TabPanelProps {
  tab: Tab;
  isActive: boolean;
}

function TabPanel({ tab, isActive }: TabPanelProps) {
  const tabType = getTabTypeFromPath(tab.path);

  // Memoize params extraction to avoid recalculating on every render
  const contextValue = useMemo<TabParamsContextValue>(() => ({
    params: extractParamsFromPath(tab.path),
    tabPath: tab.path,
  }), [tab.path]);

  return (
    <div
      role="tabpanel"
      id={`tabpanel-${tab.id}`}
      aria-labelledby={`tab-${tab.id}`}
      hidden={!isActive}
      style={{ display: isActive ? 'flex' : 'none' }}
      className="flex-1 flex-col overflow-auto"
    >
      <div className="container mx-auto p-6">
        <TabParamsContext.Provider value={contextValue}>
          <Suspense fallback={PageLoadingFallback}>
            {renderPageComponent(tabType)}
          </Suspense>
        </TabParamsContext.Provider>
      </div>
    </div>
  );
}

/**
 * TabContent renders all open tabs' content simultaneously,
 * showing only the active tab while keeping others mounted to preserve state.
 *
 * This component replaces React Router's <Outlet /> in the Layout to enable
 * tab state preservation. Each tab's content remains mounted (but hidden)
 * when switching between tabs, preserving local component state.
 */
export function TabContent() {
  const tabs = useTabsStore((state) => state.tabs);
  const activeTabId = useTabsStore((state) => state.activeTabId);

  // Track tabs that have been rendered at least once
  // This prevents mounting tabs that haven't been viewed yet (lazy rendering)
  const renderedTabsRef = useRef<Set<string>>(new Set());

  // Add active tab to rendered set
  useEffect(() => {
    if (activeTabId) {
      renderedTabsRef.current.add(activeTabId);
    }
  }, [activeTabId]);

  // Clean up removed tabs from the rendered set
  useEffect(() => {
    const currentTabIds = new Set(tabs.map(t => t.id));
    for (const id of renderedTabsRef.current) {
      if (!currentTabIds.has(id)) {
        renderedTabsRef.current.delete(id);
      }
    }
  }, [tabs]);

  return (
    <>
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        // Only render tabs that are currently active or have been active before
        const shouldRender = isActive || renderedTabsRef.current.has(tab.id);

        if (!shouldRender) {
          return null;
        }

        return <TabPanel key={tab.id} tab={tab} isActive={isActive} />;
      })}
    </>
  );
}
