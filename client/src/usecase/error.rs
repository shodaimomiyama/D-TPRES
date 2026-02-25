//! Service layer error definitions following TERASOLUNA guidelines
//!
//! This module defines error types for the service layer, distinguishing between
//! business exceptions (recoverable) and system exceptions (non-recoverable).
//!
//! Additionally defines WorkflowError for Workflow Service layer operations.

use thiserror::Error;

use crate::domain::errors::DomainError;

/// Service layer result type
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Service layer error hierarchy
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceError {
    /// Business exceptions - recoverable errors related to business logic
    #[error("Business error: {0}")]
    Business(#[from] BusinessException),

    /// System exceptions - non-recoverable errors related to system failures
    #[error("System error: {0}")]
    System(#[from] SystemException),
}

/// Business exceptions - errors that can be recovered from
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BusinessException {
    /// Validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// Resource not found errors
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Business rule violations
    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    /// Process state errors
    #[error("Invalid process state: {0}")]
    InvalidProcessState(String),

    /// Threshold not met errors
    #[error("Threshold not met: required {required}, got {actual}")]
    ThresholdNotMet { required: u8, actual: u8 },

    /// Invalid operation errors
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Role conflict errors
    #[error("Role conflict: {0}")]
    RoleConflict(String),
}

/// System exceptions - errors that cannot be recovered from
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SystemException {
    /// Cryptographic operation errors
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Storage operation errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Network operation errors
    #[error("Network error: {0}")]
    Network(String),

    /// Internal system errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Repository operation errors
    #[error("Repository error: {0}")]
    Repository(String),

    /// Serialization/Deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// AO Network specific errors
    #[error("AO Network error: {0}")]
    AONetwork(String),
}

/// Conversion from domain errors to service errors
impl From<DomainError> for ServiceError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::EntityValidation {
                entity_type,
                field,
                message,
            } => Self::Business(BusinessException::ValidationError(format!(
                "{entity_type} field '{field}' validation failed: {message}"
            ))),
            DomainError::InvalidStateTransition {
                entity_type,
                from_state,
                to_state,
                reason,
            } => Self::Business(BusinessException::InvalidProcessState(format!(
                "Invalid state transition for {entity_type}: {from_state} -> {to_state} ({reason})"
            ))),
            DomainError::BusinessRuleViolation { rule, message } => {
                Self::Business(BusinessException::BusinessRuleViolation(format!(
                    "Rule '{rule}' violated: {message}"
                )))
            }
            DomainError::UnauthorizedRole {
                required_role,
                actual_role,
                operation,
            } => Self::Business(BusinessException::AuthorizationError(format!(
                "Operation '{operation}' requires role '{required_role}' but got '{actual_role}'"
            ))),
            DomainError::ThresholdConstraintViolation {
                required_threshold,
                available_shares,
                operation: _,
            } => Self::Business(BusinessException::ThresholdNotMet {
                required: required_threshold,
                actual: available_shares,
            }),
            DomainError::NotFound { entity_type, id } => {
                Self::Business(BusinessException::ResourceNotFound(format!(
                    "{entity_type} with id '{id}' not found"
                )))
            }
            DomainError::CryptographicError { operation, details } => {
                Self::System(SystemException::Crypto(format!(
                    "Crypto operation '{operation}' failed: {details}"
                )))
            }
            DomainError::StorageError { operation, details } => {
                Self::System(SystemException::Storage(format!(
                    "Storage operation '{operation}' failed: {details}"
                )))
            }
            DomainError::SerializationError { operation, details } => {
                Self::System(SystemException::Serialization(format!(
                    "Serialization operation '{operation}' failed: {details}"
                )))
            }
            _ => Self::System(SystemException::Internal(format!(
                "Unexpected domain error: {err:?}"
            ))),
        }
    }
}

/// Helper methods for ServiceError
impl ServiceError {
    /// Check if this error is recoverable (business errors are recoverable)
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Business(_))
    }

    /// Check if this is a business error
    #[must_use]
    pub const fn is_business_error(&self) -> bool {
        matches!(self, Self::Business(_))
    }

    /// Check if this is a system error
    #[must_use]
    pub const fn is_system_error(&self) -> bool {
        matches!(self, Self::System(_))
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::Business(BusinessException::ValidationError(message.into()))
    }

    /// Create an authorization error
    pub fn authorization_error(message: impl Into<String>) -> Self {
        Self::Business(BusinessException::AuthorizationError(message.into()))
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::Business(BusinessException::ResourceNotFound(message.into()))
    }

    /// Create a crypto error
    pub fn crypto_error(message: impl Into<String>) -> Self {
        Self::System(SystemException::Crypto(message.into()))
    }

    /// Create a storage error
    pub fn storage_error(message: impl Into<String>) -> Self {
        Self::System(SystemException::Storage(message.into()))
    }

    /// Create an AO network error
    pub fn ao_network_error(message: impl Into<String>) -> Self {
        Self::System(SystemException::AONetwork(message.into()))
    }
}

// ============================================================================
// WorkflowError - Workflow Service Layer Errors
// ============================================================================

