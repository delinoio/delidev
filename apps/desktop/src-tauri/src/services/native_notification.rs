use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// Task type identifiers for notification extra data.
/// Must match NotificationTaskType enum in useNotificationClickHandler.ts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationTaskType {
    UnitTask,
    CompositeTask,
}

impl NotificationTaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationTaskType::UnitTask => "unit_task",
            NotificationTaskType::CompositeTask => "composite_task",
        }
    }
}

/// Payload for notification click events
#[derive(Debug, Clone, serde::Serialize)]
pub struct NotificationClickPayload {
    pub task_type: String,
    pub task_id: String,
}

/// A notification with task context for click handling
#[derive(Debug, Clone)]
pub struct TaskNotification {
    pub title: String,
    pub body: String,
    pub task_type: NotificationTaskType,
    pub task_id: String,
}

/// Native notification service with click handler support.
/// Uses platform-specific APIs to provide notification click callbacks.
pub struct NativeNotificationService {
    #[cfg(target_os = "windows")]
    app_handle: Option<AppHandle>,
    #[cfg(target_os = "macos")]
    app_handle: Option<AppHandle>,
    #[cfg(target_os = "linux")]
    click_sender: Option<mpsc::UnboundedSender<NotificationClickPayload>>,
}

impl NativeNotificationService {
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        #[cfg(target_os = "linux")]
        {
            // On Linux, initialize with click handler if app_handle is provided
            if let Some(handle) = app_handle {
                let (tx, rx) = mpsc::unbounded_channel();
                let service = Self {
                    click_sender: Some(tx),
                };
                service.start_click_handler(handle, rx);
                return service;
            }
        }

