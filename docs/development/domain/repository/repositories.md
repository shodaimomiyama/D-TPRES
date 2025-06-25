---
title: "D-TPRES Repository Interface詳細設計"
description: "全Repository Interfaceの包括的定義とCRUD操作仕様"
tags: ["repository-pattern", "interface-design", "crud-operations", "domain-driven-design"]
status: "specification"
created: "2025-06-25"
author: "D-TPRES Development Team"
---

# D-TPRES Repository Interface詳細設計

## 1. 概要

本ドキュメントは、D-TPRESシステムにおける全てのRepository Interfaceの詳細設計を定義します。Repository InterfaceはEntityのCRUD操作とドメイン特化クエリを抽象化し、永続化詳細から独立したデータアクセス層を提供します。

## 2. 設計原則

### 2.1 基本原則
- **CRUD操作特化**: ビジネスロジックを含まない
- **抽象化**: 永続化実装詳細から独立
- **型安全性**: ジェネリクスによる型安全な操作
- **非同期対応**: async/awaitによる非同期処理
- **エラー処理**: Result型による明示的エラー処理

### 2.2 命名規約
- Interface名: `<Entity>Repository` 形式
- メソッド名: 動詞_前置詞_名詞 形式（例: `find_by_id`）
- 非同期メソッド: 全て`async fn`として定義

## 3. 基本Repository Trait

### 3.1 概要
全てのRepository実装の基底となる汎用CRUD操作を定義するtrait。

### 3.2 詳細定義

```rust
use async_trait::async_trait;
use std::error::Error;

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
}
```

## 4. ProcessEntityRepository Interface

### 4.1 概要
ProcessEntityのCRUD操作とプロセス固有のクエリ操作を定義。

### 4.2 詳細定義

```rust
use crate::domain::entity::{ProcessEntity, OwnerData, HolderData, RequesterData, PerformanceMetrics};

/// ProcessEntityリポジトリインターフェース
/// 
/// マルチロール対応プロセスの永続化操作を提供
#[async_trait]
pub trait ProcessEntityRepository: Repository<ProcessEntity, String> {
    /// 名前による検索
    /// 
    /// # 引数
    /// - `name`: プロセス名
    /// 
    /// # 使用例
    /// ```rust
    /// let process = repo.find_by_name("AliceOwnerProcess").await?;
    /// ```
    async fn find_by_name(&self, name: &str) -> Result<Option<ProcessEntity>, Self::Error>;
    
    /// アクティブロール別検索
    /// 
    /// # 引数
    /// - `role`: ロール名（"owner", "holder", "requester"）
    /// 
    /// # 戻り値
    /// 指定ロールを持つプロセスのリスト
    async fn find_by_active_role(&self, role: &str) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// Owner機能を持つプロセス検索
    /// 
    /// # 戻り値
    /// owner_dataが設定されているプロセスのリスト
    async fn find_processes_with_owner_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// Holder機能を持つプロセス検索
    /// 
    /// # 戻り値
    /// holder_dataが設定されているプロセスのリスト
    async fn find_processes_with_holder_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// Requester機能を持つプロセス検索
    /// 
    /// # 戻り値
    /// requester_dataが設定されているプロセスのリスト
    async fn find_processes_with_requester_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// 信頼性スコア順検索（Holder選択用）
    /// 
    /// # 引数
    /// - `limit`: 取得する最大件数
    /// 
    /// # 戻り値
    /// 信頼性スコア降順でソートされたHolderプロセス
    /// 
    /// # 使用シーン
    /// Phase 3でkFrag配布先Holderを選択する際に使用
    async fn find_holders_by_reliability_desc(&self, limit: usize) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// 負荷状況別検索（Holder選択用）
    /// 
    /// # 引数
    /// - `max_load`: 最大許容負荷
    /// 
    /// # 戻り値
    /// 負荷が閾値以下のHolderプロセス（負荷昇順）
    async fn find_holders_by_load_asc(&self, max_load: u64) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// 暗号操作対応別検索
    /// 
    /// # 引数
    /// - `operation`: 暗号操作名（"shamir_split", "pre_encrypt"等）
    /// 
    /// # 戻り値
    /// 指定操作をサポートするプロセスのリスト
    async fn find_by_crypto_operation(&self, operation: &str) -> Result<Vec<ProcessEntity>, Self::Error>;
    
    /// パフォーマンスメトリクス更新
    /// 
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `metrics`: 新しいメトリクス
    /// 
    /// # 実装注意点
    /// - 部分更新として実装
    /// - メトリクスのみ更新し、他フィールドは変更しない
    async fn update_performance_metrics(
        &self, 
        process_id: &str, 
        metrics: &PerformanceMetrics
    ) -> Result<(), Self::Error>;
    
    /// OwnerData更新
    /// 
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `owner_data`: 新しいOwnerData
    /// 
    /// # 使用シーン
    /// - 新しい秘密の管理開始時
    /// - kFrag生成完了時
    async fn update_owner_data(
        &self,
        process_id: &str,
        owner_data: &OwnerData,
    ) -> Result<(), Self::Error>;
    
    /// HolderData更新
    /// 
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `holder_data`: 新しいHolderData
    /// 
    /// # 使用シーン
    /// - kFrag受信時
    /// - 再暗号化完了時
    async fn update_holder_data(
        &self,
        process_id: &str,
        holder_data: &HolderData,
    ) -> Result<(), Self::Error>;
    
    /// RequesterData更新
    /// 
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `requester_data`: 新しいRequesterData
    /// 
    /// # 使用シーン
    /// - アクセス要求作成時
    /// - cFrag収集完了時
    async fn update_requester_data(
        &self,
        process_id: &str,
        requester_data: &RequesterData,
    ) -> Result<(), Self::Error>;
}
```

## 5. ShareEntityRepository Interface

### 5.1 概要
ShareEntityのCRUD操作とShamir Secret Sharing関連のクエリ操作を定義。

### 5.2 詳細定義

```rust
use crate::domain::entity::ShareEntity;

