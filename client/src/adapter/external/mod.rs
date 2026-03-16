//! External system adapters module
//!
//! Provides adapters for external system integrations including
//! Arweave and AO network communication.
//!
//! ## AO modules
//! - `ao/`       — HyperBEAM-native AO client (current)
//! - `ao_cwao/`  — Legacy CosmWasm AO client (preserved for reference)

pub mod ao;
pub mod ao_cwao;
pub mod arweave;
pub mod mock_ao;

pub use arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};

// Re-export HyperBEAM-native AO types (new default)
pub use ao::{
    AOClient, AOConfig, AOExecuteMsg, AONativeResponse, AOQueryMsg,
    Binary, GetCFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
#[cfg(feature = "production-ao")]
pub use ao::{ArweaveJWK, DataItemBuilder, DataItemSigner, ProductionAOClient};

pub use mock_ao::{MockAOClient, MockConfig};
