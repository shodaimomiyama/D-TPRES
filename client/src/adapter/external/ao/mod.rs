//! AO Network client module
//!
//! Provides production implementation of AOClient trait
//! for AO Network communication.
//!
//! ## Module Structure
//!
//! - `client` - AOClient trait definition
//! - `config` - AO Network connection configuration
//! - `message` - Message types for AO communication
//! - `data_item` - ANS-104 DataItem builder and signer (production feature)
//! - `production_client` - Production AOClient implementation (production feature)

mod client;
mod config;
#[cfg(feature = "production-ao")]
mod data_item;
mod message;
#[cfg(feature = "production-ao")]
mod production_client;

pub use client::AOClient;
pub use config::AOConfig;
#[cfg(feature = "production-ao")]
pub use data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
pub use message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus,
    ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
    QueryMsg, ValidateMessage,
};
#[cfg(feature = "production-ao")]
pub use production_client::ProductionAOClient;
