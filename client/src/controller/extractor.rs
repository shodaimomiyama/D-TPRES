//! Controller layer extractors for DTO construction
//!
//! Converts validated raw parameters into UseCase layer DTOs.

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{PublicKey, SecretKey};
use crate::usecase::dto::{SecretMetadata, SecretRecoveryRequest, SecretSharingRequest};

/// Extractor for SecretSharingRequest DTO
///
/// Converts validated parameters into SecretSharingRequest.
/// Does not perform validation - that is the Validator's responsibility.
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
        secret: Vec<u8>,
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
        requester_process_id: String,
    ) -> SecretRecoveryRequest {
        SecretRecoveryRequest {
            secret_id,
            requester_secret_key,
            requester_process_id,
        }
    }
}

impl Default for RecoverExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

    fn create_test_keys() -> (SecretKey, PublicKey) {
        let crypto_service = CryptoServiceImpl::new();
        crypto_service
            .generate_keypair()
            .expect("Failed to generate test keys")
    }

    // ========================================================================
    // ShareExtractor Tests
    // ========================================================================

    #[test]
    fn test_share_extractor_creates_request() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let request = extractor.extract(
            b"secret data".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner_process_123".to_string(),
            None,
        );

        assert_eq!(request.threshold, 3);
        assert_eq!(request.total_shares, 5);
        assert_eq!(request.owner_process_id, "owner_process_123");
    }

    #[test]
    fn test_share_extractor_secret_preserved() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();
        let secret = b"my secret data".to_vec();

        let request = extractor.extract(
            secret.clone(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner_123".to_string(),
            None,
        );

        assert_eq!(request.secret, secret);
    }

    #[test]
    fn test_share_extractor_threshold_params() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let request = extractor.extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            7,
            10,
            "owner".to_string(),
            None,
        );

        assert_eq!(request.threshold, 7);
        assert_eq!(request.total_shares, 10);
    }

    #[test]
    fn test_share_extractor_owner_keys() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();
        let owner_pk_bytes = owner_pk.key_data.clone();

        let request = extractor.extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner".to_string(),
            None,
        );

        assert!(!request.owner_secret_key.is_empty());
        assert_eq!(request.owner_public_key.key_data, owner_pk_bytes);
    }

    #[test]
    fn test_share_extractor_requester_key() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();
        let requester_pk_bytes = requester_pk.key_data.clone();

        let request = extractor.extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner".to_string(),
            None,
        );

        assert_eq!(request.requester_public_key.key_data, requester_pk_bytes);
    }

    #[test]
    fn test_share_extractor_with_metadata() {
        let extractor = ShareExtractor::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let metadata = SecretMetadata {
            name: Some("Test Secret".to_string()),
            description: Some("A test secret for unit testing".to_string()),
            expires_at: Some(1735689600),
            tags: vec!["test".to_string(), "unit".to_string()],
        };

        let request = extractor.extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner".to_string(),
            Some(metadata),
        );

        assert!(request.metadata.is_some());
        let meta = request.metadata.as_ref().unwrap();
        assert_eq!(meta.name, Some("Test Secret".to_string()));
        assert_eq!(meta.tags.len(), 2);
    }

    #[test]
    fn test_share_extractor_default() {
        let extractor: ShareExtractor = Default::default();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let request = extractor.extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner".to_string(),
            None,
        );

        assert_eq!(request.threshold, 3);
    }

    // ========================================================================
    // RecoverExtractor Tests
    // ========================================================================

    #[test]
    fn test_recover_extractor_creates_request() {
        let extractor = RecoverExtractor::new();
        let (requester_sk, _) = create_test_keys();
        let secret_id = SecretId::new("secret_abc123");

        let request = extractor.extract(
            secret_id.clone(),
            requester_sk,
            "requester_process_456".to_string(),
        );

        assert_eq!(request.secret_id.as_str(), "secret_abc123");
        assert_eq!(request.requester_process_id, "requester_process_456");
    }

    #[test]
    fn test_recover_extractor_secret_id() {
        let extractor = RecoverExtractor::new();
        let (requester_sk, _) = create_test_keys();
        let secret_id = SecretId::new("my_secret_id");

        let request = extractor.extract(secret_id, requester_sk, "process".to_string());

        assert_eq!(request.secret_id.as_str(), "my_secret_id");
    }

    #[test]
    fn test_recover_extractor_requester_key() {
        let extractor = RecoverExtractor::new();
        let (requester_sk, _) = create_test_keys();
        let secret_id = SecretId::new("secret");

        let request = extractor.extract(secret_id, requester_sk, "process".to_string());

        assert!(!request.requester_secret_key.is_empty());
    }

    #[test]
    fn test_recover_extractor_process_id() {
        let extractor = RecoverExtractor::new();
        let (requester_sk, _) = create_test_keys();
        let secret_id = SecretId::new("secret");

        let request = extractor.extract(secret_id, requester_sk, "my_process_id".to_string());

        assert_eq!(request.requester_process_id, "my_process_id");
    }

    #[test]
    fn test_recover_extractor_default() {
        let extractor: RecoverExtractor = Default::default();
        let (requester_sk, _) = create_test_keys();
        let secret_id = SecretId::new("secret");

        let request = extractor.extract(secret_id, requester_sk, "process".to_string());

        assert_eq!(request.requester_process_id, "process");
    }
}
