//! Domain entities for D-TPRES
//!
//! Pure data structures representing business concepts in the D-TPRES system.
//! All entities follow these principles:
//! - No methods (only data)
//! - Serializable/Deserializable
//! - Private fields with public access (struct fields can be public)
//! - Zeroize for sensitive data

pub mod access_request;
pub mod capsule;
pub mod process;
pub mod reencryption;
pub mod secret_details;
pub mod share;

pub use access_request::{AccessRequestEntity, EvmVerificationData, ProofPkgData};
pub use capsule::CapsuleEntity;
pub use process::{
    EntityReferences, HolderData, HolderFragmentInfo, OwnerData, PerformanceMetrics, ProcessEntity,
    RequesterData, SecretIndex,
};
pub use reencryption::{CFragData, ReencryptionEntity};
pub use secret_details::{AccessRecord, SecretDetailsEntity};
pub use share::ShareEntity;
