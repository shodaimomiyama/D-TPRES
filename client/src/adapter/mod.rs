//! Adapter layer module
//!
//! Provides adapter layer components including error definitions,
//! repository implementations, and external system adapters.

pub mod errors;
pub mod external;
pub mod repository_impl;

pub use errors::{AOCommunicationError, AOResult, AdapterError, AdapterResult};
pub use external::{
    AOAttribute, AOClient, AOEvent, AOMessageTags, AOResponse, Binary, ExecuteMsg, MockAOClient,
    MockConfig, QueryMsg, ValidateMessage,
};
pub use external::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};
pub use repository_impl::{ArweaveClient, Tag};
