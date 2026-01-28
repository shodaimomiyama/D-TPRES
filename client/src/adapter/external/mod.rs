//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave network communication.

pub mod arweave;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};

pub mod ao_client;
pub mod ao_message;

pub use ao_client::{AOClient, MockAOClient, MockConfig};
pub use ao_message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus,
    ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
    QueryMsg, ValidateMessage,
};
