//! External system adapters
//!
//! This module provides adapters for external system communication,
//! primarily the AO Network client for communicating with AO processes.

pub mod ao_client;
pub mod ao_message;

pub use ao_client::{AOClient, MockAOClient, MockConfig};
pub use ao_message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus,
    ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, MAX_BINARY_SIZE, MAX_ID_LENGTH,
    QueryMsg, ValidateMessage,
};
