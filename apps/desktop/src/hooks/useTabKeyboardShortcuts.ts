import { useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useTabsStore, TabType } from "../stores/tabs";

/**
 * Hook to handle global keyboard shortcuts for tab management:
 * - Ctrl/Cmd + T: Open new tab
 * - Ctrl/Cmd + W: Close current tab (or go to dashboard if only one tab)
 * - Ctrl/Cmd + Tab: Next tab
 * - Ctrl/Cmd + Shift + Tab: Previous tab
 * - Ctrl/Cmd + 1-9: Switch to tab by index
 */
export function useTabKeyboardShortcuts() {
  const navigate = useNavigate();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Only handle shortcuts with Ctrl (Windows/Linux) or Cmd (Mac)
      const isMod = e.ctrlKey || e.metaKey;
      if (!isMod) return;

      // Don't trigger shortcuts when typing in input fields
      const target = e.target as HTMLElement;
      const isInputField =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      // Get current state
      const { tabs, activeTabId, setActiveTab, closeTab, openTab, closeAllTabs } =
        useTabsStore.getState();

      // Ctrl/Cmd + T: Open new tab
      if (e.key === "t" || e.key === "T") {
        // Only handle if not in an input field
        if (isInputField) return;

        e.preventDefault();

        // Open a new dashboard tab
        openTab({
          type: TabType.Dashboard,
          title: "Dashboard",
          path: "/",
        });
        navigate("/");
        return;
      }

      // Ctrl/Cmd + W: Close current tab or go to dashboard if only one tab
      if (e.key === "w" || e.key === "W") {
        // Only handle if not in an input field
        if (isInputField) return;

        e.preventDefault();

        if (tabs.length === 1) {
          // Only one tab: redirect to dashboard and clear history
          closeAllTabs();
          navigate("/");
        } else if (activeTabId) {
          // Multiple tabs: close current tab and navigate to adjacent tab
          const closingTabIndex = tabs.findIndex((t) => t.id === activeTabId);
          const newTabs = tabs.filter((t) => t.id !== activeTabId);
          const newActiveIndex = Math.min(closingTabIndex, newTabs.length - 1);
          const newActiveTab = newTabs[newActiveIndex];

          closeTab(activeTabId);
          if (newActiveTab) {
            navigate(newActiveTab.path);
          }
        }
        return;
      }

      // Ctrl/Cmd + Tab: Next tab
      // Ctrl/Cmd + Shift + Tab: Previous tab
      if (e.key === "Tab") {
        e.preventDefault();

        const currentIndex = tabs.findIndex((t) => t.id === activeTabId);
        let newIndex: number;

        if (e.shiftKey) {
          // Previous tab (wrap around)
          newIndex = currentIndex > 0 ? currentIndex - 1 : tabs.length - 1;
        } else {
          // Next tab (wrap around)
          newIndex = currentIndex < tabs.length - 1 ? currentIndex + 1 : 0;
        }

        const newTab = tabs[newIndex];
        if (newTab) {
          setActiveTab(newTab.id);
          navigate(newTab.path);
        }
        return;
      }

      // Ctrl/Cmd + 1-9: Switch to tab by index
      const numberKey = parseInt(e.key, 10);
      if (numberKey >= 1 && numberKey <= 9) {
        // Only handle if not in an input field
        if (isInputField) return;

        e.preventDefault();

        // 1-8 = first 8 tabs, 9 = last tab
        const targetIndex = numberKey === 9 ? tabs.length - 1 : numberKey - 1;
        const targetTab = tabs[targetIndex];

        if (targetTab) {
          setActiveTab(targetTab.id);
          navigate(targetTab.path);
        }
        return;
      }
    },
    [navigate]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleKeyDown]);
}
