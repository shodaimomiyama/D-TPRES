//! KeyPair Value Object
//!
//! Represents an Umbral PRE key pair (secret_key, public_key).
//! Secret key is zeroized on drop for memory safety.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// PRE key pair value object
///
/// Holds serialized Umbral PRE key pair.
/// Secret key is automatically zeroized when dropped.
///
/// # Security
/// - Implements Zeroize/ZeroizeOnDrop for the secret key
/// - Does not implement Clone to prevent accidental copying
/// - Debug output redacts the secret key
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KeyPair {
    secret_key: Vec<u8>,
    #[zeroize(skip)]
    public_key: Vec<u8>,
}

impl KeyPair {
    /// Create a new KeyPair from raw key bytes
    ///
    /// # Arguments
    /// * `secret_key` - Serialized Umbral SecretKey bytes
    /// * `public_key` - Serialized Umbral PublicKey bytes
    ///
    /// # Returns
    /// New KeyPair instance
    pub fn new(secret_key: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            secret_key,
            public_key,
        }
    }

    /// Get the secret key bytes
    ///
    /// # Security
    /// Handle returned bytes carefully - they contain sensitive material
    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    /// Get the public key bytes
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("secret_key", &"[REDACTED]")
            .field("public_key_len", &self.public_key.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_pair_new() {
        let sk = vec![1u8; 32];
        let pk = vec![2u8; 33];

        let pair = KeyPair::new(sk.clone(), pk.clone());

        assert_eq!(pair.secret_key(), sk.as_slice());
        assert_eq!(pair.public_key(), pk.as_slice());
    }

    #[test]
    fn test_key_pair_debug_redacted() {
        let pair = KeyPair::new(vec![1u8; 32], vec![2u8; 33]);

        let debug_str = format!("{:?}", pair);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("[1, 1, 1"));
    }
}
