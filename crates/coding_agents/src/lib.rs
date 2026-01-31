//! AI coding agent abstraction and Docker sandboxing
//!
//! This crate provides a unified interface for AI coding agents like Claude
//! Code, OpenCode, Aider, and others. It also provides Docker sandboxing
//! capabilities for isolated execution.
//!
//! On mobile platforms (iOS and Android), only the type definitions are
//! available. The agent execution and Docker sandboxing functionality
//! requires a desktop platform.

mod output;

// Agent execution and Docker sandboxing are only available on desktop platforms
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod agent;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod docker;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod stream;

// Types that are available on all platforms
mod types;

// Re-export types that are always available
pub use output::*;
pub use types::*;

// Re-export execution-related types only on desktop platforms
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use agent::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use docker::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use stream::*;
