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
