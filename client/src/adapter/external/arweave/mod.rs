//! Arweave client module
//!
//! Provides production implementation of ArweaveClient trait
//! for Arweave network communication.
//!
//! ## Module Structure
//!
//! - `client` - HTTP client for Arweave gateway communication
//! - `config` - Client configuration
//! - `wallet` - Wallet management and RSA-PSS signing
//! - `deep_hash` - Arweave DeepHash algorithm (SHA-384)
//! - `merkle` - Merkle tree for data_root calculation
//! - `transaction` - Transaction types and building logic

mod client;
mod config;
mod deep_hash;
mod merkle;
mod transaction;
mod wallet;

pub use client::ArweaveClientImpl;
pub use config::ArweaveClientConfig;
pub use wallet::ArweaveWallet;
