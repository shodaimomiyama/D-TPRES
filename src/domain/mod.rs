//! Domain layer for D-TPRES
//!
//! Contains entities, repository interfaces, value objects, and domain errors.
//! This layer implements pure business logic without any dependencies on
//! external frameworks or infrastructure concerns.

pub mod entities;
pub mod repositories;
pub mod errors;

// Re-export commonly used types
pub use entities::{
    AccessRequestEntity, CapsuleEntity, ProcessEntity, ReencryptionEntity, RekeyFragmentEntity,
    SecretDetailsEntity, ShareEntity,
};
pub use entities::{
    AccessRecord, CFragData, EntityReferences, EvmVerificationData, HolderData, HolderFragmentInfo,
    OwnerData, PerformanceMetrics, ProcessEntity as Process, ProofPkgData, RequesterData,
    SecretIndex,
};
pub use entities::{
    AccessRequestStatus, AccessResult, CryptoOperation, CryptoPhase, ProcessRole,
    ReencryptionStatus, RekeyFragmentStatus, SecretStatus,
};

pub use repositories::{
    AccessRequestEntityRepository, CapsuleEntityRepository, ProcessEntityRepository,
    ReencryptionEntityRepository, RekeyFragmentEntityRepository, SecretDetailsEntityRepository,
    ShareEntityRepository,
};

pub use errors::{DomainError, DomainResult};