//! External system adapters
//!
//! This module provides adapters for external system communication,
//! primarily the AO Network client for communicating with AO processes.

pub mod ao_client;
pub mod ao_config;
pub mod ao_message;
#[cfg(feature = "production-ao")]
pub mod data_item;
#[cfg(feature = "production-ao")]
pub mod production_ao_client;

pub use ao_client::{AOClient, MockAOClient, MockConfig};
pub use ao_config::AOConfig;
pub use ao_message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus,
    ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
    QueryMsg, ValidateMessage,
};
#[cfg(feature = "production-ao")]
pub use data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
#[cfg(feature = "production-ao")]
pub use production_ao_client::ProductionAOClient;
