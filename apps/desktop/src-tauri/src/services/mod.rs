mod app_state;
mod composite_planning;
mod concurrency;
mod custom_command;
mod native_notification;
mod notification;
mod repository;
mod repository_group;
mod task;
mod vcs;
mod workspace;

// Desktop-only modules (require Docker, Git, file watching)
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod agent_execution;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod config_watcher;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod docker;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod git;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod update;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod worktree_cleanup;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use agent_execution::*;
pub use app_state::*;
pub use composite_planning::*;
pub use concurrency::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use config_watcher::*;
pub use custom_command::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use docker::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use git::*;
pub use native_notification::*;
pub use notification::*;
pub use repository::*;
pub use repository_group::*;
pub use task::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use update::*;
pub use vcs::*;
pub use workspace::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use worktree_cleanup::*;
