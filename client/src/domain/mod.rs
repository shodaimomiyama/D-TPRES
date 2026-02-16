//! Domain layer for FORMIX
//!
//! Contains entities, repository interfaces, value objects, and domain errors.
//! This layer implements pure business logic without any dependencies on
//! external frameworks or infrastructure concerns.

pub mod entities;
pub mod errors;
pub mod value_objects;

// Re-export entities
pub use entities::{
    CFrag, Capsule, EncryptedShareData, KFrag, Secret, SecretState, ShareCollection,
};

// Re-export value objects
pub use value_objects::{
    CFragId, CapsuleId, KFragId, KeyPair, SYMMETRIC_KEY_SIZE, SecretData, SecretId,
    ShareCollectionId, SymmetricKey,
};

pub use errors::{DomainError, DomainResult};
