//! Legacy CWAO AO Network client module (preserved for reference)
//!
//! The production client and data_item modules have been superseded by
//! the HyperBEAM-native implementation in `ao/`. The trait, config,
//! and message types remain for CWAO compatibility.

mod client;
mod config;
mod message;

pub use client::AOClient;
pub use config::AOConfig;
pub use message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CFragEntry, CapsuleInfo,
    CapsuleStatus, ExecuteMsg, GetCFragResponse, GetCFragsBySecretResponse,
    ListCapsulesByKFragResponse, QueryMsg, ValidateMessage, MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
