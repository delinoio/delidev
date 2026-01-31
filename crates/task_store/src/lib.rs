//! Task storage abstraction for DeliDev.
//!
//! This crate provides a trait-based abstraction for task storage,
//! with implementations for SQLite (single-user mode), PostgreSQL
//! (multi-user mode), and in-memory (testing).

mod error;
mod memory;
mod traits;

pub use error::*;
pub use memory::*;
pub use traits::*;