/// ShareEntityリポジトリインターフェース
/// 
/// Shamirデータシェアの永続化操作を提供
#[async_trait]
pub trait ShareEntityRepository: Repository<ShareEntity, String> {
    /// データID別検索
    /// 
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// 
    /// # 戻り値
    /// 同一データグループの全シェア
    async fn find_by_data_id(&self, data_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;
    
    /// 秘密ID別検索
    /// 
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// 
    /// # 戻り値
    /// 同一秘密から生成された全シェア
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;
    
    /// オーナー別検索
    /// 
    /// # 引数
    /// - `owner_public_key`: オーナー公開鍵
    /// 
    /// # 戻り値
    /// 指定オーナーが所有する全シェア
    async fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<ShareEntity>, Self::Error>;
    
    /// 閾値インデックス指定検索
    /// 
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// - `index`: 閾値インデックス（1からn）
    /// 
    /// # 戻り値
    /// 指定インデックスのシェア
    async fn find_by_threshold_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<ShareEntity>, Self::Error>;
    
    /// Shamir再構築用シェア収集
    /// 
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// - `threshold`: 必要な閾値数（k）
    /// 
    /// # 戻り値
    /// 再構築に必要な最小数のシェア
    /// 
    /// # 実装注意点
    /// - 利用可能なシェアから最適なk個を選択
    /// - 最終アクセス時刻等を考慮した選択
    async fn collect_shares_for_reconstruction(
        &self,
        secret_id: &str,
        threshold: u8,
    ) -> Result<Vec<ShareEntity>, Self::Error>;
    
    /// 完全性検証用データ取得
    /// 
    /// # 引数
    /// - `share_id`: シェア識別子
    /// 
    /// # 戻り値
    /// - `true`: 整合性検証成功
    /// - `false`: 整合性検証失敗
    /// 
    /// # 実装注意点
    /// - integrity_hashを使用した検証
    async fn verify_share_integrity(&self, share_id: &str) -> Result<bool, Self::Error>;
    
    /// 最終アクセス時刻更新
    /// 
    /// # 引数
    /// - `share_id`: シェア識別子
    /// - `accessed_at`: アクセス時刻
    /// 
    /// # 使用シーン
    /// - シェア読み込み時
    /// - 再構築処理時
    async fn update_last_accessed(&self, share_id: &str, accessed_at: u64) -> Result<(), Self::Error>;
    
    /// 古いシェア検索（クリーンアップ用）
    /// 
    /// # 引数
    /// - `threshold_date`: 閾値日時（Unix timestamp）
    /// 
    /// # 戻り値
    /// 指定日時より古いシェアのリスト
    async fn find_older_than(&self, threshold_date: u64) -> Result<Vec<ShareEntity>, Self::Error>;
}
```

## 6. CapsuleEntityRepository Interface

### 6.1 概要
CapsuleEntityのCRUD操作とProxy Re-Encryption関連のクエリ操作を定義。

### 6.2 詳細定義

```rust
use crate::domain::entity::CapsuleEntity;

/// CapsuleEntityリポジトリインターフェース
/// 
/// PREカプセルの永続化操作を提供
#[async_trait]
pub trait CapsuleEntityRepository: Repository<CapsuleEntity, String> {
    /// データID別検索
    /// 
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// 
    /// # 戻り値
    /// 同一データグループの全カプセル
    async fn find_by_data_id(&self, data_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;
    
    /// 秘密ID別検索
    /// 
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// 
    /// # 戻り値
    /// 同一秘密に関連する全カプセル
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;
    
    /// カプセルインデックス指定検索
    /// 
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// - `index`: カプセルインデックス
    /// 
    /// # 戻り値
    /// 指定インデックスのカプセル
    /// 
    /// # 使用シーン
    /// Phase 4で特定のShareに対応するCapsuleを取得
    async fn find_by_capsule_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<CapsuleEntity>, Self::Error>;
    
    /// オーナー別検索
    /// 
    /// # 引数
    /// - `owner_public_key`: オーナー公開鍵
    /// 
    /// # 戻り値
    /// 指定オーナーが所有する全カプセル
    async fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<CapsuleEntity>, Self::Error>;
    
    /// 対応する暗号文ID別検索
    /// 
    /// # 引数
    /// - `ciphertext_id`: 暗号文識別子（ShareEntity ID）
    /// 
    /// # 戻り値
    /// 対応するカプセル
    async fn find_by_ciphertext_id(&self, ciphertext_id: &str) -> Result<Option<CapsuleEntity>, Self::Error>;
    
    /// データサイズ範囲検索
    /// 
    /// # 引数
    /// - `min_size`: 最小サイズ（バイト）
    /// - `max_size`: 最大サイズ（バイト）
    /// 
    /// # 戻り値
    /// サイズ範囲内のカプセルのリスト
    /// 
    /// # 使用シーン
    /// ストレージ管理、統計分析
    async fn find_by_size_range(
        &self,
        min_size: usize,
        max_size: usize,
    ) -> Result<Vec<CapsuleEntity>, Self::Error>;
}
```

## 7. AccessRequestEntityRepository Interface

### 7.1 概要
AccessRequestEntityのCRUD操作とアクセス制御関連のクエリ操作を定義。

### 7.2 詳細定義

```rust
use crate::domain::entity::{AccessRequestEntity, EvmVerificationData, ProofPkgData};

/// AccessRequestEntityリポジトリインターフェース
/// 
/// アクセス要求の永続化操作を提供
#[async_trait]
pub trait AccessRequestEntityRepository: Repository<AccessRequestEntity, String> {
    /// 要求者別検索
    /// 
    /// # 引数
    /// - `requester_process_id`: 要求者プロセスID
    /// 
    /// # 戻り値
    /// 指定要求者の全アクセス要求
    async fn find_by_requester(&self, requester_process_id: &str) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// 対象データID別検索
    /// 
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// 
    /// # 戻り値
    /// 指定データへの全アクセス要求
    async fn find_by_target_data_id(&self, data_id: &str) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// 対象秘密ID別検索
    /// 
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// 
    /// # 戻り値
    /// 指定秘密への全アクセス要求
    async fn find_by_target_secret_id(&self, secret_id: &str) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// 状態別検索
    /// 
    /// # 引数
    /// - `status`: 状態（"pending", "evm_verified", "approved", "rejected", "completed"）
    /// 
    /// # 戻り値
    /// 指定状態のアクセス要求リスト
    async fn find_by_status(&self, status: &str) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// アクセス者公開鍵別検索
    /// 
    /// # 引数
    /// - `public_key`: アクセス者公開鍵（pkA）
    /// 
    /// # 戻り値
    /// 指定公開鍵に関連する全アクセス要求
    async fn find_by_accessor_public_key(&self, public_key: &[u8]) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// EVM検証済み要求検索
    /// 
    /// # 戻り値
    /// EVM検証が完了したアクセス要求のリスト
    /// 
    /// # 使用シーン
    /// Phase 3への移行対象となる要求の抽出
    async fn find_evm_verified_requests(&self) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// タイムアウト要求検索
    /// 
    /// # 引数
    /// - `current_time`: 現在時刻（Unix timestamp）
    /// 
    /// # 戻り値
    /// タイムアウトしたアクセス要求のリスト
    async fn find_timed_out_requests(&self, current_time: u64) -> Result<Vec<AccessRequestEntity>, Self::Error>;
    
