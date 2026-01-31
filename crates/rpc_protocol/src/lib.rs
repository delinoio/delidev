//! JSON-RPC protocol definitions for DeliDev server/client communication
//!
//! This crate defines the JSON-RPC 2.0 protocol used for communication between
//! the DeliDev main server, worker servers, and clients.

mod error;
mod methods;
mod types;

pub use error::*;
pub use methods::*;
pub use types::*;
