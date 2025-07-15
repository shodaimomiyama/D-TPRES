//! Domain entities for D-TPRES
//!
//! Pure data structures representing business concepts in the D-TPRES system.
//! All entities follow these principles:
//! - No methods (only data)
//! - Serializable/Deserializable
//! - Private fields with public access (struct fields can be public)
//! - Zeroize for sensitive data

pub mod process;
pub mod share;
pub mod capsule;
pub mod access_request;
pub mod rekey_fragment;
pub mod reencryption;
pub mod secret_details;

pub use process::{
    ProcessEntity, OwnerData, HolderData, RequesterData, 
    PerformanceMetrics, SecretIndex, EntityReferences, HolderFragmentInfo
};
pub use share::ShareEntity;
pub use capsule::CapsuleEntity;
pub use access_request::{AccessRequestEntity, EvmVerificationData, ProofPkgData};
pub use rekey_fragment::RekeyFragmentEntity;
pub use reencryption::{ReencryptionEntity, CFragData};
pub use secret_details::{SecretDetailsEntity, AccessRecord};