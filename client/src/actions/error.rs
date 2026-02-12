use std::fmt;

use crate::controller::error::ValidationError;
use crate::usecase::error::WorkflowError;

/// Actions層のエラー型
#[derive(Debug)]
#[non_exhaustive]
pub enum ActionError {
    /// Controller層でのバリデーション失敗
    ValidationFailed {
        /// エラーコード（例: "secret_empty", "invalid_threshold"）
        code: String,
        /// 人間可読なエラーメッセージ
        message: String,
    },
    /// UseCase層でのワークフロー失敗
    WorkflowFailed {
        /// エラーメッセージ
        message: String,
    },
    /// リソースが見つからない
    ResourceNotFound {
        /// リソース種別（例: "secret", "process"）
        resource: String,
    },
    /// 暗号操作エラー
    CryptoError {
        /// エラーメッセージ
        message: String,
    },
    /// Arweave batch storage partially failed (immutable storage, no rollback)
    PartialStorageFailure {
        capsule_tx_id: String,
        successful_share_tx_ids: Vec<String>,
        failed_shares: Vec<(String, String)>,
        message: String,
    },
}

impl ActionError {
    /// ValidationFailed を作成
    pub fn validation_failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            code: code.into(),
            message: message.into(),
        }
    }

    /// WorkflowFailed を作成
    pub fn workflow_failed(message: impl Into<String>) -> Self {
        Self::WorkflowFailed {
            message: message.into(),
        }
    }

    /// ResourceNotFound を作成
    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::ResourceNotFound {
            resource: resource.into(),
        }
    }

    /// CryptoError を作成
    pub fn crypto_error(message: impl Into<String>) -> Self {
        Self::CryptoError {
            message: message.into(),
        }
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed { code, message } => {
                write!(f, "Validation failed [{code}]: {message}")
            }
            Self::WorkflowFailed { message } => {
                write!(f, "Workflow failed: {message}")
            }
            Self::ResourceNotFound { resource } => {
                write!(f, "Resource not found: {resource}")
            }
            Self::CryptoError { message } => {
                write!(f, "Crypto error: {message}")
            }
            Self::PartialStorageFailure { message, .. } => {
                write!(f, "Partial storage failure: {message}")
            }
        }
    }
}

impl std::error::Error for ActionError {}

impl From<ValidationError> for ActionError {
    fn from(err: ValidationError) -> Self {
        Self::ValidationFailed {
            code: err.code().to_string(),
            message: err.message().to_string(),
        }
    }
}

impl From<WorkflowError> for ActionError {
    fn from(err: WorkflowError) -> Self {
        match err {
            WorkflowError::ValidationError(msg) => Self::ValidationFailed {
                code: "workflow_validation".to_string(),
                message: msg,
            },
            WorkflowError::ResourceNotFound(resource) => Self::ResourceNotFound { resource },
            WorkflowError::CryptoError(msg) => Self::CryptoError { message: msg },
            WorkflowError::PartialStorageFailure {
                capsule_tx_id,
                successful_share_tx_ids,
                failed_shares,
                failed_count,
                total_count,
            } => Self::PartialStorageFailure {
                capsule_tx_id,
                successful_share_tx_ids,
                failed_shares,
                message: format!("{failed_count} of {total_count} share storage operations failed"),
            },
            _ => Self::WorkflowFailed {
                message: err.to_string(),
            },
        }
    }
}

