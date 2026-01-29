import { useNavigate } from "react-router-dom";
import { useCallback } from "react";
import { useTabsStore, TabType, getTabTypeFromPath, getDefaultTitleForType } from "../stores/tabs";

interface OpenInNewTabOptions {
  path: string;
  title?: string;
  type?: TabType;
  taskId?: string;
}

/**
 * Hook for handling tab-aware navigation.
 * Supports Ctrl+Click (or Cmd+Click on Mac) to open in new tab.
 */
export function useTabNavigation() {
  const navigate = useNavigate();
  const { openTab } = useTabsStore();

  /**
   * Opens a path in a new tab (allows multiple tabs with the same path)
   */
  const openInNewTab = useCallback(
    (options: OpenInNewTabOptions) => {
      const { path, taskId } = options;
      const type = options.type ?? getTabTypeFromPath(path);
      const title = options.title ?? getDefaultTitleForType(type);

      // Open new tab (allow multiple tabs with the same path)
      openTab({
        type,
        title,
        path,
        taskId,
      });

      navigate(path);
    },
    [openTab, navigate]
  );

  /**
   * Handles click event with Ctrl/Cmd detection.
   * Returns true if navigation was handled (new tab opened).
   */
  const handleClick = useCallback(
    (e: React.MouseEvent, options: OpenInNewTabOptions): boolean => {
      // Check for Ctrl (Windows/Linux) or Cmd (Mac) key
      const isModifierPressed = e.ctrlKey || e.metaKey;

      if (isModifierPressed) {
        e.preventDefault();
        openInNewTab(options);
        return true;
      }

      // Regular click - let the default Link behavior handle it
      return false;
    },
    [openInNewTab]
  );

  /**
   * Navigate to a path, optionally opening in new tab based on modifier key
   */
  const navigateTo = useCallback(
    (path: string, options?: { title?: string; type?: TabType; taskId?: string; newTab?: boolean }) => {
      if (options?.newTab) {
        openInNewTab({
          path,
          title: options.title,
          type: options.type,
          taskId: options.taskId,
        });
      } else {
        navigate(path);
      }
    },
    [navigate, openInNewTab]
  );

  return {
    openInNewTab,
    handleClick,
    navigateTo,
  };
}
