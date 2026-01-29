import { memo, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { X, Home, ListTodo, Layers, Settings, MessageSquare, FolderGit2, Plus } from "lucide-react";
import { useTabsStore, TabType, type Tab } from "../../stores/tabs";
import { cn } from "../../lib/utils";

interface TabBarProps {
  className?: string;
}

// Get icon for tab type
function getTabIcon(type: TabType) {
  switch (type) {
    case TabType.Dashboard:
      return <Home className="h-3.5 w-3.5" />;
    case TabType.UnitTask:
      return <ListTodo className="h-3.5 w-3.5" />;
    case TabType.CompositeTask:
      return <Layers className="h-3.5 w-3.5" />;
    case TabType.Repositories:
      return <FolderGit2 className="h-3.5 w-3.5" />;
    case TabType.RepositoryGroups:
      return <Layers className="h-3.5 w-3.5" />;
    case TabType.Settings:
    case TabType.RepositorySettings:
      return <Settings className="h-3.5 w-3.5" />;
    case TabType.Chat:
      return <MessageSquare className="h-3.5 w-3.5" />;
    default:
      return null;
  }
}

interface TabItemProps {
  tab: Tab;
  isActive: boolean;
  onActivate: () => void;
  onClose: () => void;
  canClose: boolean;
  // Array index of this tab (not to be confused with HTML tabIndex attribute)
  index: number;
  // Callback to register the tab element ref
  registerRef: (id: string, element: HTMLDivElement | null) => void;
}

const TabItem = memo(function TabItem({ tab, isActive, onActivate, onClose, canClose, index: _index, registerRef }: TabItemProps) {
  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClose();
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    // Middle click to close
    if (e.button === 1 && canClose) {
      e.preventDefault();
      onClose();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Activate tab on Enter or Space
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onActivate();
    }
  };

  // Callback ref to register/unregister the element
  const refCallback = useCallback(
    (element: HTMLDivElement | null) => {
      registerRef(tab.id, element);
    },
    [registerRef, tab.id]
  );

  return (
    <div
      ref={refCallback}
      role="tab"
      aria-selected={isActive}
      aria-controls="tab-panel"
      tabIndex={isActive ? 0 : -1}
      className={cn(
        "group flex items-center gap-1.5 px-3 py-1.5 text-sm cursor-pointer select-none",
        "border-r border-border transition-colors min-w-0 max-w-[200px]",
        "focus:outline-none focus:ring-2 focus:ring-primary focus:ring-inset",
        isActive
          ? "bg-background text-foreground"
          : "bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground"
      )}
      onClick={onActivate}
      onMouseDown={handleMouseDown}
      onKeyDown={handleKeyDown}
    >
      <span className="shrink-0">{getTabIcon(tab.type)}</span>
      <span className="truncate">{tab.title}</span>
      {canClose && (
        <button
          className={cn(
            "shrink-0 p-0.5 rounded hover:bg-muted-foreground/20",
            "opacity-0 group-hover:opacity-100 transition-opacity",
            isActive && "opacity-60"
          )}
          onClick={handleClose}
          aria-label={`Close ${tab.title}`}
          tabIndex={-1}
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </div>
  );
});

export function TabBar({ className }: TabBarProps) {
  const navigate = useNavigate();
  const { tabs, activeTabId, setActiveTab, closeTab } = useTabsStore();

  // Ref Map for storing tab element references
  const tabRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  // Stable callback for registering tab refs
  const registerRef = useCallback((id: string, element: HTMLDivElement | null) => {
    if (element) {
      tabRefs.current.set(id, element);
    } else {
      tabRefs.current.delete(id);
    }
  }, []);

  const handleActivateTab = (tab: Tab) => {
    setActiveTab(tab.id);
    navigate(tab.path);
  };

  // Note: We use getState() here instead of the tabs/activeTabId from the hook
  // to avoid stale closure issues. When this function is called, we need the
  // latest state to correctly compute the navigation target before closing the tab.
  const handleCloseTab = (tabId: string) => {
    const { tabs, activeTabId } = useTabsStore.getState();
    const closingActiveTab = tabId === activeTabId;
    const closingTabIndex = tabs.findIndex((t) => t.id === tabId);

    // Compute navigation target before closing the tab to avoid stale state
    let navigationTarget: string | null = null;
    if (closingActiveTab && tabs.length > 1) {
      const newTabs = tabs.filter((t) => t.id !== tabId);
      const newActiveIndex = Math.min(closingTabIndex, newTabs.length - 1);
      const newActiveTab = newTabs[newActiveIndex];
      if (newActiveTab) {
        navigationTarget = newActiveTab.path;
      }
    }

    // Close the tab
    closeTab(tabId);

    // Navigate after closing
    if (navigationTarget) {
      navigate(navigationTarget);
    }
  };

  // Handle keyboard navigation for arrow keys
  const handleKeyDown = (e: React.KeyboardEvent) => {
    const currentIndex = tabs.findIndex((t) => t.id === activeTabId);
    let newIndex: number | null = null;

    switch (e.key) {
      case "ArrowLeft":
        e.preventDefault();
        newIndex = currentIndex > 0 ? currentIndex - 1 : tabs.length - 1;
        break;
      case "ArrowRight":
        e.preventDefault();
        newIndex = currentIndex < tabs.length - 1 ? currentIndex + 1 : 0;
        break;
      case "Home":
        e.preventDefault();
        newIndex = 0;
        break;
      case "End":
        e.preventDefault();
        newIndex = tabs.length - 1;
        break;
    }

    if (newIndex !== null && newIndex !== currentIndex) {
      const newTab = tabs[newIndex];
      handleActivateTab(newTab);
      // Focus the new tab element using ref
      tabRefs.current.get(newTab.id)?.focus();
    }
  };

  const handleNewTab = () => {
    // Open a new dashboard tab
    const { openTab } = useTabsStore.getState();
    openTab({
      type: TabType.Dashboard,
      title: "Dashboard",
      path: "/",
    });
    navigate("/");
  };

  return (
    <div
      role="tablist"
      aria-label="Open tabs"
      className={cn(
        "flex items-stretch bg-muted border-b border-border overflow-x-auto",
        className
      )}
      onKeyDown={handleKeyDown}
    >
      {tabs.map((tab, index) => (
        <TabItem
          key={tab.id}
          tab={tab}
          isActive={tab.id === activeTabId}
          onActivate={() => handleActivateTab(tab)}
          onClose={() => handleCloseTab(tab.id)}
          canClose={tabs.length > 1}
          index={index}
          registerRef={registerRef}
        />
      ))}
      <button
        className={cn(
          "flex items-center justify-center px-2 py-1.5",
          "text-muted-foreground hover:text-foreground hover:bg-muted",
          "transition-colors border-r border-border"
        )}
        onClick={handleNewTab}
        aria-label="New tab"
        title="New tab"
      >
        <Plus className="h-4 w-4" />
      </button>
    </div>
  );
}
