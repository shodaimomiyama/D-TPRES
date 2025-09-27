//! WASM Bindings for D-TPRES Local Crypto Operations
//!
//! This module provides JavaScript/TypeScript bindings for local cryptographic
//! operations in D-TPRES, allowing kFrag generation in browser environments.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use crate::crypto_core::{CryptoService, CryptoServiceImpl};

// When the `console_error_panic_hook` feature is enabled, we can call the
// `set_panic_hook` function at least once during initialization, and then
// we will get better error messages if our code ever panics.
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Serializable structure for kFrag data that can be passed to JavaScript
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableKFrag {
    pub id: u32,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// Serializable structure for encryption setup results
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalEncryptionResult {
    pub kfrags: Vec<SerializableKFrag>,
    pub shares_count: usize,
    pub threshold: u8,
    pub capsule_id: String,
    pub success: bool,
}

/// Main WASM-bound class for local crypto operations
#[wasm_bindgen]
pub struct LocalCrypto {
    crypto_service: CryptoServiceImpl,
}

#[wasm_bindgen]
impl LocalCrypto {
    /// Create a new LocalCrypto instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        set_panic_hook();
        Self {
            crypto_service: CryptoServiceImpl::new(),
        }
    }

    /// Initialize the crypto service (call this first)
    #[wasm_bindgen]
    pub fn initialize(&self) -> Result<(), JsError> {
        // Any initialization logic if needed
        Ok(())
    }

    /// Generate kFrags locally in the browser/Node.js environment
    /// Returns a JSON string containing the results
    #[wasm_bindgen]
    pub fn generate_kfrags(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> Result<String, JsError> {
        // Input validation
        if secret.is_empty() {
            return Err(JsError::new("Secret cannot be empty"));
        }
        if threshold == 0 || total_shares == 0 {
            return Err(JsError::new("Threshold and total_shares must be greater than 0"));
        }
        if threshold > total_shares {
            return Err(JsError::new("Threshold cannot be greater than total_shares"));
        }

        // Generate owner keypair
        let (owner_sk, owner_pk) = self.crypto_service
            .generate_keypair()
            .map_err(|e| JsError::new(&format!("Failed to generate keypair: {:?}", e)))?;

        // Generate re-encryption key (simplified for MVP)
        let re_key = self.crypto_service
            .generate_reencryption_key(&owner_sk, &owner_pk)
            .map_err(|e| JsError::new(&format!("Failed to generate re-encryption key: {:?}", e)))?;

        // Generate kFrags
        let kfrags = self.crypto_service
            .create_kfrags(&re_key, threshold, total_shares)
            .map_err(|e| JsError::new(&format!("Failed to create kFrags: {:?}", e)))?;

        // Convert to serializable format
        let serializable_kfrags: Vec<SerializableKFrag> = kfrags
            .into_iter()
            .map(|kf| SerializableKFrag {
                id: kf.id as u32,
                key_data: kf.key_data.clone(),
                verification_data: kf.verification_data.clone(),
                precursor: kf.precursor.clone(),
            })
            .collect();

        // Create result structure
        let result = LocalEncryptionResult {
            kfrags: serializable_kfrags,
            shares_count: total_shares as usize,
            threshold,
            capsule_id: format!("capsule_{}", js_sys::Date::now() as u64),
            success: true,
        };

        // Serialize to JSON
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Failed to serialize result: {:?}", e)))
    }

    /// Generate a random secret for testing purposes
    #[wasm_bindgen]
    pub fn generate_test_secret(&self, length: usize) -> Result<Vec<u8>, JsError> {
        if length == 0 || length > 1024 {
            return Err(JsError::new("Secret length must be between 1 and 1024 bytes"));
        }

        // Generate random bytes using rand crate
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; length];
        rng.fill_bytes(&mut bytes);
        Ok(bytes)
    }

    /// Get version information
    #[wasm_bindgen]
    pub fn version(&self) -> String {
        "D-TPRES LocalCrypto v0.1.0-mvp".to_string()
    }

    /// Validate a secret before using it for kFrag generation
    #[wasm_bindgen]
    pub fn validate_secret(&self, secret: &[u8]) -> Result<bool, JsError> {
        if secret.is_empty() {
            return Ok(false);
        }
        if secret.len() < 16 {
            return Ok(false);
        }
        if secret.len() > 1024 {
            return Ok(false);
        }
        Ok(true)
    }

    /// Generate Shamir secret shares
    #[wasm_bindgen]
    pub fn generate_shamir_shares(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> Result<String, JsError> {
        // Input validation
        if secret.is_empty() {
            return Err(JsError::new("Secret cannot be empty"));
        }
        if threshold == 0 || total_shares == 0 {
            return Err(JsError::new("Threshold and total_shares must be greater than 0"));
        }
        if threshold > total_shares {
            return Err(JsError::new("Threshold cannot be greater than total_shares"));
        }

        // For MVP: Create mock shares that satisfy the interface
        // TODO: Implement proper Shamir Secret Sharing when compatible library is found
        let mut shares = Vec::new();
        for i in 0..total_shares {
            let share_data = format!("share_{}_{}", i + 1, js_sys::Date::now() as u64);
            shares.push(serde_json::json!({
                "index": i + 1,
                "data": share_data.as_bytes().to_vec(),
                "threshold": threshold,
                "total_shares": total_shares,
                "type": "mock_share"
            }));
        }

        // Serialize to JSON
        serde_json::to_string(&shares)
            .map_err(|e| JsError::new(&format!("Failed to serialize shares: {:?}", e)))
    }
}

/// Utility functions for JavaScript integration
#[wasm_bindgen]
pub struct Utils;

#[wasm_bindgen]
impl Utils {
    /// Convert a JavaScript Uint8Array to Vec<u8>
    #[wasm_bindgen]
    pub fn uint8_array_to_vec(array: &js_sys::Uint8Array) -> Vec<u8> {
        array.to_vec()
    }

    /// Convert Vec<u8> to JavaScript Uint8Array
    #[wasm_bindgen]
    pub fn vec_to_uint8_array(vec: Vec<u8>) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(&vec[..])
    }

    /// Get current timestamp
    #[wasm_bindgen]
    pub fn timestamp() -> f64 {
        js_sys::Date::now()
    }
}