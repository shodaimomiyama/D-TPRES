//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave network communication.

pub mod arweave;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};

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
