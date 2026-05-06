//! AO Network client module (HyperBEAM-native)
//!
//! This module targets HyperBEAM (~wasm64@1.0) with AO-native message format.
//!
//! ## Module Structure
//! - `client` - AOClient trait
//! - `config` - AO Network connection config (HyperBEAM endpoints)
//! - `message` - AO-native message types (AOExecuteMsg, AOQueryMsg, AONativeResponse)
//! - `wallet` - Arweave JWK wallet loader (hyperbeam feature)
//! - `signer` - RFC-9421 HTTP message signatures (hyperbeam feature)
//! - `tabm` - TABM multipart encoder (hyperbeam feature)
//! - `hyperbeam_client` - HyperBEAMClient AOClient impl (hyperbeam feature)

mod client;
mod config;
mod message;
#[cfg(feature = "hyperbeam")]
pub mod signer;
#[cfg(feature = "hyperbeam")]
pub mod tabm;
#[cfg(feature = "hyperbeam")]
pub mod wallet;
#[cfg(feature = "hyperbeam")]
mod hyperbeam_client;

pub use client::AOClient;
pub use config::AOConfig;
pub use message::{
    validate_binary, validate_id, AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary,
    GetCFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
#[cfg(feature = "hyperbeam")]
pub use hyperbeam_client::HyperBEAMClient;
