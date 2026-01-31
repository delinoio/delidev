//! Cross-platform keychain access for DeliDev.
//!
//! This crate provides:
//! - Trait-based keychain abstraction
//! - Native keychain implementations (macOS Keychain, Windows Credential
//!   Manager, Linux Secret Service)
//! - Known secret key management

mod error;
mod keychain;
mod keys;

pub use error::*;
pub use keychain::*;
pub use keys::*;
