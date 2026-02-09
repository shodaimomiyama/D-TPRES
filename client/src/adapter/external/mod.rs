//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave network communication and AO Network mock client.

pub mod arweave;
pub mod mock_ao;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};
pub use mock_ao::{
    AOAttribute, AOClient, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo,
    CapsuleStatus, ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, MockAOClient,
    MockConfig, QueryMsg, ValidateMessage, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
