//! Secret Entity (Aggregate Root)
//!
//! Manages secret metadata and state transitions.
//! This is the aggregate root for the secret management domain.

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{CapsuleId, KFragId, SecretId, ShareCollectionId};

/// Secret state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretState {
    /// Initial state after creation
    Initialized,
    /// Secret has been split into shares
    Split,
    /// KFrags have been distributed to holders
    Distributed,
    /// Secret has been recovered by requester
    Recovered,
}

impl std::fmt::Display for SecretState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialized => write!(f, "Initialized"),
            Self::Split => write!(f, "Split"),
            Self::Distributed => write!(f, "Distributed"),
            Self::Recovered => write!(f, "Recovered"),
        }
    }
}

/// Secret entity (aggregate root)
///
/// Manages the lifecycle and metadata of a secret in the D-TPRES system.
/// Does not hold the actual secret data - that's handled by SecretData value object.
#[derive(Debug, Clone)]
pub struct Secret {
    id: SecretId,
    threshold_k: u8,
    threshold_n: u8,
    state: SecretState,
    capsule_id: Option<CapsuleId>,
    share_collection_id: Option<ShareCollectionId>,
    kfrag_ids: Vec<KFragId>,
    owner_public_key: Vec<u8>,
    requester_public_key: Option<Vec<u8>>,
    created_at: u64,
}

