//! Service層
//!
//! ビジネスロジックを実装するサービス層です。
//! Core ServiceとWorkflow Serviceの2層構造を採用しています。

pub mod core;
pub mod error;

pub use error::{BusinessException, ServiceError, ServiceResult, SystemException};

#[cfg(feature = "workflow")]
pub mod workflow;