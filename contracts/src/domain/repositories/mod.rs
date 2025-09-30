//! D-TPRESドメイン層のRepositoryインターフェース
//!
//! このモジュールは、ドメイン層のRepository interfaceを定義します。
//! 依存性逆転原則（DIP）に従い、永続化の実装詳細から独立したインターフェースを提供。

use std::error::Error;

pub mod access_request;
pub mod capsule;
pub mod process;
pub mod reencryption;
pub mod rekey_fragment;
pub mod secret_details;
pub mod share;

pub use access_request::AccessRequestEntityRepository;
pub use capsule::CapsuleEntityRepository;
pub use process::ProcessEntityRepository;
pub use reencryption::ReencryptionEntityRepository;
pub use rekey_fragment::RekeyFragmentEntityRepository;
pub use secret_details::SecretDetailsEntityRepository;
pub use share::ShareEntityRepository;

/// エンティティCRUD操作の汎用Repositoryインターフェース
/// AO環境では同期実行が必須
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    type Error: Error + Send + Sync + 'static;

    // Arweaveの不変性のため重複ID確認が必要
    fn create(&self, entity: &T) -> Result<(), Self::Error>;

    fn find_by_id(&self, id: &ID) -> Result<Option<T>, Self::Error>;

    // 同時更新対策のためバージョンチェック付き楽観ロック
    fn update(&self, entity: &T) -> Result<(), Self::Error>;

    // Arweaveストレージの不変性により論理削除のみ
    fn delete(&self, id: &ID) -> Result<(), Self::Error>;

    // AO環境では大規模データセットのためページネーション推奨
    fn find_all(&self) -> Result<Vec<T>, Self::Error>;

    fn exists(&self, id: &ID) -> Result<bool, Self::Error>;

    // 論理削除されたエンティティを除外
    fn count(&self) -> Result<usize, Self::Error>;

    // AOステートレス環境の効率化のためバッチ処理
    fn find_by_ids(&self, ids: &[ID]) -> Result<Vec<T>, Self::Error>;

    // 一貫性保証のためアトミックなバッチ作成
    fn create_batch(&self, entities: &[T]) -> Result<(), Self::Error>;

    // 各エンティティに楽観ロックを適用したバッチ更新
    fn update_batch(&self, entities: &[T]) -> Result<(), Self::Error>;
}

/// 大規模データセット処理用のページネーションサポート
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// メモリ効率的なクエリのためのページング可能なRepository拡張
pub trait PageableRepository<T, ID>: Repository<T, ID>
where
    T: Send + Sync,
    ID: Send + Sync,
{
    fn find_page(&self, page: usize, per_page: usize) -> Result<Page<T>, Self::Error>;
}