impl Secret {
    /// Create a new Secret entity
    ///
    /// # Arguments
    /// * `threshold_k` - Minimum number of shares required for recovery
    /// * `threshold_n` - Total number of shares to create
    /// * `owner_public_key` - Owner's PRE public key
    ///
    /// # Errors
    /// Returns `DomainError` if threshold parameters are invalid
    pub fn new(
        threshold_k: u8,
        threshold_n: u8,
        owner_public_key: Vec<u8>,
    ) -> Result<Self, DomainError> {
        // Validate threshold parameters
        if threshold_k == 0 {
            return Err(DomainError::EntityValidation {
                entity_type: "Secret".to_string(),
                field: "threshold_k".to_string(),
                message: "Threshold k must be greater than 0".to_string(),
            });
        }

        if threshold_k > threshold_n {
            return Err(DomainError::EntityValidation {
                entity_type: "Secret".to_string(),
                field: "threshold".to_string(),
                message: format!("Threshold k={} must be <= n={}", threshold_k, threshold_n),
            });
        }

        if owner_public_key.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "Secret".to_string(),
                field: "owner_public_key".to_string(),
                message: "Owner public key cannot be empty".to_string(),
            });
        }

        Ok(Self {
            id: SecretId::generate(),
            threshold_k,
            threshold_n,
            state: SecretState::Initialized,
            capsule_id: None,
            share_collection_id: None,
            kfrag_ids: Vec::new(),
            owner_public_key,
            requester_public_key: None,
            created_at: current_timestamp(),
        })
    }

    /// Reconstruct a Secret from stored data (for repository use)
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: SecretId,
        threshold_k: u8,
        threshold_n: u8,
        state: SecretState,
        capsule_id: Option<CapsuleId>,
        share_collection_id: Option<ShareCollectionId>,
        kfrag_ids: Vec<KFragId>,
        owner_public_key: Vec<u8>,
        requester_public_key: Option<Vec<u8>>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            threshold_k,
            threshold_n,
            state,
            capsule_id,
            share_collection_id,
            kfrag_ids,
            owner_public_key,
            requester_public_key,
            created_at,
        }
    }

    /// Get the secret ID
    pub fn id(&self) -> &SecretId {
        &self.id
    }

    /// Get threshold k (minimum shares required)
    pub fn threshold_k(&self) -> u8 {
        self.threshold_k
    }

    /// Get threshold n (total shares)
    pub fn threshold_n(&self) -> u8 {
        self.threshold_n
    }

    /// Get current state
    pub fn state(&self) -> SecretState {
        self.state
    }

    /// Get capsule ID if set
    pub fn capsule_id(&self) -> Option<&CapsuleId> {
        self.capsule_id.as_ref()
    }

    /// Get share collection ID if set
    pub fn share_collection_id(&self) -> Option<&ShareCollectionId> {
        self.share_collection_id.as_ref()
    }

    /// Get KFrag IDs
    pub fn kfrag_ids(&self) -> &[KFragId] {
        &self.kfrag_ids
    }

    /// Get owner's public key
    pub fn owner_public_key(&self) -> &[u8] {
        &self.owner_public_key
    }

    /// Get requester's public key if set
    pub fn requester_public_key(&self) -> Option<&[u8]> {
        self.requester_public_key.as_deref()
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Transition to Split state after shares are created
    ///
    /// # Arguments
    /// * `share_collection_id` - ID of the created ShareCollection
    /// * `capsule_id` - ID of the created Capsule
    ///
    /// # Errors
    /// Returns error if state transition is invalid
    pub fn split(
        &mut self,
        share_collection_id: ShareCollectionId,
        capsule_id: CapsuleId,
    ) -> Result<(), DomainError> {
        if self.state != SecretState::Initialized {
            return Err(DomainError::InvalidStateTransition {
                entity_type: "Secret".to_string(),
                from_state: self.state.to_string(),
                to_state: SecretState::Split.to_string(),
                reason: "Can only split from Initialized state".to_string(),
            });
        }

        self.share_collection_id = Some(share_collection_id);
        self.capsule_id = Some(capsule_id);
        self.state = SecretState::Split;
        Ok(())
    }

    /// Transition to Distributed state after KFrags are distributed
    ///
    /// # Arguments
    /// * `kfrag_ids` - IDs of the distributed KFrags
    ///
    /// # Errors
    /// Returns error if state transition is invalid or kfrag count is wrong
    pub fn distribute(&mut self, kfrag_ids: Vec<KFragId>) -> Result<(), DomainError> {
        if self.state != SecretState::Split {
            return Err(DomainError::InvalidStateTransition {
                entity_type: "Secret".to_string(),
                from_state: self.state.to_string(),
                to_state: SecretState::Distributed.to_string(),
                reason: "Can only distribute from Split state".to_string(),
            });
        }

        if kfrag_ids.len() != self.threshold_n as usize {
            return Err(DomainError::EntityValidation {
                entity_type: "Secret".to_string(),
                field: "kfrag_ids".to_string(),
                message: format!(
                    "Expected {} KFrags, got {}",
                    self.threshold_n,
                    kfrag_ids.len()
                ),
            });
        }

        self.kfrag_ids = kfrag_ids;
        self.state = SecretState::Distributed;
        Ok(())
    }

    /// Set requester's public key for re-encryption
    pub fn set_requester_public_key(&mut self, public_key: Vec<u8>) -> Result<(), DomainError> {
        if public_key.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "Secret".to_string(),
                field: "requester_public_key".to_string(),
                message: "Requester public key cannot be empty".to_string(),
            });
        }
        self.requester_public_key = Some(public_key);
        Ok(())
    }

    /// Mark as recovered (final state)
    pub fn mark_recovered(&mut self) -> Result<(), DomainError> {
        if self.state != SecretState::Distributed {
            return Err(DomainError::InvalidStateTransition {
                entity_type: "Secret".to_string(),
                from_state: self.state.to_string(),
                to_state: SecretState::Recovered.to_string(),
                reason: "Can only recover from Distributed state".to_string(),
            });
        }
        self.state = SecretState::Recovered;
        Ok(())
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_new_valid() {
        let pk = vec![1u8; 33];
        let secret = Secret::new(2, 3, pk).unwrap();

        assert_eq!(secret.threshold_k(), 2);
        assert_eq!(secret.threshold_n(), 3);
        assert_eq!(secret.state(), SecretState::Initialized);
        assert!(secret.capsule_id().is_none());
        assert!(secret.share_collection_id().is_none());
    }

    #[test]
    fn test_secret_new_invalid_threshold_zero() {
        let pk = vec![1u8; 33];
        let result = Secret::new(0, 3, pk);
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_new_invalid_threshold_k_greater_than_n() {
        let pk = vec![1u8; 33];
        let result = Secret::new(5, 3, pk);
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_new_empty_public_key() {
        let result = Secret::new(2, 3, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_split_transition() {
        let pk = vec![1u8; 33];
        let mut secret = Secret::new(2, 3, pk).unwrap();

        let collection_id = ShareCollectionId::generate();
        let capsule_id = CapsuleId::generate();

        secret
            .split(collection_id.clone(), capsule_id.clone())
            .unwrap();

        assert_eq!(secret.state(), SecretState::Split);
        assert_eq!(secret.share_collection_id(), Some(&collection_id));
        assert_eq!(secret.capsule_id(), Some(&capsule_id));
    }

    #[test]
    fn test_secret_invalid_split_from_wrong_state() {
        let pk = vec![1u8; 33];
        let mut secret = Secret::new(2, 3, pk).unwrap();

        // First split
        secret
            .split(ShareCollectionId::generate(), CapsuleId::generate())
            .unwrap();

        // Try to split again (should fail)
        let result = secret.split(ShareCollectionId::generate(), CapsuleId::generate());
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_distribute_transition() {
        let pk = vec![1u8; 33];
        let mut secret = Secret::new(2, 3, pk).unwrap();

        secret
            .split(ShareCollectionId::generate(), CapsuleId::generate())
            .unwrap();

        let kfrag_ids = vec![
            KFragId::generate(),
            KFragId::generate(),
            KFragId::generate(),
        ];

        secret.distribute(kfrag_ids.clone()).unwrap();

        assert_eq!(secret.state(), SecretState::Distributed);
        assert_eq!(secret.kfrag_ids().len(), 3);
    }

    #[test]
    fn test_secret_distribute_wrong_kfrag_count() {
        let pk = vec![1u8; 33];
        let mut secret = Secret::new(2, 3, pk).unwrap();

        secret
            .split(ShareCollectionId::generate(), CapsuleId::generate())
            .unwrap();

        // Try to distribute with wrong number of kfrags
        let kfrag_ids = vec![KFragId::generate(), KFragId::generate()]; // Only 2, need 3
        let result = secret.distribute(kfrag_ids);
        assert!(result.is_err());
    }
}
