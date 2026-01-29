import { useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Enum for notification task types - must match the Rust side
 */
export enum NotificationTaskType {
  UnitTask = "unit_task",
  CompositeTask = "composite_task",
}

/**
 * Interface for notification click payload from the backend.
 * This matches the NotificationClickPayload struct in native_notification.rs
 */
interface NotificationClickPayload {
  task_type: string;
  task_id: string;
}

/**
 * Hook that sets up notification click handlers to navigate to task detail pages.
 *
 * When a notification with task context is clicked, this hook will:
 * 1. Focus the application window
 * 2. Navigate to the relevant task detail page (unit or composite task)
 *
 * This hook listens for the "notification-clicked" Tauri event emitted by
 * the native notification service when a user clicks on a notification.
 *
 * This hook should be used at the app root level (e.g., in App.tsx or AppContent).
 */
// Registration state: "idle" | "registering" | "registered"
type RegistrationState = "idle" | "registering" | "registered";

export function useNotificationClickHandler(): void {
  const navigate = useNavigate();
  // Use a single ref for registration state to avoid race conditions
  const registrationStateRef = useRef<RegistrationState>("idle");

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let isMounted = true;

    async function setupNotificationHandlers() {
      // Only register once per app lifecycle
      // Atomic check-and-set to prevent race conditions in React Strict Mode
      if (registrationStateRef.current !== "idle") {
        return;
      }

      registrationStateRef.current = "registering";

      try {
        // Only proceed if the component is still mounted
        if (!isMounted) {
          registrationStateRef.current = "idle";
          return;
        }

        // Listen for the notification-clicked event from the native notification service
        unlisten = await listen<NotificationClickPayload>(
          "notification-clicked",
          (event) => {
            handleNotificationClick(event.payload);
          }
        );

        registrationStateRef.current = "registered";
        console.log("Native notification click handlers registered successfully");
      } catch (error) {
        registrationStateRef.current = "idle";
        console.error("Failed to setup notification handlers:", error);
      }
    }

    function handleNotificationClick(payload: NotificationClickPayload) {
      const { task_type: taskType, task_id: taskId } = payload;

      if (!taskType || !taskId) {
        console.log("Notification clicked without task context");
        return;
      }

      // Focus the application window
      focusWindow();

      // Navigate to the appropriate task detail page
      try {
        if (taskType === NotificationTaskType.UnitTask) {
          navigate(`/unit-tasks/${taskId}`);
        } else if (taskType === NotificationTaskType.CompositeTask) {
          navigate(`/composite-tasks/${taskId}`);
        } else {
          console.log(`Unknown task type: ${taskType}`);
          return;
        }

        console.log(`Navigated to ${taskType} detail page: ${taskId}`);
      } catch (error) {
        console.error("Failed to navigate to task:", error);
      }
    }

    async function focusWindow() {
      try {
        const window = getCurrentWindow();
        await window.setFocus();
        // Also unminimize if minimized
        if (await window.isMinimized()) {
          await window.unminimize();
        }
      } catch (error) {
        console.error("Failed to focus window:", error);
      }
    }

    setupNotificationHandlers();

    // Cleanup function
    return () => {
      isMounted = false;
      registrationStateRef.current = "idle";
      if (unlisten) {
        unlisten();
      }
    };
  }, [navigate]);
}
