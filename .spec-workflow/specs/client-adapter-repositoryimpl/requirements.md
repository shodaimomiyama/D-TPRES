# Requirements Document: Client Adapter Repository Implementation

## Introduction

このドキュメントは、D-TPRESクライアントライブラリ（`client/`）のAdapter層におけるRepository実装の要件を定義します。Repository層で定義されたRepository Interface（`SecretRepository`, `ShareCollectionRepository`, `CapsuleRepository`, `KFragRepository`, `CFragRepository`）を実装します。

この実装により、クライアントライブラリのドメインエンティティ（Secret, ShareCollection, Capsule, KFrag, CFrag）の永続化が可能になり、Phase 1（秘密分割）とPhase 3（秘密復元）の処理においてデータの保存・取得ができるようになります。

## Scope

### In Scope

- `client/src/adapter/repository_impl/` ディレクトリ内のRepository実装
  - `mod.rs` - モジュールエクスポート
  - `secret_impl.rs` - SecretRepository実装
  - `share_impl.rs` - ShareCollectionRepository実装
  - `capsule_impl.rs` - CapsuleRepository実装
  - `kfrag_impl.rs` - KFragRepository実装
  - `cfrag_impl.rs` - CFragRepository実装
- `client/src/adapter/errors.rs` - Adapter層エラー定義
- `client/src/adapter/mod.rs` - Adapterモジュールエクスポート

### Out of Scope（別PRで実装）

- `client/src/adapter/external/` ディレクトリ（ArweaveClient, AOClient等）
- 実際のArweaveネットワーク接続実装

## Technology Stack

### Arweave Client Library