/// Workflow layer result type
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Workflow layer error hierarchy
///
/// Defines errors specific to Workflow Service operations that orchestrate
/// Core Services for PHASE 1 (secret sharing) and PHASE 3 (secret recovery).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkflowError {
    /// Input parameter validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Crypto operation failures (from CryptoService)
    #[error("Crypto operation failed: {0}")]
    CryptoError(String),

    /// Storage operation failures (from StorageService)
    #[error("Storage operation failed: {0}")]
    StorageError(String),

    /// AO Network communication failures
    #[error("AO communication failed: {0}")]
    AOCommunicationError(String),

    /// Insufficient cFrags for threshold reconstruction
    #[error("Insufficient cFrags: need {required}, got {actual}")]
    InsufficientCFrags { required: u8, actual: u8 },

    /// Decryption failures at specific phases
    #[error("Decryption failed at phase: {phase}")]
    DecryptionError { phase: String },

    /// Resource not found errors
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Arweave batch storage partially failed (immutable storage, no rollback)
    #[error("Partial storage failure: {failed_count} of {total_count} items failed")]
    PartialStorageFailure {
        capsule_tx_id: String,
        successful_share_tx_ids: Vec<String>,
        failed_shares: Vec<(String, String)>,
        failed_count: usize,
        total_count: usize,
    },

    /// DelegateCapsule to AO partially failed mid-loop (AO state may be inconsistent)
    ///
    /// Encrypted shares and capsule **are safely stored on Arweave** (`share_tx_ids`
    /// and `capsule_tx_id`). Only the AO delegation step had failures.
    ///
    /// Whether retry is safe depends on the AO contract's idempotency guarantee.
    /// Consult the AO contract spec before retrying DelegateCapsule calls.
    #[error("DelegateCapsule partial failure: {failed_count} of {total_count} kFrag delegations failed for capsule {capsule_tx_id}")]
    PartialDelegateCapsuleFailure {
        /// Arweave capsule tx ID (= `capsule_id` on AO contract)
        capsule_tx_id: String,
        /// Arweave share tx IDs — encrypted shares are stored even on failure
        share_tx_ids: Vec<String>,
        /// kFrag IDs that were successfully delegated
        successful_kfrag_ids: Vec<String>,
        /// (kfrag_id, error_message) pairs for failed delegations
        failed_kfrag_ids: Vec<(String, String)>,
        failed_count: usize,
        total_count: usize,
    },
}

/// Conversion from ServiceError to WorkflowError
impl From<ServiceError> for WorkflowError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Business(business_err) => match business_err {
                BusinessException::ValidationError(msg) => WorkflowError::ValidationError(msg),
                BusinessException::ResourceNotFound(msg) => WorkflowError::ResourceNotFound(msg),
                BusinessException::ThresholdNotMet { required, actual } => {
                    WorkflowError::InsufficientCFrags { required, actual }
                }
                other => WorkflowError::ValidationError(other.to_string()),
            },
            ServiceError::System(system_err) => match system_err {
                SystemException::Crypto(msg) => WorkflowError::CryptoError(msg),
                SystemException::Storage(msg) => WorkflowError::StorageError(msg),
                SystemException::AONetwork(msg) => WorkflowError::AOCommunicationError(msg),
                other => WorkflowError::StorageError(other.to_string()),
            },
        }
    }
}

/// Helper methods for WorkflowError
impl WorkflowError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }

    /// Create a crypto error
    pub fn crypto(message: impl Into<String>) -> Self {
        Self::CryptoError(message.into())
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>) -> Self {
        Self::StorageError(message.into())
    }

    /// Create an AO communication error
    pub fn ao_communication(message: impl Into<String>) -> Self {
        Self::AOCommunicationError(message.into())
    }

    /// Create an insufficient cFrags error
    #[must_use]
    pub const fn insufficient_cfrags(required: u8, actual: u8) -> Self {
        Self::InsufficientCFrags { required, actual }
    }

    /// Create a decryption error
    pub fn decryption(phase: impl Into<String>) -> Self {
        Self::DecryptionError {
            phase: phase.into(),
        }
    }

    /// Create a resource not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::ResourceNotFound(message.into())
    }

    /// Check if this error is recoverable
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ValidationError(_) | Self::ResourceNotFound(_) | Self::InsufficientCFrags { .. }
        )
    }

    /// Create a partial DelegateCapsule failure error
    ///
    /// `share_tx_ids` should contain the Arweave tx IDs of encrypted shares that
    /// were successfully stored, so callers know Arweave data is intact.
    pub fn partial_delegate_capsule_failure(
        capsule_tx_id: impl Into<String>,
        share_tx_ids: Vec<String>,
        successful_kfrag_ids: Vec<String>,
        failed_kfrag_ids: Vec<(String, String)>,
    ) -> Self {
        let failed_count = failed_kfrag_ids.len();
        let total_count = successful_kfrag_ids.len() + failed_count;
        Self::PartialDelegateCapsuleFailure {
            capsule_tx_id: capsule_tx_id.into(),
            share_tx_ids,
            successful_kfrag_ids,
            failed_kfrag_ids,
            failed_count,
            total_count,
        }
    }
}
