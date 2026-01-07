//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave network communication.

pub mod arweave;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};
