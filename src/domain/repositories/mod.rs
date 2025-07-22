//! Repository interfaces for D-TPRES domain layer
//!
//! このモジュールは、ドメイン層のRepository interfaceを定義します。
//! 依存性逆転原則（DIP）に従い、永続化の実装詳細から独立したインターフェースを提供。

use async_trait::async_trait;
use std::error::Error;

pub mod errors;
pub use errors::{RepositoryError, RepositoryResult};

pub mod process;
pub mod share;
pub mod capsule;
pub mod access_request;
pub mod rekey_fragment;
pub mod reencryption;
pub mod secret_details;

pub use process::ProcessEntityRepository;
pub use share::ShareEntityRepository;
pub use capsule::CapsuleEntityRepository;
pub use access_request::AccessRequestEntityRepository;
pub use rekey_fragment::RekeyFragmentEntityRepository;
pub use reencryption::ReencryptionEntityRepository;
pub use secret_details::SecretDetailsEntityRepository;

/// 汎用Repositoryインターフェース
///
/// # 型パラメータ
/// - `T`: Entity型
/// - `ID`: Entity識別子型
///
/// # 実装要件
/// - 全メソッドは非同期で実装
/// - エラー型は`std::error::Error`を実装
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// エラー型定義
    type Error: Error + Send + Sync + 'static;

    /// エンティティ作成
    ///
    /// # 引数
    /// - `entity`: 作成するエンティティ
    ///
    /// # 戻り値
    /// - `Ok(())`: 作成成功
    /// - `Err(Self::Error)`: 作成失敗
    ///
    /// # 実装注意点
    /// - 既存IDの場合はエラーを返す
    /// - トランザクション保証が必要
    async fn create(&self, entity: &T) -> Result<(), Self::Error>;

    /// IDによる検索
    ///
    /// # 引数
    /// - `id`: 検索対象のID
    ///
    /// # 戻り値
    /// - `Ok(Some(T))`: エンティティ発見
    /// - `Ok(None)`: エンティティ未発見
    /// - `Err(Self::Error)`: 検索エラー
    async fn find_by_id(&self, id: &ID) -> Result<Option<T>, Self::Error>;

    /// エンティティ更新
    ///
    /// # 引数
    /// - `entity`: 更新するエンティティ
    ///
    /// # 戻り値
    /// - `Ok(())`: 更新成功
    /// - `Err(Self::Error)`: 更新失敗
    ///
    /// # 実装注意点
    /// - 楽観ロック（version）のチェック
    /// - 存在しないIDの場合はエラー
    async fn update(&self, entity: &T) -> Result<(), Self::Error>;

    /// エンティティ削除
    ///
    /// # 引数
    /// - `id`: 削除対象のID
    ///
    /// # 戻り値
    /// - `Ok(())`: 削除成功
    /// - `Err(Self::Error)`: 削除失敗
    ///
    /// # 実装注意点
    /// - Arweaveでは論理削除として実装
    async fn delete(&self, id: &ID) -> Result<(), Self::Error>;

    /// 全件取得
    ///
    /// # 戻り値
    /// - `Ok(Vec<T>)`: 全エンティティのリスト
    /// - `Err(Self::Error)`: 取得エラー
    ///
    /// # 実装注意点
    /// - 大量データ対応（ページネーション推奨）
    /// - 削除済みエンティティは除外
    async fn find_all(&self) -> Result<Vec<T>, Self::Error>;

    /// 存在確認
    ///
    /// # 引数
    /// - `id`: 確認対象のID
    ///
    /// # 戻り値
    /// - `Ok(true)`: 存在する
    /// - `Ok(false)`: 存在しない
    /// - `Err(Self::Error)`: 確認エラー
    async fn exists(&self, id: &ID) -> Result<bool, Self::Error>;

    /// 件数取得
    ///
    /// # 戻り値
    /// - `Ok(usize)`: エンティティ総数
    /// - `Err(Self::Error)`: カウントエラー
    ///
    /// # 実装注意点
    /// - 削除済みエンティティは除外
    async fn count(&self) -> Result<usize, Self::Error>;

    /// IDリストによるバッチ取得
    ///
    /// # 引数
    /// - `ids`: 取得対象のIDリスト
    ///
    /// # 戻り値
    /// - `Ok(Vec<T>)`: 見つかったエンティティのリスト
    /// - `Err(Self::Error)`: 取得エラー
    ///
    /// # 実装注意点
    /// - 存在しないIDは結果に含めない
    /// - 順序は保証されない
    async fn find_by_ids(&self, ids: &[ID]) -> Result<Vec<T>, Self::Error>;

    /// バッチ作成
    ///
    /// # 引数
    /// - `entities`: 作成するエンティティのリスト
    ///
    /// # 戻り値
    /// - `Ok(())`: 全件作成成功
    /// - `Err(Self::Error)`: 作成失敗（部分的な成功は許可しない）
    ///
    /// # 実装注意点
    /// - トランザクション内で実行
    /// - 一つでも失敗したら全てロールバック
    async fn create_batch(&self, entities: &[T]) -> Result<(), Self::Error>;

    /// バッチ更新
    ///
    /// # 引数
    /// - `entities`: 更新するエンティティのリスト
    ///
    /// # 戻り値
    /// - `Ok(())`: 全件更新成功
    /// - `Err(Self::Error)`: 更新失敗
    ///
    /// # 実装注意点
    /// - 楽観ロックチェック
    /// - トランザクション保証
    async fn update_batch(&self, entities: &[T]) -> Result<(), Self::Error>;
}

/// ページネーション用の共通構造体
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// ページネーション対応Repository trait拡張
#[async_trait]
pub trait PageableRepository<T, ID>: Repository<T, ID>
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// ページネーション検索
    ///
    /// # 引数
    /// - `page`: ページ番号（0始まり）
    /// - `per_page`: 1ページあたりの件数
    ///
    /// # 戻り値
    /// - ページネーション結果
    async fn find_page(&self, page: usize, per_page: usize) -> Result<Page<T>, Self::Error>;
}