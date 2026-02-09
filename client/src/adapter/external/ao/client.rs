//! AO Network client trait
//!
//! This module provides the AOClient trait for abstracting AO Network communication.

use async_trait::async_trait;

use super::message::{AOResponse, Binary, ExecuteMsg, QueryMsg};
use crate::adapter::errors::AOCommunicationError;

/// AOClient trait for AO Network communication
///
/// This trait abstracts the communication with AO Network processes,
/// enabling dependency injection and easy mocking for tests.
#[async_trait]
pub trait AOClient: Send + Sync {
    /// Execute a state-changing message to an AO Process
    ///
    /// # Arguments
    /// * `process_id` - The AO Process ID to send the message to
    /// * `msg` - The execute message to send
    ///
    /// # Returns
    /// * `Ok(AOResponse)` - The response from the process
    /// * `Err(AOCommunicationError)` - If the execution fails
    async fn execute(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError>;

    /// Query an AO Process (read-only, via dry_run)
    ///
    /// # Arguments
    /// * `process_id` - The AO Process ID to query
    /// * `msg` - The query message to send
    ///
    /// # Returns
    /// * `Ok(Binary)` - The query result as binary data
    /// * `Err(AOCommunicationError)` - If the query fails
    async fn query(&self, process_id: &str, msg: QueryMsg) -> Result<Binary, AOCommunicationError>;

    /// Dry-run execution (read-only state inspection)
    ///
    /// # Arguments
    /// * `process_id` - The AO Process ID to dry-run against
    /// * `msg` - The execute message to simulate
    ///
    /// # Returns
    /// * `Ok(AOResponse)` - The simulated response
    /// * `Err(AOCommunicationError)` - If the dry-run fails
    async fn dry_run(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError>;
}
