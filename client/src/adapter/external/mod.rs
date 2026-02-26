//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave and AO network communication.

pub mod ao;
pub mod arweave;
pub mod mock_ao;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};

pub use ao::{
    AOAttribute, AOClient, AOConfig, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta,
    CapsuleInfo, CapsuleStatus, ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse,
    QueryMsg, ValidateMessage, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
#[cfg(feature = "production-ao")]
pub use ao::{ArweaveJWK, DataItemBuilder, DataItemSigner, ProductionAOClient};

pub use mock_ao::{MockAOClient, MockConfig};
