use tauri::AppHandle;

use super::native_notification::{
    NativeNotificationService, NotificationTaskType, TaskNotification,
};
use crate::entities::{CompositeTaskStatus, UnitTaskStatus};

/// Service for managing desktop notifications.
/// Uses the native notification service for platform-specific click handling.
pub struct NotificationService {
    native_service: NativeNotificationService,
}

impl NotificationService {
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        Self {
            native_service: NativeNotificationService::new(app_handle),
        }
    }

    /// Sends a desktop notification for unit task status changes.
    /// Includes task context so clicking the notification navigates to the task
    /// detail page.
    pub fn notify_task_status_change(
        &self,
        task_id: &str,
        task_title: &str,
        old_status: UnitTaskStatus,
        new_status: UnitTaskStatus,
    ) {
        let (title, body) = match new_status {
            UnitTaskStatus::InProgress => (
                "Task Started".to_string(),
                format!("'{}' is now in progress", task_title),
            ),
            UnitTaskStatus::InReview => (
                "Task Ready for Review".to_string(),
                format!("'{}' is ready for your review", task_title),
            ),
            UnitTaskStatus::Approved => (
                "Task Approved".to_string(),
                format!("'{}' has been approved", task_title),
            ),
            UnitTaskStatus::PrOpen => (
                "PR Created".to_string(),
                format!("Pull request created for '{}'", task_title),
            ),
            UnitTaskStatus::Done => (
                "Task Completed".to_string(),
                format!("'{}' has been completed", task_title),
            ),
            UnitTaskStatus::Rejected => (
                "Task Rejected".to_string(),
                format!("'{}' has been rejected", task_title),
            ),
        };

        // Only send notification if status actually changed
        if old_status != new_status {
            self.native_service.show(TaskNotification {
                title,
                body,
                task_type: NotificationTaskType::UnitTask,
                task_id: task_id.to_string(),
            });
        }
    }

    /// Sends a desktop notification for composite task status changes.
    /// Includes task context so clicking the notification navigates to the task
    /// detail page.
    pub fn notify_composite_task_status_change(
        &self,
        task_id: &str,
        old_status: CompositeTaskStatus,
        new_status: CompositeTaskStatus,
    ) {
        let (title, body) = match new_status {
            CompositeTaskStatus::Planning => (
                "Planning Started".to_string(),
                "Planning task graph for composite task".to_string(),
            ),
            CompositeTaskStatus::PendingApproval => (
                "Plan Ready for Approval".to_string(),
                "Composite task plan is ready for your approval".to_string(),
            ),
            CompositeTaskStatus::InProgress => (
                "Composite Task Executing".to_string(),
                "Composite task is now executing".to_string(),
            ),
            CompositeTaskStatus::Done => (
                "Composite Task Completed".to_string(),
                "Composite task has been completed".to_string(),
            ),
            CompositeTaskStatus::Rejected => (
                "Composite Task Rejected".to_string(),
                "Composite task has been rejected".to_string(),
            ),
        };

        // Only send notification if status actually changed
        if old_status != new_status {
            self.native_service.show(TaskNotification {
                title,
                body,
                task_type: NotificationTaskType::CompositeTask,
                task_id: task_id.to_string(),
            });
        }
    }

    /// Sends a notification when execution fails.
    /// Includes task context so clicking the notification navigates to the task
    /// detail page.
    pub fn notify_execution_error(&self, task_id: &str, task_title: &str, error: &str) {
        self.native_service.show(TaskNotification {
            title: "Task Execution Failed".to_string(),
            body: format!("'{}' failed: {}", task_title, error),
            task_type: NotificationTaskType::UnitTask,
            task_id: task_id.to_string(),
        });
    }

    /// Sends a notification when CI fails.
    /// Includes task context so clicking the notification navigates to the task
    /// detail page.
    pub fn notify_ci_failure(&self, task_id: &str, task_title: &str) {
        self.native_service.show(TaskNotification {
            title: "CI Failure Detected".to_string(),
            body: format!("CI checks failed for '{}'", task_title),
            task_type: NotificationTaskType::UnitTask,
            task_id: task_id.to_string(),
        });
    }

    /// Sends a notification when review comments are received.
    /// Includes task context so clicking the notification navigates to the task
    /// detail page.
    pub fn notify_review_comments(&self, task_id: &str, task_title: &str, comment_count: usize) {
        self.native_service.show(TaskNotification {
            title: "Review Comments Received".to_string(),
            body: format!(
                "{} new review comment(s) on '{}'",
                comment_count, task_title
            ),
            task_type: NotificationTaskType::UnitTask,
            task_id: task_id.to_string(),
        });
    }
}