/// Actions層の結果型
pub type ActionResult<T> = Result<T, ActionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_error_validation_failed_constructor() {
        let err = ActionError::validation_failed("test_code", "test message");
        match err {
            ActionError::ValidationFailed { code, message } => {
                assert_eq!(code, "test_code");
                assert_eq!(message, "test message");
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_action_error_workflow_failed_constructor() {
        let err = ActionError::workflow_failed("workflow error");
        match err {
            ActionError::WorkflowFailed { message } => {
                assert_eq!(message, "workflow error");
            }
            _ => panic!("Expected WorkflowFailed"),
        }
    }

    #[test]
    fn test_action_error_resource_not_found_constructor() {
        let err = ActionError::resource_not_found("secret");
        match err {
            ActionError::ResourceNotFound { resource } => {
                assert_eq!(resource, "secret");
            }
            _ => panic!("Expected ResourceNotFound"),
        }
    }

    #[test]
    fn test_action_error_crypto_error_constructor() {
        let err = ActionError::crypto_error("crypto failed");
        match err {
            ActionError::CryptoError { message } => {
                assert_eq!(message, "crypto failed");
            }
            _ => panic!("Expected CryptoError"),
        }
    }

    #[test]
    fn test_action_error_display_validation_failed() {
        let err = ActionError::validation_failed("test", "Test error");
        let display = err.to_string();
        assert!(display.contains("test"));
        assert!(display.contains("Test error"));
        assert!(display.contains("Validation failed"));
    }

    #[test]
    fn test_action_error_display_workflow_failed() {
        let err = ActionError::workflow_failed("workflow error");
        let display = err.to_string();
        assert!(display.contains("Workflow failed"));
        assert!(display.contains("workflow error"));
    }

    #[test]
    fn test_action_error_display_resource_not_found() {
        let err = ActionError::resource_not_found("secret");
        let display = err.to_string();
        assert!(display.contains("Resource not found"));
        assert!(display.contains("secret"));
    }

    #[test]
    fn test_action_error_display_crypto_error() {
        let err = ActionError::crypto_error("crypto failed");
        let display = err.to_string();
        assert!(display.contains("Crypto error"));
        assert!(display.contains("crypto failed"));
    }

    #[test]
    fn test_action_error_from_validation_error() {
        let val_err = ValidationError::new("test_code", "test message");
        let action_err: ActionError = val_err.into();

        match action_err {
            ActionError::ValidationFailed { code, message } => {
                assert_eq!(code, "test_code");
                assert_eq!(message, "test message");
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_action_error_from_workflow_error_validation() {
        let workflow_err = WorkflowError::ValidationError("validation failed".to_string());
        let action_err: ActionError = workflow_err.into();

        match action_err {
            ActionError::ValidationFailed { code, message } => {
                assert_eq!(code, "workflow_validation");
                assert_eq!(message, "validation failed");
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_action_error_from_workflow_error_resource_not_found() {
        let workflow_err = WorkflowError::ResourceNotFound("secret".to_string());
        let action_err: ActionError = workflow_err.into();

        match action_err {
            ActionError::ResourceNotFound { resource } => {
                assert_eq!(resource, "secret");
            }
            _ => panic!("Expected ResourceNotFound"),
        }
    }

    #[test]
    fn test_action_error_from_workflow_error_crypto() {
        let workflow_err = WorkflowError::CryptoError("crypto failed".to_string());
        let action_err: ActionError = workflow_err.into();

        match action_err {
            ActionError::CryptoError { message } => {
                assert_eq!(message, "crypto failed");
            }
            _ => panic!("Expected CryptoError"),
        }
    }

    #[test]
    fn test_action_error_from_workflow_error_partial_storage_failure() {
        let workflow_err = WorkflowError::PartialStorageFailure {
            capsule_tx_id: "tx_capsule_001".to_string(),
            successful_share_tx_ids: vec!["tx_share_0".to_string(), "tx_share_2".to_string()],
            failed_shares: vec![
                ("1".to_string(), "storage error".to_string()),
                ("3".to_string(), "timeout".to_string()),
            ],
            failed_count: 2,
            total_count: 4,
        };
        let action_err: ActionError = workflow_err.into();

        match action_err {
            ActionError::PartialStorageFailure {
                capsule_tx_id,
                successful_share_tx_ids,
                failed_shares,
                message,
            } => {
                assert_eq!(capsule_tx_id, "tx_capsule_001");
                assert_eq!(successful_share_tx_ids.len(), 2);
                assert_eq!(failed_shares.len(), 2);
                assert!(message.contains("2 of 4"));
            }
            _ => panic!("Expected PartialStorageFailure"),
        }
    }

    #[test]
    fn test_action_error_display_partial_storage_failure() {
        let err = ActionError::PartialStorageFailure {
            capsule_tx_id: "tx_001".to_string(),
            successful_share_tx_ids: vec!["tx_s1".to_string()],
            failed_shares: vec![("1".to_string(), "err".to_string())],
            message: "1 of 2 share storage operations failed".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Partial storage failure"));
        assert!(display.contains("1 of 2"));
    }
}
