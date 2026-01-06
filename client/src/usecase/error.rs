//! Service layer error definitions following TERASOLUNA guidelines
//!
//! This module defines error types for the service layer, distinguishing between
//! business exceptions (recoverable) and system exceptions (non-recoverable).

use thiserror::Error;

use crate::domain::errors::DomainError;

/// Service layer result type
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Service layer error hierarchy
#[derive(Debug, Error)]
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
pub enum SystemException {
    /// Cryptographic operation errors
    #[error("Crypto error: {0}")]
    CryptoError(String),

    /// Storage operation errors
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Network operation errors
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Internal system errors
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Repository operation errors
    #[error("Repository error: {0}")]
    RepositoryError(String),

    /// Serialization/Deserialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// AO Network specific errors
    #[error("AO Network error: {0}")]
    AONetworkError(String),
}

/// Conversion from domain errors to service errors
impl From<DomainError> for ServiceError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::EntityValidation {
                entity_type,
                field,
                message,
            } => ServiceError::Business(BusinessException::ValidationError(format!(
                "{} field '{}' validation failed: {}",
                entity_type, field, message
            ))),
            DomainError::InvalidStateTransition {
                entity_type,
                from_state,
                to_state,
                reason,
            } => ServiceError::Business(BusinessException::InvalidProcessState(format!(
                "Invalid state transition for {}: {} -> {} ({})",
                entity_type, from_state, to_state, reason
            ))),
            DomainError::BusinessRuleViolation { rule, message } => {
                ServiceError::Business(BusinessException::BusinessRuleViolation(format!(
                    "Rule '{}' violated: {}",
                    rule, message
                )))
            }
            DomainError::UnauthorizedRole {
                required_role,
                actual_role,
                operation,
            } => ServiceError::Business(BusinessException::AuthorizationError(format!(
                "Operation '{}' requires role '{}' but got '{}'",
                operation, required_role, actual_role
            ))),
            DomainError::ThresholdConstraintViolation {
                required_threshold,
                available_shares,
                operation: _,
            } => ServiceError::Business(BusinessException::ThresholdNotMet {
                required: required_threshold,
                actual: available_shares,
            }),
            DomainError::NotFound { entity_type, id } => {
                ServiceError::Business(BusinessException::ResourceNotFound(format!(
                    "{} with id '{}' not found",
                    entity_type, id
                )))
            }
            DomainError::CryptographicError { operation, details } => {
                ServiceError::System(SystemException::CryptoError(format!(
                    "Crypto operation '{}' failed: {}",
                    operation, details
                )))
            }
            DomainError::StorageError { operation, details } => {
                ServiceError::System(SystemException::StorageError(format!(
                    "Storage operation '{}' failed: {}",
                    operation, details
                )))
            }
            DomainError::SerializationError { operation, details } => {
                ServiceError::System(SystemException::SerializationError(format!(
                    "Serialization operation '{}' failed: {}",
                    operation, details
                )))
            }
            _ => ServiceError::System(SystemException::InternalError(format!(
                "Unexpected domain error: {:?}",
                err
            ))),
        }
    }
}

/// Helper methods for ServiceError
impl ServiceError {
    /// Check if this error is recoverable (business errors are recoverable)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ServiceError::Business(_))
    }

    /// Check if this is a business error
    pub fn is_business_error(&self) -> bool {
        matches!(self, ServiceError::Business(_))
    }

    /// Check if this is a system error
    pub fn is_system_error(&self) -> bool {
        matches!(self, ServiceError::System(_))
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        ServiceError::Business(BusinessException::ValidationError(message.into()))
    }

    /// Create an authorization error
    pub fn authorization_error(message: impl Into<String>) -> Self {
        ServiceError::Business(BusinessException::AuthorizationError(message.into()))
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        ServiceError::Business(BusinessException::ResourceNotFound(message.into()))
    }

    /// Create a crypto error
    pub fn crypto_error(message: impl Into<String>) -> Self {
        ServiceError::System(SystemException::CryptoError(message.into()))
    }

    /// Create a storage error
    pub fn storage_error(message: impl Into<String>) -> Self {
        ServiceError::System(SystemException::StorageError(message.into()))
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
}
