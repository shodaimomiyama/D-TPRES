//! Adapter layer module
//!
//! Provides adapter layer components including error definitions
//! and repository implementations for external system integration.

pub mod errors;
pub mod repository_impl;

pub use errors::{AdapterError, AdapterResult};
pub use repository_impl::{ArweaveClient, Tag};
