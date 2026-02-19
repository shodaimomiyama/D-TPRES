//! Type-safe ID Value Objects for Domain Entities
//!
//! Uses newtype pattern to ensure compile-time type safety
//! and prevent mixing different entity IDs.

use std::fmt;

/// Unique identifier for Secret entity (aggregate root)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretId(String);

impl SecretId {
    /// Create a new SecretId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique SecretId using UUID v4
    pub fn generate() -> Self {
        Self(generate_uuid())
    }

    /// Get the string representation
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for ShareCollection entity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShareCollectionId(String);

impl ShareCollectionId {
    /// Create a new ShareCollectionId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique ShareCollectionId using UUID v4
    pub fn generate() -> Self {
        Self(generate_uuid())
    }

    /// Get the string representation
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ShareCollectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for Capsule entity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapsuleId(String);

impl CapsuleId {
    /// Create a new CapsuleId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique CapsuleId using UUID v4
    pub fn generate() -> Self {
        Self(generate_uuid())
    }

    /// Get the string representation
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapsuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for KFrag entity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KFragId(String);

impl KFragId {
    /// Create a new KFragId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique KFragId using UUID v4
    pub fn generate() -> Self {
        Self(generate_uuid())
    }

    /// Get the string representation
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KFragId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for CFrag entity
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CFragId(String);

impl CFragId {
    /// Create a new CFragId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique CFragId using UUID v4
    pub fn generate() -> Self {
        Self(generate_uuid())
    }

    /// Get the string representation
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CFragId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generate a UUID v4 format string
/// Uses a simple implementation suitable for WASM environment
#[allow(clippy::cast_possible_truncation)]
fn generate_uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    // Combine timestamp with counter for uniqueness
    let combined = timestamp ^ (u128::from(counter) << 64);
    let random_part = combined ^ (combined >> 32);

    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (random_part & 0xFFFF_FFFF) as u32,
        ((random_part >> 32) & 0xFFFF) as u16,
        ((random_part >> 48) & 0x0FFF) as u16,
        (((random_part >> 60) & 0x3F) | 0x80) as u16 | ((random_part & 0xFF) << 8) as u16,
        ((random_part ^ (random_part >> 16)) & 0xFFFF_FFFF_FFFF)
            ^ (u128::from(counter) & 0xFFFF_FFFF_FFFF)
    )
}
