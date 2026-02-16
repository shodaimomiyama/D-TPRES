# Adapter層

`client/src/adapter/`

## 概要

外部システム（Arweave、AO Network）のインフラストラクチャ実装とArweaveバックエンドのリポジトリ実装。

```
adapter/
├── errors.rs              — エラー定義
├── repository_impl/       — リポジトリ実装
│   ├── mod.rs             — ArweaveClientトレイト & タグヘルパー
│   ├── secret_impl.rs
│   ├── capsule_impl.rs
│   ├── kfrag_impl.rs
│   ├── cfrag_impl.rs
│   └── share_impl.rs
└── external/              — 外部システムクライアント
    ├── arweave/           — Arweaveゲートウェイクライアント
    ├── ao/                — AO Networkクライアント
    └── mock_ao/           — テスト用モックAOクライアント
```

## ArweaveClientトレイト (`repository_impl/mod.rs`)

```rust
#[async_trait]
pub trait ArweaveClient: Send + Sync {
    async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError>;
    async fn post(&self, data: &[u8], tags: Vec<Tag>) -> Result<String, AdapterError>;
    async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError>;
}
```

### タグシステム

**タグ名定数（`tag_names`）:**

| 定数 | 値 |
|-----|-----|
| `APP_NAME` | `"App-Name"` |
| `ENTITY_TYPE` | `"Entity-Type"` |
| `ENTITY_ID` | `"Entity-Id"` |
| `SECRET_ID` | `"Secret-Id"` |
| `DELETED` | `"Deleted"` |
| `KFRAG_ID` | `"KFrag-Id"` |
| `HOLDER_INDEX` | `"Holder-Index"` |

**タグ値定数（`tag_values`）:** `"FORMIX"`、`"Secret"`、`"Capsule"`、`"KFrag"`、`"CFrag"`、`"ShareCollection"`。

**ヘルパー関数:** `app_tag()`、`entity_type_tag()`、`secret_id_tag()` 等。

テスト用にインメモリ`HashMap`ストレージの`MockArweaveClient`を含む。

## リポジトリ実装

すべてのリポジトリは同一パターンに従う:

1. **StoredX構造体** — `from_entity()` / `to_entity()`変換付きbincodeシリアライズ可能な表現
2. **ArweaveXRepository\<C: ArweaveClient\>** — asyncトレイト実装
3. **論理削除** — エンティティは削除済みマークを付与（物理削除しない）
4. **タグベースクエリ** — メタデータ検索にArweaveタグを使用
5. **最新エントリロジック** — バージョン管理データは最新のtx_idを使用

機密性の高い格納型（`StoredKFrag`、`StoredCFrag`）は`Drop`と`Zeroize`を実装。

### 実装済みリポジトリ

| リポジトリ | エンティティ | 追加メソッド |
|-----------|------------|------------|
| `ArweaveSecretRepository` | Secret | — |
| `ArweaveCapsuleRepository` | Capsule | `find_by_secret_id` |
| `ArweaveKFragRepository` | KFrag | `find_by_secret_id`、`find_by_holder_index`、`delete_by_secret_id` |
| `ArweaveCFragRepository` | CFrag | `find_by_secret_id`、`find_by_kfrag_id`、`count_by_secret_id`、`delete_by_secret_id` |
| `ArweaveShareCollectionRepository` | ShareCollection | `find_by_secret_id` |

## AO Networkクライアント (`external/ao/`)

### AOClientトレイト (`client.rs`)

```rust
#[async_trait]
pub trait AOClient: Send + Sync {
    async fn execute(&self, process_id: &str, msg: ExecuteMsg)
        -> Result<AOResponse, AOCommunicationError>;
    async fn query(&self, process_id: &str, msg: QueryMsg)
        -> Result<Binary, AOCommunicationError>;
    async fn dry_run(&self, process_id: &str, msg: ExecuteMsg)
        -> Result<AOResponse, AOCommunicationError>;
}
```

### メッセージ型 (`message.rs`)

**ExecuteMsgバリアント:**

| バリアント | 説明 |
|----------|------|
| `DelegateKFrag` | Owner → Holder kFrag委譲 |
| `DelegateCapsule` | Owner → Holder カプセル委譲 |
| `SubmitKFrag` | Holder kFrag格納 |
| `SubmitCapsule` | Holder カプセル格納 |
| `Reencrypt` | 再暗号化トリガー |

**QueryMsgバリアント:**

| バリアント | 説明 |
|----------|------|
| `GetCFrag` | 暗号フラグメントを取得 |
| `ListCapsulesByKFrag` | kFragに対応するカプセル一覧 |

**レスポンス型:** `AOResponse`（success, data, events, message_id）、`Binary`（base64エンコードバイトデータ）。

`ValidateMessage`トレイトはIDフォーマット（ASCII英数字 + アンダースコア/ハイフン）とバイナリサイズ制限（最大128KB）を強制。

### ProductionAOClient (`production_client.rs`)

フィーチャーゲート（`production`）。実際のAO Network通信用HTTPクライアント。

### 設定 (`config.rs`)

ゲートウェイURL、タイムアウト、リトライ設定。

### ANS-104 DataItem (`data_item.rs`)

フィーチャーゲート（`production`）。AOメッセージ送信用のANS-104バンドルフォーマットを実装。

## MockAOClient (`external/mock_ao/client.rs`)

テスト用インメモリAOクライアント。

**ストレージ:** kfrags、cfrags（キー = `"{kfrag_id}:{capsule_id}"`）、capsules。

**テストヘルパー:**

| メソッド | 説明 |
|---------|------|
| `get_stored_kfrags()` | 格納済みkFragを参照 |
| `get_stored_cfrags()` | 格納済みcFragを参照 |
| `get_stored_capsules()` | 格納済みカプセルを参照 |
| `clear()` | 全ストレージをリセット |
| `inject_error()` | テスト用にエラーを注入 |
| `clear_error()` | 注入エラーを除去 |

メッセージバリデーション付きで完全な`AOClient`トレイトを実装。

## Arweaveクライアント (`external/arweave/`)

| モジュール | 説明 |
|----------|------|
| `client.rs` | `ArweaveClientImpl` — 本番用HTTPクライアント |
| `config.rs` | 設定（ゲートウェイURL、タイムアウト） |
| `wallet.rs` | JWKウォレット管理とRSA-PSS署名 |
| `deep_hash.rs` | Arweave DeepHash（SHA-384） |
| `merkle.rs` | `data_root`計算用Merkleツリー |
| `transaction.rs` | トランザクション構造と構築ロジック |

## エラー型 (`errors.rs`)

**AdapterError:**

- `StorageError`、`SerializationError`、`NotFound`、`ConnectionError`
- `NetworkError`、`ConfigurationError`、`ValidationError`、`QueryError`

**AOCommunicationError:**

- `ConnectionError`、`Timeout`、`ProcessNotFound`、`InvalidProcessId`
- `SerializationError`、`DeserializationError`、`ValidationError`
- `InsufficientCFrags`、`PartialSendFailure`、`ExecutionError`

変換チェーン: `AOCommunicationError` → `AdapterError` → `DomainError`。
