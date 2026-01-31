//! AI coding agent abstraction and Docker sandboxing
//!
//! This crate provides a unified interface for AI coding agents like Claude Code,
//! OpenCode, Aider, and others. It also provides Docker sandboxing capabilities
//! for isolated execution.

mod agent;
mod docker;
mod output;
mod stream;

pub use agent::*;
pub use docker::*;
pub use output::*;
pub use stream::*;
