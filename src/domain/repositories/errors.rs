//! Repository層の共通エラー型定義
//!
//! D-TPRESシステムのRepository操作で発生する可能性のあるエラーを定義。
//! 永続化層の実装詳細から独立したエラー型を提供。

use std::error::Error;
use thiserror::Error;

/// Repository操作の共通エラー型
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// エンティティが見つからない
    #[error("Entity not found: {id}")]
    NotFound { id: String },

    /// エンティティが既に存在する
    #[error("Entity already exists: {id}")]
    AlreadyExists { id: String },

    /// 楽観ロックエラー
    #[error("Concurrent modification detected for entity: {id}")]
    ConcurrentModification { id: String },

    /// バリデーションエラー
    #[error("Validation error: {message}")]
    ValidationError { message: String },

    /// ストレージエラー
    #[error("Storage error: {0}")]
    StorageError(#[from] Box<dyn Error + Send + Sync>),

    /// シリアライゼーションエラー
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// タイムアウト
    #[error("Operation timed out")]
    Timeout,

    /// その他のエラー
    #[error("Internal error: {0}")]
    Internal(String),
}

impl RepositoryError {
    /// ストレージエラーを作成
    pub fn storage_error<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::StorageError(Box::new(error))
    }

    /// 内部エラーを作成
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// バリデーションエラーを作成
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// エンティティが見つからないエラーを作成
    pub fn not_found<S: Into<String>>(id: S) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// エンティティが既に存在するエラーを作成
    pub fn already_exists<S: Into<String>>(id: S) -> Self {
        Self::AlreadyExists { id: id.into() }
    }

    /// 楽観ロックエラーを作成
    pub fn concurrent_modification<S: Into<String>>(id: S) -> Self {
        Self::ConcurrentModification { id: id.into() }
    }
}

/// Repository操作の結果型
pub type RepositoryResult<T> = Result<T, RepositoryError>;