    /// 状態更新
    /// 
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `status`: 新しい状態
    /// 
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    /// - タイムスタンプの自動更新
    async fn update_status(&self, request_id: &str, status: &str) -> Result<(), Self::Error>;
    
    /// EVM検証結果更新
    /// 
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `verification_data`: EVM検証データ
    /// 
    /// # 使用シーン
    /// Phase 2でEVM検証が完了した際
    async fn update_evm_verification(
        &self,
        request_id: &str,
        verification_data: &EvmVerificationData,
    ) -> Result<(), Self::Error>;
    
    /// ProofPkg設定
    /// 
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `proof_pkg`: ProofPkgデータ
    /// 
    /// # 使用シーン
    /// elciaoによるProofPkg生成完了時
    async fn set_proof_pkg(
        &self,
        request_id: &str,
        proof_pkg: &ProofPkgData,
    ) -> Result<(), Self::Error>;
    
    /// 完了マーキング
    /// 
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `completed_at`: 完了時刻
    /// 
    /// # 使用シーン
    /// Phase 5で復号が成功した際
    async fn mark_completed(&self, request_id: &str, completed_at: u64) -> Result<(), Self::Error>;
}
```

## 8. RekeyFragmentEntityRepository Interface

### 8.1 概要
RekeyFragmentEntityのCRUD操作と再暗号化キー管理関連のクエリ操作を定義。

### 8.2 詳細定義

```rust
use crate::domain::entity::RekeyFragmentEntity;

/// RekeyFragmentEntityリポジトリインターフェース
/// 
/// 再暗号化キーフラグメントの永続化操作を提供
#[async_trait]
pub trait RekeyFragmentEntityRepository: Repository<RekeyFragmentEntity, String> {
    /// 秘密ID別検索
    /// 
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// 
    /// # 戻り値
    /// 指定秘密に関連する全kFrag
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// アクセス要求ID別検索
    /// 
    /// # 引数
    /// - `request_id`: アクセス要求識別子
    /// 
    /// # 戻り値
    /// 指定要求に関連する全kFrag
    async fn find_by_access_request_id(&self, request_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// アクセス制御条件別検索
    /// 
    /// # 引数
    /// - `condition`: アクセス制御条件
    /// 
    /// # 戻り値
    /// 指定条件に関連する全kFrag
    async fn find_by_access_control_condition(
        &self,
        condition: &str,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// Holder別検索
    /// 
    /// # 引数
    /// - `holder_id`: HolderプロセスID
    /// 
    /// # 戻り値
    /// 指定Holderが保持する全kFrag
    async fn find_by_holder(&self, holder_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// 状態別検索
    /// 
    /// # 引数
    /// - `status`: 状態（"created", "distributed", "active", "consumed", "expired"）
    /// 
    /// # 戻り値
    /// 指定状態のkFragリスト
    async fn find_by_status(&self, status: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// アクティブフラグメント検索
    /// 
    /// # 引数
    /// - `access_control_condition`: アクセス制御条件
    /// - `accessor_public_key`: アクセス者公開鍵
    /// 
    /// # 戻り値
    /// 使用可能なアクティブkFragのリスト
    /// 
    /// # 使用シーン
    /// Phase 4で再暗号化可能なkFragを探す際
    async fn find_active_fragments_for_condition(
        &self,
        access_control_condition: &str,
        accessor_public_key: &[u8],
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// 期限切れフラグメント検索
    /// 
    /// # 引数
    /// - `current_time`: 現在時刻
    /// 
    /// # 戻り値
    /// 期限切れのkFragリスト
    async fn find_expired_fragments(&self, current_time: u64) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;
    
    /// 状態更新
    /// 
    /// # 引数
    /// - `fragment_id`: フラグメント識別子
    /// - `status`: 新しい状態
    /// 
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    async fn update_status(&self, fragment_id: &str, status: &str) -> Result<(), Self::Error>;
    
    /// 配布マーキング
    /// 
    /// # 引数
    /// - `fragment_id`: フラグメント識別子
    /// - `distributed_at`: 配布時刻
    /// 
    /// # 使用シーン
    /// Phase 3でHolderへの配布が完了した際
    async fn mark_distributed(&self, fragment_id: &str, distributed_at: u64) -> Result<(), Self::Error>;
    
    /// Holder負荷分散情報取得
    /// 
    /// # 戻り値
    /// HolderIDと保持kFrag数のタプルリスト
    /// 
    /// # 使用シーン
    /// Phase 3で新規kFrag配布先を決定する際
    async fn get_holder_load_distribution(&self) -> Result<Vec<(String, u64)>, Self::Error>;
}
```

## 9. ReencryptionEntityRepository Interface

### 9.1 概要
ReencryptionEntityのCRUD操作とプロキシ再暗号化関連のクエリ操作を定義。

### 9.2 詳細定義

```rust
use crate::domain::entity::{ReencryptionEntity, CFragData};

/// ReencryptionEntityリポジトリインターフェース
/// 
/// プロキシ再暗号化処理の永続化操作を提供
#[async_trait]
pub trait ReencryptionEntityRepository: Repository<ReencryptionEntity, String> {
    /// アクセス要求ID別検索
    /// 
    /// # 引数
    /// - `request_id`: アクセス要求識別子
    /// 
    /// # 戻り値
    /// 指定要求に関連する全再暗号化処理
    async fn find_by_access_request_id(&self, request_id: &str) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// 要求者プロセスID別検索
    /// 
    /// # 引数
    /// - `process_id`: 要求者プロセスID
    /// 
    /// # 戻り値
    /// 指定要求者の全再暗号化処理
    async fn find_by_requester_process_id(&self, process_id: &str) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// 対象カプセルID別検索
    /// 
    /// # 引数
    /// - `capsule_id`: カプセル識別子
    /// 
    /// # 戻り値
    /// 指定カプセルに対する再暗号化処理
    async fn find_by_target_capsule_id(&self, capsule_id: &str) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// 状態別検索
    /// 
    /// # 引数
    /// - `status`: 状態（"initiated", "collecting", "threshold_met", "completed", "failed"）
    /// 
    /// # 戻り値
    /// 指定状態の再暗号化処理リスト
    async fn find_by_status(&self, status: &str) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// アクティブ再暗号化検索
    /// 
    /// # 戻り値
    /// 処理中の再暗号化リスト
    /// 
    /// # 使用シーン
    /// 定期的な状態確認、タイムアウト処理
    async fn find_active_reencryptions(&self) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// タイムアウト再暗号化検索
    /// 
    /// # 引数
    /// - `current_time`: 現在時刻
    /// 
    /// # 戻り値
    /// タイムアウトした再暗号化処理のリスト
    async fn find_timed_out_reencryptions(&self, current_time: u64) -> Result<Vec<ReencryptionEntity>, Self::Error>;
    
    /// 状態更新
    /// 
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `status`: 新しい状態
    /// 
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    /// - 閾値達成時の自動状態更新
    async fn update_status(&self, reencryption_id: &str, status: &str) -> Result<(), Self::Error>;
    
    /// cFrag追加
    /// 
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `cfrag`: 新しいcFragデータ
    /// 
    /// # 使用シーン
    /// Phase 4でHolderからcFragを受信した際
    /// 
    /// # 実装注意点
    /// - 重複チェック
    /// - 閾値達成チェック
    async fn add_cfrag(
        &self,
        reencryption_id: &str,
        cfrag: &CFragData,
    ) -> Result<(), Self::Error>;
    
    /// 閾値達成チェック
    /// 
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// 
    /// # 戻り値
    /// - `true`: 必要数のcFragが収集済み
    /// - `false`: まだ不足
    async fn check_threshold_met(&self, reencryption_id: &str) -> Result<bool, Self::Error>;
    
    /// 完了マーキング
    /// 
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `completed_at`: 完了時刻
    /// 
    /// # 使用シーン
    /// Phase 4で閾値数のcFragが収集できた際
    async fn mark_completed(&self, reencryption_id: &str, completed_at: u64) -> Result<(), Self::Error>;
}
```

## 10. エラー処理設計

### 10.1 共通エラー型

```rust
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
```

## 11. 実装ガイドライン

### 11.1 Repository実装の原則

```rust
// ✅ 正しい実装例
#[async_trait]
impl ShareEntityRepository for ShareEntityRepositoryImpl {
    async fn find_by_data_id(&self, data_id: &str) -> Result<Vec<ShareEntity>, Self::Error> {
        // 純粋なデータアクセスロジックのみ
        let shares = self.storage.query_by_tag("data_id", data_id).await?;
        Ok(shares)
    }
}

// ❌ 間違った実装例（ビジネスロジックを含む）
#[async_trait]
impl ShareEntityRepository for ShareEntityRepositoryImpl {
    async fn find_valid_shares(&self, data_id: &str) -> Result<Vec<ShareEntity>, Self::Error> {
        let shares = self.storage.query_by_tag("data_id", data_id).await?;
        // NG: ビジネスロジック
        let valid_shares = shares.into_iter()
            .filter(|s| s.threshold_index <= s.shamir_threshold)
            .collect();
        Ok(valid_shares)
    }
}
```

### 11.2 トランザクション処理

```rust
// Repository実装でのトランザクション例
pub struct TransactionalRepository<T> {
    inner: T,
}

impl<T> TransactionalRepository<T> {
    pub async fn with_transaction<F, R>(&self, f: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&T) -> Future<Output = Result<R, RepositoryError>>,
    {
        // トランザクション開始
        let tx = self.begin_transaction().await?;
        
        match f(&self.inner).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }
}
```

### 11.3 ページネーション

```rust
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
pub trait PageableRepository<T, ID>: Repository<T, ID> {
    async fn find_page(
        &self, 
        page: usize, 
        per_page: usize
    ) -> Result<Page<T>, Self::Error>;
}
```

## 12. テスト戦略

### 12.1 Repositoryのモックテスト

```rust
use mockall::*;

// モック生成
mock! {
    ProcessRepo {}
    
    #[async_trait]
    impl ProcessEntityRepository for ProcessRepo {
        async fn find_by_name(&self, name: &str) -> Result<Option<ProcessEntity>, RepositoryError>;
        // ... 他のメソッド
    }
}

// テスト例
#[tokio::test]
async fn test_find_process_by_name() {
    let mut mock = MockProcessRepo::new();
    mock.expect_find_by_name()
        .with(eq("TestProcess"))
        .times(1)
        .returning(|_| Ok(Some(ProcessEntity {
            process_id: "test_001".to_string(),
            process_name: "TestProcess".to_string(),
            // ...
        })));
    
    let result = mock.find_by_name("TestProcess").await;
    assert!(result.is_ok());
}
```

## 13. まとめ

D-TPRES Repository Interface設計は以下の特徴を持ちます：

1. **CRUD操作特化**: ビジネスロジックを含まない純粋なデータアクセス
2. **型安全性**: ジェネリクスとtraitによる型安全な設計
3. **非同期対応**: 全操作がasync/awaitに対応
4. **テスタビリティ**: モック可能なinterface設計
5. **拡張性**: 新しいクエリメソッドの追加が容易

これらのRepository Interfaceは、Infrastructure層で具体的に実装され、Arweaveストレージとの連携を提供します。

---

**Document Status**: Repository Interface Specification  
**Version**: 1.0  
**Next Steps**: Repository Implementation設計（repository_implementations.md参照）