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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let business_err = ServiceError::validation_error("Invalid input");
        assert!(business_err.is_business_error());
        assert!(!business_err.is_system_error());

        let system_err = ServiceError::crypto_error("Encryption failed");
        assert!(!system_err.is_business_error());
        assert!(system_err.is_system_error());
    }

    #[test]
    fn test_domain_error_conversion() {
        let domain_err = DomainError::EntityValidation {
            entity_type: "User".to_string(),
            field: "email".to_string(),
            message: "Invalid format".to_string(),
        };

        let service_err: ServiceError = domain_err.into();
        assert!(service_err.is_business_error());
    }

    // ========================================================================
    // WorkflowError Unit Tests (Task 13)
    // ========================================================================

    #[test]
    fn test_workflow_error_validation() {
        let err = WorkflowError::validation("Invalid threshold value");
        assert!(matches!(err, WorkflowError::ValidationError(_)));
        assert!(err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("Validation error"));
        assert!(msg.contains("Invalid threshold value"));
    }

    #[test]
    fn test_workflow_error_from_crypto_service() {
        // Test conversion from ServiceError::System(CryptoError) to WorkflowError::CryptoError
        let service_err = ServiceError::crypto_error("Encryption failed");
        let workflow_err: WorkflowError = service_err.into();

        assert!(matches!(workflow_err, WorkflowError::CryptoError(_)));
        assert!(!workflow_err.is_recoverable());

        let msg = workflow_err.to_string();
        assert!(msg.contains("Crypto operation failed"));
    }

    #[test]
    fn test_workflow_error_from_storage_service() {
        // Test conversion from ServiceError::System(StorageError) to WorkflowError::StorageError
        let service_err = ServiceError::storage_error("Arweave upload failed");
        let workflow_err: WorkflowError = service_err.into();

        assert!(matches!(workflow_err, WorkflowError::StorageError(_)));
        assert!(!workflow_err.is_recoverable());

        let msg = workflow_err.to_string();
        assert!(msg.contains("Storage operation failed"));
    }

    #[test]
    fn test_workflow_error_ao_communication() {
        let err = WorkflowError::ao_communication("Failed to send kFrags to Owner-Process");
        assert!(matches!(err, WorkflowError::AOCommunicationError(_)));
        assert!(!err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("AO communication failed"));
        assert!(msg.contains("Failed to send kFrags"));

        // Test conversion from ServiceError::System(AONetworkError)
        let service_err = ServiceError::ao_network_error("Network timeout");
        let workflow_err: WorkflowError = service_err.into();
        assert!(matches!(
            workflow_err,
            WorkflowError::AOCommunicationError(_)
        ));
    }

    #[test]
    fn test_workflow_error_insufficient_cfrags() {
        let err = WorkflowError::insufficient_cfrags(3, 2);
        assert!(matches!(
            err,
            WorkflowError::InsufficientCFrags {
                required: 3,
                actual: 2
            }
        ));
        assert!(err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("Insufficient cFrags"));
        assert!(msg.contains("need 3"));
        assert!(msg.contains("got 2"));

        // Test conversion from ServiceError::Business(ThresholdNotMet)
        let service_err = ServiceError::Business(BusinessException::ThresholdNotMet {
            required: 5,
            actual: 3,
        });
        let workflow_err: WorkflowError = service_err.into();
        assert!(matches!(
            workflow_err,
            WorkflowError::InsufficientCFrags {
                required: 5,
                actual: 3
            }
        ));
    }

    #[test]
    fn test_workflow_error_decryption() {
        let err = WorkflowError::decryption("PRE decapsulation");
        assert!(matches!(err, WorkflowError::DecryptionError { .. }));
        assert!(!err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("Decryption failed"));
        assert!(msg.contains("PRE decapsulation"));

        // Test with phase detail
        let err2 = WorkflowError::DecryptionError {
            phase: "AES-GCM share decryption".to_string(),
        };
        let msg2 = err2.to_string();
        assert!(msg2.contains("AES-GCM share decryption"));
    }

    #[test]
    fn test_workflow_error_resource_not_found() {
        let err = WorkflowError::not_found("Secret abc123 not found");
        assert!(matches!(err, WorkflowError::ResourceNotFound(_)));
        assert!(err.is_recoverable());

        let msg = err.to_string();
        assert!(msg.contains("Resource not found"));
        assert!(msg.contains("abc123"));

        // Test conversion from ServiceError::Business(ResourceNotFound)
        let service_err = ServiceError::not_found("Capsule not found");
        let workflow_err: WorkflowError = service_err.into();
        assert!(matches!(workflow_err, WorkflowError::ResourceNotFound(_)));
    }

    #[test]
    fn test_workflow_error_from_validation_business() {
        // Test conversion from ServiceError::Business(ValidationError)
        let service_err = ServiceError::validation_error("Invalid parameters");
        let workflow_err: WorkflowError = service_err.into();

        assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));
        assert!(workflow_err.is_recoverable());
    }

    #[test]
    fn test_workflow_error_from_other_business() {
        // Test that other business exceptions convert to ValidationError
        let service_err = ServiceError::Business(BusinessException::AuthorizationError(
            "Not authorized".into(),
        ));
        let workflow_err: WorkflowError = service_err.into();

        // Should convert to ValidationError as fallback
        assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));
    }

    #[test]
    fn test_workflow_error_from_other_system() {
        // Test that other system exceptions convert to StorageError
        let service_err =
            ServiceError::System(SystemException::Network("Connection refused".into()));
        let workflow_err: WorkflowError = service_err.into();

        // Should convert to StorageError as fallback
        assert!(matches!(workflow_err, WorkflowError::StorageError(_)));
    }

    #[test]
    fn test_workflow_error_is_recoverable() {
        // Recoverable errors
        assert!(WorkflowError::validation("test").is_recoverable());
        assert!(WorkflowError::not_found("test").is_recoverable());
        assert!(WorkflowError::insufficient_cfrags(3, 2).is_recoverable());

        // Non-recoverable errors
        assert!(!WorkflowError::crypto("test").is_recoverable());
        assert!(!WorkflowError::storage("test").is_recoverable());
        assert!(!WorkflowError::ao_communication("test").is_recoverable());
        assert!(!WorkflowError::decryption("test").is_recoverable());
        assert!(
            !WorkflowError::PartialStorageFailure {
                capsule_tx_id: "tx".to_string(),
                successful_share_tx_ids: vec![],
                failed_shares: vec![],
                failed_count: 0,
                total_count: 0,
            }
            .is_recoverable()
        );
    }

    #[test]
    fn test_workflow_error_partial_storage_failure() {
        let err = WorkflowError::PartialStorageFailure {
            capsule_tx_id: "tx_capsule_001".to_string(),
            successful_share_tx_ids: vec!["tx_share_0".to_string()],
            failed_shares: vec![("1".to_string(), "storage error".to_string())],
            failed_count: 1,
            total_count: 2,
        };

        let msg = err.to_string();
        assert!(msg.contains("Partial storage failure"));
        assert!(msg.contains("1 of 2"));
        assert!(!err.is_recoverable());

        match err {
            WorkflowError::PartialStorageFailure {
                capsule_tx_id,
                successful_share_tx_ids,
                failed_shares,
                failed_count,
                total_count,
            } => {
                assert_eq!(capsule_tx_id, "tx_capsule_001");
                assert_eq!(successful_share_tx_ids.len(), 1);
                assert_eq!(failed_shares.len(), 1);
                assert_eq!(failed_count, 1);
                assert_eq!(total_count, 2);
                assert_eq!(failed_shares[0].0, "1");
            }
            _ => panic!("Expected PartialStorageFailure"),
        }
    }
}
