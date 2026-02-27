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
    pub const fn new(secret_key: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            secret_key,
            public_key,
        }
    }

    /// Get the secret key bytes
    ///
    /// # Security
    /// Handle returned bytes carefully - they contain sensitive material
    #[allow(clippy::missing_const_for_fn)]
    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    /// Get the public key bytes
    #[allow(clippy::missing_const_for_fn)]
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
