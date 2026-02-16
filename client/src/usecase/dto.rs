//! Data Transfer Objects for Workflow Services
//!
//! Defines request and result types for SecretSharingWorkflowService
//! and SecretRecoveryWorkflowService.

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{PublicKey, SecretKey};

// ============================================================================
// SecretSharingWorkflowService DTOs
// ============================================================================

/// Phase 1 execution request for secret sharing
///
/// Contains all parameters needed to split and distribute a secret.
/// Sensitive data (secret, owner_secret_key) is zeroized on drop.
#[derive(Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct SecretSharingRequest {
    /// Secret data to be split (will be zeroized on drop)
    pub secret: Vec<u8>,
    /// Owner's secret key for PRE (generated via CryptoService::generate_keypair)
    pub owner_secret_key: SecretKey,
    /// Owner's public key for PRE
    pub owner_public_key: PublicKey,
    /// Requester's public key for PRE (who will be able to recover)
    pub requester_public_key: PublicKey,
    /// Threshold k (minimum shares needed for reconstruction)
    pub threshold: u8,
    /// Total shares n (total number of shares to generate)
    pub total_shares: u8,
    /// Owner-Process ID for AO communication (kFrag delivery destination)
    pub owner_process_id: String,
    /// Optional metadata for the secret
    pub metadata: Option<SecretMetadata>,
}

impl Drop for SecretSharingRequest {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

/// Optional metadata for a secret
#[derive(Debug, Clone, Default)]
#[allow(clippy::exhaustive_structs)]
pub struct SecretMetadata {
    /// Human-readable name for the secret
    pub name: Option<String>,
    /// Description of the secret
    pub description: Option<String>,
    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: Option<u64>,
    /// Custom tags for categorization
    pub tags: Vec<String>,
}

/// Phase 1 execution result
///
/// Contains identifiers and transaction IDs for the created secret components.
#[derive(Debug, Clone)]
#[allow(clippy::exhaustive_structs)]
pub struct SecretSharingResult {
    /// Generated secret ID (used for recovery)
    pub secret_id: SecretId,
    /// Capsule's Arweave transaction ID
    pub capsule_tx_id: String,
    /// Encrypted shares' transaction IDs
    pub share_tx_ids: Vec<String>,
    /// Number of kFrags generated and sent
    pub kfrag_count: u8,
    /// Owner's public key (to share with requester)
    pub owner_public_key: PublicKey,
}

// ============================================================================
// SecretRecoveryWorkflowService DTOs
// ============================================================================

/// Phase 3 execution request for secret recovery
///
/// cFrags and Capsule are fetched internally by the WorkflowService from
/// AO Network and Arweave. Client only provides secret_id and requester credentials.
#[derive(Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct SecretRecoveryRequest {
    /// Secret ID to recover
    pub secret_id: SecretId,
    /// Requester's secret key for PRE decryption
    pub requester_secret_key: SecretKey,
    /// Requester-Process ID for AO communication (cFrag retrieval source)
    pub requester_process_id: String,
}

/// Phase 3 execution result
///
/// Contains the recovered secret data. Automatically zeroized on drop.
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
#[allow(clippy::exhaustive_structs)]
pub struct SecretRecoveryResult {
    /// Recovered secret data (zeroized on drop)
    pub recovered_secret: Vec<u8>,
    /// Audit trail transaction ID on Arweave
    #[zeroize(skip)]
    pub audit_tx_id: String,
}

// ============================================================================
// SecretStatus Enum
// ============================================================================

/// Status of a secret in the FORMIX system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecretStatus {
    /// Secret has been created (Phase 1 completed)
    Created,
    /// kFrags have been distributed to holders (Phase 2 in progress)
    KFragsDistributed,
    /// Secret has been recovered at least once
    Recovered,
    /// Secret has been revoked and can no longer be recovered
    Revoked,
}

impl SecretStatus {
    /// Check if the secret can be recovered
    pub const fn can_recover(&self) -> bool {
        matches!(
            self,
            Self::Created | Self::KFragsDistributed | Self::Recovered
        )
    }

    /// Check if the secret is active (not revoked)
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::Revoked)
    }
}

impl std::fmt::Display for SecretStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::KFragsDistributed => write!(f, "kfrags_distributed"),
            Self::Recovered => write!(f, "recovered"),
            Self::Revoked => write!(f, "revoked"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_status_can_recover() {
        assert!(SecretStatus::Created.can_recover());
        assert!(SecretStatus::KFragsDistributed.can_recover());
        assert!(SecretStatus::Recovered.can_recover());
        assert!(!SecretStatus::Revoked.can_recover());
    }

    #[test]
    fn test_secret_status_is_active() {
        assert!(SecretStatus::Created.is_active());
        assert!(SecretStatus::KFragsDistributed.is_active());
        assert!(SecretStatus::Recovered.is_active());
        assert!(!SecretStatus::Revoked.is_active());
    }

    #[test]
    fn test_secret_status_display() {
        assert_eq!(format!("{}", SecretStatus::Created), "created");
        assert_eq!(
            format!("{}", SecretStatus::KFragsDistributed),
            "kfrags_distributed"
        );
        assert_eq!(format!("{}", SecretStatus::Recovered), "recovered");
        assert_eq!(format!("{}", SecretStatus::Revoked), "revoked");
    }

    #[test]
    fn test_secret_metadata_default() {
        let metadata = SecretMetadata::default();
        assert!(metadata.name.is_none());
        assert!(metadata.description.is_none());
        assert!(metadata.expires_at.is_none());
        assert!(metadata.tags.is_empty());
    }
}
