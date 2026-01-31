//! Task storage and management for DeliDev
//!
//! This crate provides a storage abstraction for tasks, sessions, and
//! repositories. It supports both SQLite (for single-user/single-process mode)
//! and PostgreSQL (for multi-user mode).

mod entities;
mod error;
mod store;

pub use entities::*;
pub use error::*;
pub use store::*;
