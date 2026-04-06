//! AO Network client trait (HyperBEAM-native)

use async_trait::async_trait;

use super::message::{AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary};
use crate::adapter::errors::AOCommunicationError;

/// AOClient trait for HyperBEAM AO Network communication.
///
/// Replaces `ao_cwao/client.rs` which used CosmWasm-style ExecuteMsg/QueryMsg.
#[async_trait]
pub trait AOClient: Send + Sync {
    /// Send a state-changing message to an AO Process.
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError>;

    /// Query an AO Process (read-only, via dry_run).
    async fn query(
        &self,
        process_id: &str,
        msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError>;

    /// Dry-run execution (state inspection without side-effects).
    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError>;
}
