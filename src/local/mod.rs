//! ローカル環境での処理モジュール
//!
//! O-Browser（ローカル）での暗号化処理を提供します。

pub mod owner_local;

pub use owner_local::{
    OwnerLocalProcessor, OwnerLocalParams, OwnerLocalResult,
    EncryptedShare, SerializableKeyFragment
};