//! Domain layer error definitions
//!
//! Comprehensive error types for FORMIX domain operations.
//! All errors follow the principle of explicit error handling without panic.

use std::fmt;

/// Comprehensive domain error enumeration
///
/// Represents all possible errors that can occur within the domain layer.
/// Designed for AO stateless execution environment - no async operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainError {
    /// Entity validation failed
    EntityValidation {
        entity_type: String,
        field: String,
        message: String,
    },

    /// Invalid state transition attempted
    InvalidStateTransition {
        entity_type: String,
        from_state: String,
        to_state: String,
        reason: String,
    },

    /// Business rule violation
    BusinessRuleViolation { rule: String, message: String },

    /// Process role authorization failed
    UnauthorizedRole {
        required_role: String,
        actual_role: String,
        operation: String,
    },

    /// Threshold cryptography constraints violation
    ThresholdConstraintViolation {
        required_threshold: u8,
        available_shares: u8,
        operation: String,
    },

    /// Secret management constraint violation
    SecretConstraintViolation {
        secret_id: String,
        constraint: String,
        message: String,
    },

    /// Access control condition not met
    AccessControlViolation {
        condition: String,
        accessor_id: String,
        message: String,
    },

    /// Cryptographic operation failed
    CryptographicError { operation: String, details: String },

    /// Entity relationship constraint violation
    RelationshipConstraintViolation {
        parent_entity: String,
        child_entity: String,
        constraint: String,
    },

    /// Process phase constraint violation
    PhaseConstraintViolation {
        current_phase: String,
        required_phase: String,
        operation: String,
    },

    /// Entity identifier constraint violation
    IdentifierConstraintViolation {
        entity_type: String,
        identifier: String,
        constraint: String,
    },

    /// Timeout occurred during operation
    OperationTimeout {
        operation: String,
        timeout_seconds: u64,
    },

    /// Concurrent access detected (optimistic locking)
    ConcurrentAccess {
        entity_type: String,
        entity_id: String,
        expected_version: u64,
        actual_version: u64,
    },

    /// Configuration constraint violation
    ConfigurationError {
        parameter: String,
        value: String,
        constraint: String,
    },

    /// Generic validation error for complex validations
    ValidationError { message: String },

    /// Internal domain logic error
    InternalError { message: String },

    /// Entity not found in repository
    NotFound { entity_type: String, id: String },

    /// Entity already exists in repository
    AlreadyExists { entity_type: String, id: String },

    /// Storage system error
    StorageError { operation: String, details: String },

    /// Serialization/Deserialization error
    SerializationError { operation: String, details: String },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityValidation {
                entity_type,
                field,
                message,
            } => {
                write!(
                    f,
                    "Entity validation failed for {entity_type}.{field}: {message}"
                )
            }
            Self::InvalidStateTransition {
                entity_type,
                from_state,
                to_state,
                reason,
            } => {
                write!(
                    f,
                    "Invalid state transition in {entity_type}: {from_state} -> {to_state} ({reason})"
                )
            }
            Self::BusinessRuleViolation { rule, message } => {
                write!(f, "Business rule '{rule}' violation: {message}")
            }
            Self::UnauthorizedRole {
                required_role,
                actual_role,
                operation,
            } => {
                write!(
                    f,
                    "Unauthorized role for operation '{operation}': required={required_role}, actual={actual_role}"
                )
            }
            Self::ThresholdConstraintViolation {
                required_threshold,
                available_shares,
                operation,
            } => {
                write!(
                    f,
                    "Threshold constraint violation in '{operation}': required={required_threshold}, available={available_shares}"
                )
            }
            Self::SecretConstraintViolation {
                secret_id,
                constraint,
                message,
            } => {
                write!(
                    f,
                    "Secret constraint '{constraint}' violation for {secret_id}: {message}"
                )
            }
            Self::AccessControlViolation {
                condition,
                accessor_id,
                message,
            } => {
                write!(
                    f,
                    "Access control violation for condition '{condition}' by {accessor_id}: {message}"
                )
            }
            Self::CryptographicError { operation, details } => {
                write!(
                    f,
                    "Cryptographic error in operation '{operation}': {details}"
                )
            }
            Self::RelationshipConstraintViolation {
                parent_entity,
                child_entity,
                constraint,
            } => {
                write!(
                    f,
                    "Relationship constraint '{constraint}' violation between {parent_entity} and {child_entity}"
                )
            }
            Self::PhaseConstraintViolation {
                current_phase,
                required_phase,
                operation,
            } => {
                write!(
                    f,
                    "Phase constraint violation for operation '{operation}': current={current_phase}, required={required_phase}"
                )
            }
            Self::IdentifierConstraintViolation {
                entity_type,
                identifier,
                constraint,
            } => {
                write!(
                    f,
                    "Identifier constraint '{constraint}' violation for {entity_type} with ID '{identifier}'"
                )
            }
            Self::OperationTimeout {
                operation,
                timeout_seconds,
            } => {
                write!(
                    f,
                    "Operation '{operation}' timed out after {timeout_seconds} seconds"
                )
            }
            Self::ConcurrentAccess {
                entity_type,
                entity_id,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "Concurrent access detected for {entity_type} {entity_id}: expected version {expected_version}, actual {actual_version}"
                )
            }
            Self::ConfigurationError {
                parameter,
                value,
                constraint,
            } => {
                write!(
                    f,
                    "Configuration error for parameter '{parameter}': value '{value}' violates constraint '{constraint}'"
                )
            }
            Self::ValidationError { message } => {
                write!(f, "Validation error: {message}")
            }
            Self::InternalError { message } => {
                write!(f, "Internal domain error: {message}")
            }
            Self::NotFound { entity_type, id } => {
                write!(f, "{entity_type} not found: {id}")
            }
            Self::AlreadyExists { entity_type, id } => {
                write!(f, "{entity_type} already exists: {id}")
            }
            Self::StorageError { operation, details } => {
                write!(f, "Storage error in operation '{operation}': {details}")
            }
            Self::SerializationError { operation, details } => {
                write!(
                    f,
                    "Serialization error in operation '{operation}': {details}"
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// Result type alias for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

/// Domain error creation helpers
impl DomainError {
    /// Create entity validation error
    pub fn entity_validation(entity_type: &str, field: &str, message: &str) -> Self {
        Self::EntityValidation {
            entity_type: entity_type.to_string(),
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    /// Create invalid state transition error
    pub fn invalid_state_transition(
        entity_type: &str,
        from_state: &str,
        to_state: &str,
        reason: &str,
    ) -> Self {
        Self::InvalidStateTransition {
            entity_type: entity_type.to_string(),
            from_state: from_state.to_string(),
            to_state: to_state.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create business rule violation error
    pub fn business_rule_violation(rule: &str, message: &str) -> Self {
        Self::BusinessRuleViolation {
            rule: rule.to_string(),
            message: message.to_string(),
        }
    }

    /// Create unauthorized role error
    pub fn unauthorized_role(required_role: &str, actual_role: &str, operation: &str) -> Self {
        Self::UnauthorizedRole {
            required_role: required_role.to_string(),
            actual_role: actual_role.to_string(),
            operation: operation.to_string(),
        }
    }

    /// Create threshold constraint violation error
    pub fn threshold_constraint_violation(
        required_threshold: u8,
        available_shares: u8,
        operation: &str,
    ) -> Self {
        Self::ThresholdConstraintViolation {
            required_threshold,
            available_shares,
            operation: operation.to_string(),
        }
    }

    /// Create secret constraint violation error
    pub fn secret_constraint_violation(secret_id: &str, constraint: &str, message: &str) -> Self {
        Self::SecretConstraintViolation {
            secret_id: secret_id.to_string(),
            constraint: constraint.to_string(),
            message: message.to_string(),
        }
    }

    /// Create access control violation error
    pub fn access_control_violation(condition: &str, accessor_id: &str, message: &str) -> Self {
        Self::AccessControlViolation {
            condition: condition.to_string(),
            accessor_id: accessor_id.to_string(),
            message: message.to_string(),
        }
    }

    /// Create cryptographic error
    pub fn cryptographic_error(operation: &str, details: &str) -> Self {
        Self::CryptographicError {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }

    /// Create phase constraint violation error
    pub fn phase_constraint_violation(
        current_phase: &str,
        required_phase: &str,
        operation: &str,
    ) -> Self {
        Self::PhaseConstraintViolation {
            current_phase: current_phase.to_string(),
            required_phase: required_phase.to_string(),
            operation: operation.to_string(),
        }
    }

    /// Create concurrent access error
    pub fn concurrent_access(
        entity_type: &str,
        entity_id: &str,
        expected_version: u64,
        actual_version: u64,
    ) -> Self {
        Self::ConcurrentAccess {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            expected_version,
            actual_version,
        }
    }

    /// Create validation error
    pub fn validation_error(message: &str) -> Self {
        Self::ValidationError {
            message: message.to_string(),
        }
    }

    /// Create internal error
    pub fn internal_error(message: &str) -> Self {
        Self::InternalError {
            message: message.to_string(),
        }
    }

    /// Create not found error
    pub fn not_found(entity_type: &str, id: &str) -> Self {
        Self::NotFound {
            entity_type: entity_type.to_string(),
            id: id.to_string(),
        }
    }

    /// Create already exists error
    pub fn already_exists(entity_type: &str, id: &str) -> Self {
        Self::AlreadyExists {
            entity_type: entity_type.to_string(),
            id: id.to_string(),
        }
    }

    /// Create storage error
    pub fn storage_error(operation: &str, details: &str) -> Self {
        Self::StorageError {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }

    /// Create serialization error
    pub fn serialization_error(operation: &str, details: &str) -> Self {
        Self::SerializationError {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }
}
