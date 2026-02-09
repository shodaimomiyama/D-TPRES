//! Mock AO client module
//!
//! Provides mock implementation of AOClient trait for testing
//! and development without actual AO Network connection.
//!
//! ## Module Structure
//!
//! - `client` - AOClient trait and MockAOClient implementation
//! - `message` - AO message types for client-side communication

mod client;
mod message;

pub use client::{AOClient, MockAOClient, MockConfig};
pub use message::{
    AOAttribute, AOEvent, AOMessageTags, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus,
    ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, QueryMsg, ValidateMessage,
    MAX_BINARY_SIZE, MAX_ID_LENGTH,
};
