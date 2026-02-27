//! Service-layer CryptoService
//!
//! Delegates to core::CryptoService, providing a clean service-layer interface
//! for workflow consumers.

use std::sync::Arc;

use crate::service::error::ServiceResult;
use crate::usecase::core::crypto::{
    CFragData, Capsule, CipherFragment, CryptoService as CoreCryptoService, KeyFragment, PublicKey,
    ReencryptionKey, SecretKey, ShamirShare,
};

/// Service-layer CryptoService trait
pub trait CryptoService: Send + Sync {
    fn split_secret_shamir(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> ServiceResult<Vec<ShamirShare>>;

    fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> ServiceResult<Vec<u8>>;

    fn create_pre_capsule(
        &self,
        public_key: &PublicKey,
        plaintext: &[u8],
    ) -> ServiceResult<(Capsule, Vec<u8>)>;

    fn generate_reencryption_key(
        &self,
        owner_secret_key: &SecretKey,
        accessor_public_key: &PublicKey,
    ) -> ServiceResult<ReencryptionKey>;

    fn create_kfrags(
        &self,
        reencryption_key: &ReencryptionKey,
        threshold: u8,
        total_fragments: u8,
    ) -> ServiceResult<Vec<KeyFragment>>;

    fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        capsule: &Capsule,
    ) -> ServiceResult<CipherFragment>;

    fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        original_capsule: &Capsule,
        ciphertext: &[u8],
    ) -> ServiceResult<Vec<u8>>;

    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)>;

    fn derive_public_key(&self, secret_key: &SecretKey) -> ServiceResult<PublicKey>;

    fn aes_gcm_encrypt(&self, key: &[u8], plaintext: &[u8]) -> ServiceResult<Vec<u8>>;

    fn aes_gcm_decrypt(&self, key: &[u8], ciphertext: &[u8]) -> ServiceResult<Vec<u8>>;

    fn generate_symmetric_key(&self) -> ServiceResult<Vec<u8>>;

    fn verifying_key_bytes(&self) -> ServiceResult<Vec<u8>>;

    fn decrypt_pre_capsule(
        &self,
        capsule: &Capsule,
        cfrags: &[CFragData],
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        ciphertext: &[u8],
        verifying_pk_bytes: &[u8],
    ) -> ServiceResult<Vec<u8>>;
}

/// Service-layer CryptoService implementation delegating to core
pub struct CryptoServiceImpl<C: CoreCryptoService> {
    inner: Arc<C>,
}

impl<C: CoreCryptoService> CryptoServiceImpl<C> {
    pub fn new(inner: Arc<C>) -> Self {
        Self { inner }
    }
}

impl<C: CoreCryptoService> CryptoService for CryptoServiceImpl<C> {
    fn split_secret_shamir(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> ServiceResult<Vec<ShamirShare>> {
        self.inner
            .split_secret_shamir(secret, threshold, total_shares)
    }

    fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> ServiceResult<Vec<u8>> {
        self.inner.reconstruct_secret_shamir(shares, threshold)
    }

    fn create_pre_capsule(
        &self,
        public_key: &PublicKey,
        plaintext: &[u8],
    ) -> ServiceResult<(Capsule, Vec<u8>)> {
        self.inner.create_pre_capsule(public_key, plaintext)
    }

    fn generate_reencryption_key(
        &self,
        owner_secret_key: &SecretKey,
        accessor_public_key: &PublicKey,
    ) -> ServiceResult<ReencryptionKey> {
        self.inner
            .generate_reencryption_key(owner_secret_key, accessor_public_key)
    }

    fn create_kfrags(
        &self,
        reencryption_key: &ReencryptionKey,
        threshold: u8,
        total_fragments: u8,
    ) -> ServiceResult<Vec<KeyFragment>> {
        self.inner
            .create_kfrags(reencryption_key, threshold, total_fragments)
    }

    fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        capsule: &Capsule,
    ) -> ServiceResult<CipherFragment> {
        self.inner.proxy_reencrypt(kfrag, capsule)
    }

    fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        original_capsule: &Capsule,
        ciphertext: &[u8],
    ) -> ServiceResult<Vec<u8>> {
        self.inner
            .combine_and_decrypt(cfrags, accessor_secret_key, original_capsule, ciphertext)
    }

    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)> {
        self.inner.generate_keypair()
    }

    fn derive_public_key(&self, secret_key: &SecretKey) -> ServiceResult<PublicKey> {
        self.inner.derive_public_key(secret_key)
    }

    fn aes_gcm_encrypt(&self, key: &[u8], plaintext: &[u8]) -> ServiceResult<Vec<u8>> {
        self.inner.aes_gcm_encrypt(key, plaintext)
    }

    fn aes_gcm_decrypt(&self, key: &[u8], ciphertext: &[u8]) -> ServiceResult<Vec<u8>> {
        self.inner.aes_gcm_decrypt(key, ciphertext)
    }

    fn generate_symmetric_key(&self) -> ServiceResult<Vec<u8>> {
        self.inner.generate_symmetric_key()
    }

    fn verifying_key_bytes(&self) -> ServiceResult<Vec<u8>> {
        self.inner.verifying_key_bytes()
    }

    fn decrypt_pre_capsule(
        &self,
        capsule: &Capsule,
        cfrags: &[CFragData],
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        ciphertext: &[u8],
        verifying_pk_bytes: &[u8],
    ) -> ServiceResult<Vec<u8>> {
        self.inner.decrypt_pre_capsule(
            capsule,
            cfrags,
            requester_secret_key,
            owner_public_key,
            ciphertext,
            verifying_pk_bytes,
        )
    }
}