        Self {
            #[cfg(target_os = "windows")]
            app_handle,
            #[cfg(target_os = "macos")]
            app_handle,
            #[cfg(target_os = "linux")]
            click_sender: None,
        }
    }

    /// Starts a background task that handles notification click events
    #[cfg(target_os = "linux")]
    fn start_click_handler(
        &self,
        app_handle: AppHandle,
        mut rx: mpsc::UnboundedReceiver<NotificationClickPayload>,
    ) {
        tokio::spawn(async move {
            while let Some(payload) = rx.recv().await {
                Self::emit_click_event(&app_handle, payload);
            }
        });
    }

    /// Emits a notification click event to the frontend
    fn emit_click_event(app_handle: &AppHandle, payload: NotificationClickPayload) {
        if let Err(e) = app_handle.emit("notification-clicked", payload.clone()) {
            tracing::warn!("Failed to emit notification-clicked event: {}", e);
        } else {
            tracing::debug!(
                "Emitted notification-clicked event for task: {}",
                payload.task_id
            );
        }
    }

    /// Shows a notification with task context.
    /// When the notification is clicked, it will emit a Tauri event.
    pub fn show(&self, notification: TaskNotification) {
        #[cfg(target_os = "windows")]
        self.show_windows(notification);

        #[cfg(target_os = "macos")]
        self.show_macos(notification);

        #[cfg(target_os = "linux")]
        self.show_linux(notification);
    }

    #[cfg(target_os = "windows")]
    fn show_windows(&self, notification: TaskNotification) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        use tauri_winrt_notification::{Duration, Toast};

        // Limit concurrent notification threads to prevent resource exhaustion
        static ACTIVE_THREADS: AtomicUsize = AtomicUsize::new(0);
        const MAX_NOTIFICATION_THREADS: usize = 10;

        let app_handle = match &self.app_handle {
            Some(h) => h.clone(),
            None => {
                tracing::warn!("Cannot show notification: no app handle");
                return;
            }
        };

        // Check thread limit before spawning
        let current = ACTIVE_THREADS.load(Ordering::SeqCst);
        if current >= MAX_NOTIFICATION_THREADS {
            tracing::warn!(
                "Too many notification threads active ({}), showing notification without click \
                 handler",
                current
            );
            // Show notification without click handling to avoid dropping it
            std::thread::spawn(move || {
                let result = Toast::new(Toast::POWERSHELL_APP_ID)
                    .title(&notification.title)
                    .text1(&notification.body)
                    .duration(Duration::Short)
                    .show();

                if let Err(e) = result {
                    tracing::warn!("Failed to show Windows notification: {:?}", e);
                }
            });
            return;
        }

        ACTIVE_THREADS.fetch_add(1, Ordering::SeqCst);

        let task_type = notification.task_type.as_str().to_string();
        let task_id = notification.task_id.clone();

        // Windows notifications need to run on a separate thread
        std::thread::spawn(move || {
            // Use a guard to ensure we decrement the counter on exit
            struct ThreadGuard;
            impl Drop for ThreadGuard {
                fn drop(&mut self) {
                    ACTIVE_THREADS.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _guard = ThreadGuard;

            let result = Toast::new(Toast::POWERSHELL_APP_ID)
                .title(&notification.title)
                .text1(&notification.body)
                .duration(Duration::Short)
                .on_activated(move |_action| {
                    let payload = NotificationClickPayload {
                        task_type: task_type.clone(),
                        task_id: task_id.clone(),
                    };
                    Self::emit_click_event(&app_handle, payload);
                    Ok(())
                })
                .show();

            if let Err(e) = result {
                tracing::warn!("Failed to show Windows notification: {:?}", e);
            }
        });
    }

    #[cfg(target_os = "macos")]
    fn show_macos(&self, notification: TaskNotification) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        use mac_notification_sys::{Notification, NotificationResponse};

        // Limit concurrent notification threads to prevent resource exhaustion
        static ACTIVE_THREADS: AtomicUsize = AtomicUsize::new(0);
        const MAX_NOTIFICATION_THREADS: usize = 10;

        let app_handle = match &self.app_handle {
            Some(h) => h.clone(),
            None => {
                tracing::warn!("Cannot show notification with click handler: no app handle");
                // Still show the notification without click handling
                let result = Notification::new()
                    .title(&notification.title)
                    .message(&notification.body)
                    .send();

                if let Err(e) = result {
                    tracing::warn!("Failed to show macOS notification: {:?}", e);
                }
                return;
            }
        };

        // Check thread limit before spawning
        let current = ACTIVE_THREADS.load(Ordering::SeqCst);
        if current >= MAX_NOTIFICATION_THREADS {
            tracing::warn!(
                "Too many notification threads active ({}), showing without click handler",
                current
            );
            // Show notification without click handling to avoid dropping it
            let result = Notification::new()
                .title(&notification.title)
                .message(&notification.body)
                .send();

            if let Err(e) = result {
                tracing::warn!("Failed to show macOS notification: {:?}", e);
            }
            return;
        }

        ACTIVE_THREADS.fetch_add(1, Ordering::SeqCst);

        let task_type = notification.task_type.as_str().to_string();
        let task_id = notification.task_id.clone();
        let title = notification.title.clone();
        let body = notification.body.clone();

        // macOS notifications with click handling need to be in a separate thread
        // because send() blocks until the notification is interacted with or dismissed
        std::thread::spawn(move || {
            // Use a guard to ensure we decrement the counter on exit
            struct ThreadGuard;
            impl Drop for ThreadGuard {
                fn drop(&mut self) {
                    ACTIVE_THREADS.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _guard = ThreadGuard;

            let result = Notification::new()
                .title(&title)
                .message(&body)
                .send();

            match result {
                Ok(response) => {
                    tracing::debug!("macOS notification response: {:?}", response);
                    // Handle notification click
                    if matches!(response, NotificationResponse::Click) {
                        let payload = NotificationClickPayload {
                            task_type: task_type.clone(),
                            task_id: task_id.clone(),
                        };
                        Self::emit_click_event(&app_handle, payload);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to show macOS notification: {:?}", e);
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    fn show_linux(&self, notification: TaskNotification) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        use notify_rust::{Notification, Timeout};

        // Limit concurrent notification threads to prevent resource exhaustion
        static ACTIVE_THREADS: AtomicUsize = AtomicUsize::new(0);
        const MAX_NOTIFICATION_THREADS: usize = 10;

        let sender = match &self.click_sender {
            Some(s) => s.clone(),
            None => {
                tracing::warn!("Cannot handle notification clicks: no sender configured");
                // Still show the notification without click handling
                let result = Notification::new()
                    .summary(&notification.title)
                    .body(&notification.body)
                    .show();

                if let Err(e) = result {
                    tracing::warn!("Failed to show Linux notification: {}", e);
                }
                return;
            }
        };

        // Check thread limit before spawning
        let current = ACTIVE_THREADS.load(Ordering::SeqCst);
        if current >= MAX_NOTIFICATION_THREADS {
            tracing::warn!(
                "Too many notification threads active ({}), showing without click handler",
                current
            );
            // Show notification without click handling to avoid dropping it
            let result = Notification::new()
                .summary(&notification.title)
                .body(&notification.body)
                .show();

            if let Err(e) = result {
                tracing::warn!("Failed to show Linux notification: {}", e);
            }
            return;
        }

        ACTIVE_THREADS.fetch_add(1, Ordering::SeqCst);

        let task_type = notification.task_type.as_str().to_string();
        let task_id = notification.task_id.clone();
        let title = notification.title.clone();
        let body = notification.body.clone();

        // Linux notifications with actions need to be handled in a separate thread
        // because wait_for_action blocks
        std::thread::spawn(move || {
            // Use a guard to ensure we decrement the counter on exit
            struct ThreadGuard;
            impl Drop for ThreadGuard {
                fn drop(&mut self) {
                    ACTIVE_THREADS.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _guard = ThreadGuard;

            let result = Notification::new()
                .summary(&title)
                .body(&body)
                .action("default", "Open")
                .action("view", "View Task")
                .timeout(Timeout::Milliseconds(30000)) // 30 second timeout
                .show();

            match result {
                Ok(handle) => {
                    // This blocks until the notification is dismissed or an action is taken
                    // The timeout above ensures this won't block indefinitely
                    handle.wait_for_action(|action| {
                        if action == "default" || action == "view" {
                            let payload = NotificationClickPayload {
                                task_type: task_type.clone(),
                                task_id: task_id.clone(),
                            };
                            if let Err(e) = sender.send(payload) {
                                tracing::warn!("Failed to send notification click: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to show Linux notification: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_task_type_as_str() {
        assert_eq!(NotificationTaskType::UnitTask.as_str(), "unit_task");
        assert_eq!(
            NotificationTaskType::CompositeTask.as_str(),
            "composite_task"
        );
    }
}