本実装では、Arweave操作に [arweave-rs](https://github.com/nestdotland/arweave-rs) を使用することを前提とします。

- **ライブラリ**: `arweave-rs` (https://github.com/nestdotland/arweave-rs)
- **用途**: Arweaveへのデータ永続化・取得
- **注意**: 実際のArweaveClient実装は別PRで行うため、本PRではRepository実装がArweaveClientトレイト（インターフェース）に依存する形で設計

## Alignment with Product Vision

この機能は以下のプロダクト目標をサポートします：

1. **Arweave永続ストレージ**: 暗号化データと暗号カプセルをArweaveに不変に保存するという主要機能の実現基盤
2. **分散性優先**: Repository実装を通じて、中央集権的なストレージに依存しない分散型永続化を実現
3. **設計によるセキュリティ**: KFrag/CFragなどの機密暗号データの安全な永続化とメモリクリア
4. **ステートレス実行**: 明示的な状態ロード・永続化パターンを通じたステートレス設計の実現

## Requirements

### REQ-1: SecretRepository実装

**User Story:** As a システム開発者, I want Secretエンティティ（集約ルート）を永続化できるRepository実装, so that Phase 1で生成した秘密メタデータを安全に保存・復元できる

#### Acceptance Criteria

1. WHEN `save(&secret)` が呼び出される THEN システムはSecretエンティティをシリアライズして永続化 SHALL する
2. WHEN `find_by_id(&id)` が呼び出され、該当するSecretが存在する THEN システムは`Ok(Some(secret))` SHALL 返す
3. WHEN `find_by_id(&id)` が呼び出され、該当するSecretが存在しない THEN システムは`Ok(None)` SHALL 返す
4. WHEN `exists(&id)` が呼び出される THEN システムは該当Secretの存在有無をboolで SHALL 返す
5. WHEN `delete(&id)` が呼び出される THEN システムは該当Secretをソフト削除（論理削除マーカー）SHALL する
6. WHEN `find_by_ids(&ids)` が呼び出される THEN システムは存在するSecretのみを含むVec SHALL 返す

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-1.1 | Secretの保存と取得 | 有効なSecretエンティティ | `save`成功後、`find_by_id`で同一エンティティが取得できる |
| TC-1.2 | 存在しないSecretの検索 | 存在しないSecretId | `Ok(None)`が返される |
| TC-1.3 | Secretの存在確認（存在する） | 保存済みSecretId | `Ok(true)`が返される |
| TC-1.4 | Secretの存在確認（存在しない） | 未保存SecretId | `Ok(false)`が返される |
| TC-1.5 | Secretの削除 | 保存済みSecretId | 削除後、`find_by_id`で`Ok(None)`が返される |
| TC-1.6 | 存在しないSecretの削除 | 未保存SecretId | エラーなく`Ok(())`が返される |
| TC-1.7 | バッチ取得（全て存在） | 3つの保存済みSecretId | 3つのSecretを含むVecが返される |
| TC-1.8 | バッチ取得（一部存在） | 2つ存在、1つ未存在のId | 存在する2つのSecretのみを含むVecが返される |
| TC-1.9 | バッチ取得（空リスト） | 空のIdリスト | 空のVecが返される |

### REQ-2: ShareCollectionRepository実装

**User Story:** As a システム開発者, I want ShareCollection（Shamirシェアコレクション）を永続化できるRepository実装, so that Phase 1で生成した暗号化シェアを安全に保存・復元できる

#### Acceptance Criteria

1. WHEN `save(&share_collection)` が呼び出される THEN システムはShareCollectionを永続化 SHALL する
2. WHEN `find_by_id(&id)` が呼び出される THEN システムは対応するShareCollectionまたはNone SHALL 返す
3. WHEN `find_by_secret_id(&secret_id)` が呼び出される THEN システムは指定されたSecretに関連付けられたShareCollectionまたはNone SHALL 返す
4. IF ShareCollectionに`arweave_tx_id`が設定されている THEN システムはそのTXIDを永続化時に保持 SHALL する
5. WHEN `delete(&id)` が呼び出される THEN システムは該当ShareCollectionをソフト削除 SHALL する

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-2.1 | ShareCollectionの保存と取得 | 有効なShareCollection | `save`成功後、`find_by_id`で同一エンティティが取得できる |
| TC-2.2 | SecretIdでの検索（存在する） | 関連付けられたSecretId | 対応するShareCollectionが返される |
| TC-2.3 | SecretIdでの検索（存在しない） | 未関連付けのSecretId | `Ok(None)`が返される |
| TC-2.4 | ArweaveTxIdの保持確認 | txIdが設定されたShareCollection | 取得時にtxIdが保持されている |
| TC-2.5 | ShareCollectionの削除 | 保存済みShareCollectionId | 削除後、`find_by_id`で`Ok(None)`が返される |

### REQ-3: CapsuleRepository実装

**User Story:** As a システム開発者, I want Capsule（PREカプセル）を永続化できるRepository実装, so that Phase 1で生成したUmbralカプセルを保存し、Phase 3で復元できる

#### Acceptance Criteria

1. WHEN `save(&capsule)` が呼び出される THEN システムはCapsuleを永続化 SHALL する
2. WHEN `find_by_id(&id)` が呼び出される THEN システムは対応するCapsuleまたはNone SHALL 返す
3. WHEN `find_by_secret_id(&secret_id)` が呼び出される THEN システムは指定されたSecretに関連付けられたCapsuleまたはNone SHALL 返す
4. WHEN Capsuleが保存される THEN システムはcapsule_bytesとciphertext_bytesを完全に永続化 SHALL する

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-3.1 | Capsuleの保存と取得 | 有効なCapsule | `save`成功後、`find_by_id`で同一エンティティが取得できる |
| TC-3.2 | SecretIdでのCapsule検索 | 関連付けられたSecretId | 対応するCapsuleが返される |
| TC-3.3 | capsule_bytesの完全性確認 | バイナリデータを含むCapsule | 取得時にcapsule_bytesが完全に一致する |
| TC-3.4 | ciphertext_bytesの完全性確認 | バイナリデータを含むCapsule | 取得時にciphertext_bytesが完全に一致する |
| TC-3.5 | 存在しないCapsuleの検索 | 未保存CapsuleId | `Ok(None)`が返される |

### REQ-4: KFragRepository実装

**User Story:** As a システム開発者, I want KFrag（鍵フラグメント）を永続化できるRepository実装, so that Phase 1で生成した再暗号化鍵フラグメントを安全に保存できる

#### Acceptance Criteria

1. WHEN `save(&kfrag)` が呼び出される THEN システムはKFragを永続化 SHALL する
2. WHEN `find_by_id(&id)` が呼び出される THEN システムは対応するKFragまたはNone SHALL 返す
3. WHEN `find_by_secret_id(&secret_id)` が呼び出される THEN システムは指定されたSecretに関連付けられた全てのKFragをVecで SHALL 返す
4. WHEN `find_by_holder_index(&secret_id, holder_index)` が呼び出される THEN システムは特定のholderに対応するKFragまたはNone SHALL 返す
5. WHEN `delete_by_secret_id(&secret_id)` が呼び出される THEN システムは該当SecretのすべてのKFragを一括削除 SHALL する
6. IF KFragが永続化から読み込まれた後 THEN システムは使用後にメモリ上の機密データをZeroize SHALL する

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-4.1 | KFragの保存と取得 | 有効なKFrag | `save`成功後、`find_by_id`で同一エンティティが取得できる |
| TC-4.2 | SecretIdでの全KFrag取得 | 3つのKFragを持つSecretId | 3つのKFragを含むVecが返される |
| TC-4.3 | SecretIdでの取得（KFragなし） | KFragを持たないSecretId | 空のVecが返される |
| TC-4.4 | HolderIndexでの検索（存在する） | 有効なSecretIdとholder_index | 対応するKFragが返される |
| TC-4.5 | HolderIndexでの検索（存在しない） | 無効なholder_index | `Ok(None)`が返される |
| TC-4.6 | SecretIdによる一括削除 | 3つのKFragを持つSecretId | 削除後、`find_by_secret_id`で空のVecが返される |
| TC-4.7 | 一括削除の他Secret影響なし | 2つのSecretに紐づくKFrag | 指定したSecretのKFragのみ削除、他は保持 |

### REQ-5: CFragRepository実装

**User Story:** As a システム開発者, I want CFrag（再暗号化フラグメント）を永続化できるRepository実装, so that Phase 3で収集した再暗号化フラグメントを保存・取得できる

#### Acceptance Criteria

1. WHEN `save(&cfrag)` が呼び出される THEN システムはCFragを永続化 SHALL する
2. WHEN `find_by_id(&id)` が呼び出される THEN システムは対応するCFragまたはNone SHALL 返す
3. WHEN `find_by_secret_id(&secret_id)` が呼び出される THEN システムは指定されたSecretに関連付けられた全てのCFragをVecで SHALL 返す
4. WHEN `find_by_kfrag_id(&kfrag_id)` が呼び出される THEN システムは特定のKFragから生成されたCFragまたはNone SHALL 返す
5. WHEN `delete_by_secret_id(&secret_id)` が呼び出される THEN システムは該当SecretのすべてのCFragを一括削除 SHALL する
6. WHEN `count_by_secret_id(&secret_id)` が呼び出される THEN システムは該当Secretに関連付けられたCFragの数 SHALL 返す
7. IF CFragが永続化から読み込まれた後 THEN システムは使用後にメモリ上の機密データをZeroize SHALL する

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-5.1 | CFragの保存と取得 | 有効なCFrag | `save`成功後、`find_by_id`で同一エンティティが取得できる |
| TC-5.2 | SecretIdでの全CFrag取得 | 3つのCFragを持つSecretId | 3つのCFragを含むVecが返される |
| TC-5.3 | KFragIdでの検索（存在する） | 有効なKFragId | 対応するCFragが返される |
| TC-5.4 | KFragIdでの検索（存在しない） | 無効なKFragId | `Ok(None)`が返される |
| TC-5.5 | SecretIdによる一括削除 | 3つのCFragを持つSecretId | 削除後、`find_by_secret_id`で空のVecが返される |
| TC-5.6 | CFragカウント（存在する） | 3つのCFragを持つSecretId | `Ok(3)`が返される |
| TC-5.7 | CFragカウント（存在しない） | CFragを持たないSecretId | `Ok(0)`が返される |
| TC-5.8 | 閾値完了確認シナリオ | k=2, n=3のシナリオ | 2つのCFragが収集された時点でカウントが2を返す |

### REQ-6: Adapter層エラーハンドリング

**User Story:** As a システム開発者, I want Adapter層固有のエラーを適切に処理できる機能, so that 永続化操作やシリアライゼーションのエラーを明確に把握できる

#### Acceptance Criteria

1. WHEN 永続化操作が失敗する THEN システムは`AdapterError::StorageError`を SHALL 返す
2. WHEN シリアライゼーションが失敗する THEN システムは`AdapterError::SerializationError`を SHALL 返す
3. WHEN 存在しないエンティティへのアクセスが試みられる THEN システムは`Ok(None)`を SHALL 返す（エラーではなくNone）
4. IF DomainErrorへの変換が必要な場合 THEN AdapterErrorはDomainErrorに変換可能 SHALL である

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-6.1 | StorageErrorの発生 | 永続化操作が失敗するシナリオ | `AdapterError::StorageError`が返される |
| TC-6.2 | SerializationErrorの発生 | 不正なデータのシリアライズ | `AdapterError::SerializationError`が返される |
| TC-6.3 | 存在しないエンティティアクセス | 未保存のId | エラーではなく`Ok(None)`が返される |
| TC-6.4 | DomainErrorへの変換 | AdapterError | `Into<DomainError>`で変換可能 |
| TC-6.5 | エラーメッセージの安全性 | 機密データを含む操作のエラー | エラーメッセージに機密情報が含まれない |

### REQ-7: ArweaveClientトレイト依存

**User Story:** As a システム開発者, I want Repository実装がArweaveClientトレイトに依存する設計, so that 実際のArweaveClient実装を別PRで行い、テスト時にはモックを使用できる

#### Acceptance Criteria

1. WHEN Repository実装が永続化操作を行う THEN ArweaveClientトレイト経由で操作を委譲 SHALL する
2. IF ArweaveClientトレイトが定義されている THEN Repository実装はそのトレイトに依存 SHALL する
3. WHEN 単体テストを実行する THEN MockArweaveClientを使用してテスト可能 SHALL である
4. IF ArweaveClientトレイトが未定義の場合 THEN Repository実装内で最小限のトレイト定義 SHALL する（別PRで正式実装）

#### Test Cases

| ID | テストケース | 入力 | 期待結果 |
|----|------------|------|---------|
| TC-7.1 | MockArweaveClientでのテスト | モック実装 | Repository操作が正常に動作する |
| TC-7.2 | トレイト境界の確認 | Repository構造体 | ArweaveClientトレイトをジェネリック型として受け入れる |
| TC-7.3 | 依存性注入の確認 | 異なるArweaveClient実装 | 実装の差し替えが可能 |

## Non-Functional Requirements

### Code Architecture and Modularity

- **Single Responsibility Principle**: 各Repository実装ファイルは対応する1つのエンティティの永続化のみを担当する
- **Modular Design**: Repository実装は`client/src/adapter/repository_impl/`に配置し、エンティティごとに分離
- **Dependency Management**: Repository実装はDomain層、Repository層のみに依存し、UseCase層には依存しない
- **Clear Interfaces**: Repository層で定義されたtraitを正確に実装する
- **Dependency Inversion**: ArweaveClientトレイトへの依存により、実装の詳細から分離

### Performance

- **非同期操作**: すべてのRepository操作はasync/awaitを使用し、ブロッキングを回避
- **バッチ操作**: `find_by_ids`はバッチ取得を効率的に実行し、N+1問題を回避
- **キャッシュ考慮**: 将来的なキャッシュレイヤー追加を考慮した設計

### Security

- **Zeroize必須**: KFrag、CFragなどの機密データを含む構造体は、使用後にZeroizeでメモリクリア
- **エラーメッセージの安全性**: エラーメッセージに機密情報を含めない
- **定数時間操作**: 暗号データの比較には`subtle`クレートを使用

### Reliability

- **エラー伝播**: 永続化エラーは適切にDomainResultに変換して伝播
- **冪等性**: 同一エンティティの複数回saveは安全に処理（upsert動作）
- **トランザクション一貫性**: 複数エンティティの保存が必要な場合の一貫性考慮

### Usability

- **ドキュメント**: 各Repository実装にRustDocコメントを記載
- **デバッグ容易性**: tracingを使用した構造化ログの出力
- **テスト可能性**: MockArweaveClientを使用した単体テストが可能な設計
