//! Controller layer extractors for DTO construction
//!
//! Converts validated raw parameters into UseCase layer DTOs.

use zeroize::Zeroizing;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{PublicKey, SecretKey};

use crate::usecase::dto::{SecretMetadata, SecretRecoveryRequest, SecretSharingRequest};

/// Extractor for SecretSharingRequest DTO
///
/// Converts validated parameters into SecretSharingRequest.
/// Does not perform validation - that is the Validator's responsibility.
#[non_exhaustive]
pub struct ShareExtractor;

impl ShareExtractor {
    /// Create a new ShareExtractor
    pub fn new() -> Self {
        Self
    }

    /// Extract SecretSharingRequest DTO from validated parameters
    ///
    /// # Arguments
    /// * `secret` - Secret data to be split
    /// * `owner_secret_key` - Owner's secret key
    /// * `owner_public_key` - Owner's public key
    /// * `requester_public_key` - Requester's public key
    /// * `threshold` - Minimum shares required for reconstruction (k)
    /// * `total_shares` - Total number of shares to generate (n)
    /// * `owner_process_id` - Owner-Process ID for AO communication
    /// * `metadata` - Optional secret metadata
    ///
    /// # Returns
    /// * `SecretSharingRequest` - The constructed DTO
    #[allow(clippy::too_many_arguments)]
    pub fn extract(
        &self,
        secret: Zeroizing<Vec<u8>>,
        owner_secret_key: SecretKey,
        owner_public_key: PublicKey,
        requester_public_key: PublicKey,
        threshold: u8,
        total_shares: u8,
        owner_process_id: String,
        metadata: Option<SecretMetadata>,
    ) -> SecretSharingRequest {
        SecretSharingRequest {
            secret,
            owner_secret_key,
            owner_public_key,
            requester_public_key,
            threshold,
            total_shares,
            owner_process_id,
            metadata,
        }
    }
}

impl Default for ShareExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extractor for SecretRecoveryRequest DTO
///
/// Converts validated parameters into SecretRecoveryRequest.
/// Does not perform validation - that is the Validator's responsibility.
#[non_exhaustive]
pub struct RecoverExtractor;

impl RecoverExtractor {
    /// Create a new RecoverExtractor
    pub fn new() -> Self {
        Self
    }

    /// Extract SecretRecoveryRequest DTO from validated parameters
    ///
    /// # Arguments
    /// * `secret_id` - ID of the secret to recover
    /// * `requester_secret_key` - Requester's secret key
    /// * `requester_process_id` - Requester-Process ID
    ///
    /// # Returns
    /// * `SecretRecoveryRequest` - The constructed DTO
    pub fn extract(
        &self,
        secret_id: SecretId,
        requester_secret_key: SecretKey,
        owner_public_key: PublicKey,
        requester_process_id: String,
    ) -> SecretRecoveryRequest {
        SecretRecoveryRequest {
            secret_id,
            requester_secret_key,
            owner_public_key,
            requester_process_id,
        }
    }
}

impl Default for RecoverExtractor {
    fn default() -> Self {
        Self::new()
    }
}
