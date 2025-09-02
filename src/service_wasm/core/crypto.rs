//! WASM-compatible mock crypto service
//! 
//! This provides mock implementations for cryptographic operations
//! that would normally be performed off-chain in the browser.

use serde::{Deserialize, Serialize};
use crate::service::error::{ServiceError, ServiceResult};

/// Mock structures for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKey {
    pub key_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKey {
    pub key_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShamirShare {
    pub index: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReencryptionKey {
    pub delegating_sk_data: Vec<u8>,
    pub receiving_pk_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CipherFragment {
    pub fragment_id: u8,
    pub capsule_fragment: Vec<u8>,
    pub proof: Vec<u8>,
}

/// Mock CryptoService trait for WASM
pub trait CryptoService: Send + Sync {
    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)>;
    fn split_secret_shamir(&self, secret: &[u8], threshold: u8, total_shares: u8) -> ServiceResult<Vec<ShamirShare>>;
    fn create_pre_capsule(&self, public_key: &PublicKey, plaintext: &[u8]) -> ServiceResult<(Capsule, Vec<u8>)>;
    fn generate_reencryption_key(&self, owner_secret_key: &SecretKey, accessor_public_key: &PublicKey) -> ServiceResult<ReencryptionKey>;
    fn create_kfrags(&self, reencryption_key: &ReencryptionKey, threshold: u8, total_fragments: u8) -> ServiceResult<Vec<KeyFragment>>;
}

/// Mock implementation that returns placeholder data
pub struct CryptoServiceImpl;

impl CryptoServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl CryptoService for CryptoServiceImpl {
    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)> {
        // Return mock keypair for WASM
        // Actual crypto happens in O-Browser
        Ok((
            SecretKey { key_data: vec![1; 32] },
            PublicKey { key_data: vec![2; 33] },
        ))
    }
    
    fn split_secret_shamir(&self, _secret: &[u8], _threshold: u8, total_shares: u8) -> ServiceResult<Vec<ShamirShare>> {
        // Return mock shares
        let mut shares = Vec::new();
        for i in 0..total_shares {
            shares.push(ShamirShare {
                index: i + 1,
                data: vec![i; 64],
            });
        }
        Ok(shares)
    }
    
    fn create_pre_capsule(&self, _public_key: &PublicKey, _plaintext: &[u8]) -> ServiceResult<(Capsule, Vec<u8>)> {
        // Return mock capsule and ciphertext
        Ok((
            Capsule { data: vec![3; 100] },
            vec![4; 128],
        ))
    }
    
    fn generate_reencryption_key(&self, owner_secret_key: &SecretKey, accessor_public_key: &PublicKey) -> ServiceResult<ReencryptionKey> {
        // Return mock reencryption key
        Ok(ReencryptionKey {
            delegating_sk_data: owner_secret_key.key_data.clone(),
            receiving_pk_data: accessor_public_key.key_data.clone(),
        })
    }
    
    fn create_kfrags(&self, _reencryption_key: &ReencryptionKey, _threshold: u8, total_fragments: u8) -> ServiceResult<Vec<KeyFragment>> {
        // Return mock kfrags
        let mut kfrags = Vec::new();
        for i in 0..total_fragments {
            kfrags.push(KeyFragment {
                id: i,
                key_data: vec![5 + i; 50],
                verification_data: vec![6 + i; 30],
                precursor: vec![],
            });
        }
        Ok(kfrags)
    }
}

impl Default for CryptoServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}