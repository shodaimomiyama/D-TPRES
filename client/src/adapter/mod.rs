//! Adapter layer module
//!
//! Provides adapter layer components including error definitions
//! and repository implementations for external system integration.

pub mod errors;
pub mod external;
pub mod repository_impl;

pub use errors::{AdapterError, AdapterResult};
pub use external::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};
pub use repository_impl::{ArweaveClient, Tag};
