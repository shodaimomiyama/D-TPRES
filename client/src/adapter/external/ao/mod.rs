//! AO Network client module (HyperBEAM-native)
//!
//! Replaces `ao_cwao/` which used CosmWasm-style messages on the legacy AO testnet.
//! This module targets HyperBEAM (~wasm64@1.0) with AO-native message format.
//!
//! ## Module Structure
//! - `client` - AOClient trait (updated for HyperBEAM message types)
//! - `config` - AO Network connection config (HyperBEAM endpoints)
//! - `message` - AO-native message types (AOExecuteMsg, AOQueryMsg, AONativeResponse)
//! - `data_item` - ANS-104 DataItem builder and signer (production feature)
//! - `production_client` - ProductionAOClient impl (production feature)

mod client;
mod config;
#[cfg(feature = "production-ao")]
mod data_item;
mod message;
#[cfg(feature = "production-ao")]
mod production_client;
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
#[cfg(feature = "production-ao")]
pub use data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
pub use message::{
    validate_binary, validate_id, AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary,
    GetCFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
#[cfg(feature = "production-ao")]
pub use production_client::ProductionAOClient;
#[cfg(feature = "hyperbeam")]
pub use wallet::ArweaveJWK as HyperbeamArweaveJWK;
#[cfg(feature = "hyperbeam")]
pub use hyperbeam_client::HyperBEAMClient;
