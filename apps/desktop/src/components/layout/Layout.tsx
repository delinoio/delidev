import { useLocation } from "react-router-dom";
import { useState, useEffect } from "react";
import { Sidebar } from "./Sidebar";
import { TabBar } from "./TabBar";
import { TabContent } from "./TabContent";
import { CreateTaskDialog } from "../tasks/CreateTaskDialog";
import { useTabsStore } from "../../stores/tabs";
import { useTabKeyboardShortcuts } from "../../hooks";

export function Layout() {
  const [isCreateTaskOpen, setIsCreateTaskOpen] = useState(false);
  const location = useLocation();

  // Enable global keyboard shortcuts for tab management
  useTabKeyboardShortcuts();

  // Sync current route with tabs
  // Note: We use getState() to avoid recreated function references in the dependency array,
  // which would cause the effect to run on every render
  useEffect(() => {
    const { getTabByPath, setActiveTab, updateActiveTabPath } =
      useTabsStore.getState();
    const currentPath = location.pathname;
    const existingTab = getTabByPath(currentPath);

    if (existingTab) {
      // Tab exists, just activate it
      setActiveTab(existingTab.id);
    } else {
      // No tab for this path - this is a regular navigation (not Ctrl+Click)
      // Update the current active tab's path instead of opening a new one
      updateActiveTabPath(currentPath);
    }
  }, [location.pathname]);

  return (
    <div className="min-h-screen bg-background">
      <Sidebar onNewTask={() => setIsCreateTaskOpen(true)} />
      <main className="pl-64 flex flex-col h-screen">
        <TabBar />
        <TabContent />
      </main>
      <CreateTaskDialog
        open={isCreateTaskOpen}
        onOpenChange={setIsCreateTaskOpen}
      />
    </div>
  );
}
