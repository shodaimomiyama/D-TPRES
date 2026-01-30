## Summary

ArweaveClient の本番実装 (`ArweaveClientImpl`) を追加しました。これにより、実際の Arweave ネットワークとの通信が可能になります。

### 主な機能
- トランザクションデータの取得 (`get`)
- GraphQL によるトランザクション検索 (`query`)
- トランザクション投稿 (`post`) ※署名機能は未完成
- 環境変数からのウォレット読み込み (`ARWEAVE_WALLET_JWK`)

---

# 技術背景ドキュメント

## 1. Arweave とは

### 1.1 概要

Arweave は**永久保存型の分散ストレージネットワーク**です。一度データを保存すると、理論上永久に保持されます。

```
従来のストレージ:
  クライアント → AWS S3 → データ（月額課金、削除可能）

Arweave:
  クライアント → Arweave Network → データ（一回払い、永久保存）
```

### 1.2 トランザクションの仕組み

Arweaveでは、すべてのデータ保存は「トランザクション」として記録されます。

```
┌─────────────────────────────────────────────────────────────┐
│                    Arweave Transaction                      │
├─────────────────────────────────────────────────────────────┤
│  id:        "abc123..."     ← トランザクションID (43文字)    │
│  owner:     "xyz789..."     ← 送信者の公開鍵                │
│  data:      "SGVsbG8..."    ← Base64URL エンコードされたデータ│
│  tags:      [               ← メタデータ（検索に使用）        │
│    { name: "App-Name", value: "D-TPRES" },                  │
│    { name: "Content-Type", value: "application/json" }      │
│  ]                                                          │
│  signature: "sig..."        ← RSA-PSS 署名                  │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Arweave の API エンドポイント

Arweave Gateway は REST API を提供します：

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/tx/{id}/data` | GET | トランザクションのデータ取得 |
| `/tx` | POST | 新規トランザクション投稿 |
| `/graphql` | POST | GraphQL クエリ |
| `/tx/{id}` | GET | トランザクションメタデータ取得 |
| `/price/{bytes}` | GET | 保存料金の見積もり |

---

## 2. reqwest クレート

### 2.1 reqwest とは

`reqwest` は Rust の**非同期 HTTP クライアントライブラリ**です。REST API との通信に使用します。

```rust
// Cargo.toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
```

### 2.2 基本的な使い方

```rust
use reqwest::Client;

// クライアント作成（再利用推奨）
let client = Client::new();

// GET リクエスト
let response = client
    .get("https://arweave.net/tx/abc123/data")
    .send()
    .await?;

let body = response.text().await?;

// POST リクエスト（JSON）
let response = client
    .post("https://arweave.net/tx")
    .json(&my_transaction)  // 自動的にJSON変換
    .send()
    .await?;
```

### 2.3 本実装での使い方

```rust
// client.rs:146-163
pub struct ArweaveClientImpl {
    config: ArweaveClientConfig,
    http_client: Client,        // ← reqwest::Client
    wallet: Option<ArweaveWallet>,
}

impl ArweaveClientImpl {
    pub fn new(config: ArweaveClientConfig) -> Result<Self, AdapterError> {
        // Builder パターンでクライアント構築
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs()))  // タイムアウト設定
            .build()?;

        Ok(Self {
            config,
            http_client,
            wallet: None,
        })
    }
}
```

### 2.4 GET リクエストの実装詳細

```rust
// client.rs:215-276 - get メソッド
async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError> {
    // 1. URL構築
    let url = format!("{}/tx/{}/data", self.config.gateway_url(), tx_id);
    // 例: "https://arweave.net/tx/BNttzDav3jHVnNiV7nYbQv-GY0HQ-4XXsdkE5K9ylHQ/data"

    // 2. リトライループ
    loop {
        // 3. HTTP GET リクエスト送信
        match self.http_client.get(&url).send().await {
            Ok(response) => {
                // 4. ステータスコード確認
                if response.status() == reqwest::StatusCode::NOT_FOUND {
                    return Ok(None);  // 404 = データなし
                }

                if response.status().is_success() {
                    // 5. レスポンスボディ取得
                    let bytes = response.bytes().await?;

                    // 6. Base64URL デコード（Arweave特有）
                    let decoded = base64url_decode(&bytes)?;

                    return Ok(Some(decoded));
                }

                // エラー時はリトライ
            }
            Err(e) => {
                // ネットワークエラー時もリトライ
            }
        }

        // 7. 指数バックオフで待機
        retries += 1;
        tokio::time::sleep(Duration::from_millis(backoff_ms * (1 << retries))).await;
        // 1回目: 2000ms, 2回目: 4000ms, 3回目: 8000ms ...
    }
}
```

**データの流れ:**

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ArweaveClientImpl::get("BNttzDav...")                                   │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  reqwest::Client::get("https://arweave.net/tx/BNttzDav.../data")         │
│    → HTTP GET リクエスト送信                                              │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ (インターネット経由)
┌──────────────────────────────────────────────────────────────────────────┐
│  Arweave Gateway (arweave.net)                                           │
│    → トランザクションID から永久ストレージのデータを検索                    │
│    → Base64URL エンコードされたデータを返す                                │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  Response: "PGh0bWwgbGFuZz0iZW4tR0Ii..."  (Base64URL)                    │
│    → base64url_decode() でバイト列に変換                                  │
│    → "<html lang=\"en-GB\"..." (元のHTML)                                │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2.5 POST リクエストの実装詳細

```rust
// client.rs:326-361 - post メソッド（トランザクション投稿部分）
let url = format!("{}/tx", self.config.gateway_url());

loop {
    match self.http_client
        .post(&url)
        .json(&tx)           // ArweaveTransaction を JSON にシリアライズ
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                // 成功時: トランザクションIDを取得
                let tx_id = response.text().await?;
                return Ok(tx_id);
            }
        }
        Err(e) => { /* リトライ */ }
    }

    // 指数バックオフ
    retries += 1;
    tokio::time::sleep(...).await;
}
```

---

## 3. GraphQL と Arweave

### 3.1 GraphQL とは

GraphQL は**クエリ言語**です。REST API と違い、必要なデータだけを柔軟に取得できます。

```
REST API の場合:
  GET /users/123          → ユーザー情報全部
  GET /users/123/posts    → 投稿一覧全部
  GET /users/123/friends  → 友達一覧全部
  （3回のリクエスト、不要なデータも含む）

GraphQL の場合:
  POST /graphql
  query {
    user(id: 123) {
      name              ← 名前だけ
      posts { title }   ← 投稿のタイトルだけ
    }
  }
  （1回のリクエスト、必要なデータのみ）
```

### 3.2 Arweave GraphQL の用途

Arweave では、**タグによるトランザクション検索**に GraphQL を使用します。

```graphql
# D-TPRES アプリのトランザクションを検索
{
  transactions(
    tags: [
      { name: "App-Name", values: ["D-TPRES"] }
      { name: "Type", values: ["Capsule"] }
    ]
    first: 100
  ) {
    edges {
      node {
        id          # トランザクションID
      }
      cursor        # ページネーション用カーソル
    }
    pageInfo {
      hasNextPage   # 次のページがあるか
    }
  }
}
```

### 3.3 GraphQL レスポンスの構造

```json
{
  "data": {
    "transactions": {
      "edges": [
        {
          "node": { "id": "abc123..." },
          "cursor": "WyIyMDI0LTAxLTAxIiwgMV0="
        },
        {
          "node": { "id": "def456..." },
          "cursor": "WyIyMDI0LTAxLTAyIiwgMl0="
        }
      ],
      "pageInfo": {
        "hasNextPage": true
      }
    }
  }
}
```

### 3.4 Rust での GraphQL レスポンス型定義

```rust
// client.rs:14-67 - GraphQL レスポンス型

// 最上位のレスポンス
#[derive(Debug, Deserialize)]
pub(crate) struct GraphQLResponse {
    pub data: Option<TransactionsData>,    // 成功時のデータ
    pub errors: Option<Vec<GraphQLError>>, // エラー時のメッセージ
}

// data.transactions
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionsData {
    pub transactions: TransactionConnection,
}

// data.transactions (Connection パターン)
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionConnection {
    pub edges: Vec<TransactionEdge>,       // 検索結果の配列
    #[serde(rename = "pageInfo")]          // JSON: "pageInfo" → Rust: page_info
    pub page_info: PageInfo,
}

// data.transactions.edges[n]
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionEdge {
    pub node: TransactionNode,             // 実際のデータ
    pub cursor: String,                    // ページネーション用
}

// data.transactions.edges[n].node
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionNode {
    pub id: String,                        // トランザクションID
    pub block: Option<BlockInfo>,          // ブロック情報（オプション）
}

// data.transactions.pageInfo
#[derive(Debug, Deserialize)]
pub(crate) struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,               // 次のページの有無
}
```

**JSON → Rust のマッピング:**

```
JSON                                    Rust
─────────────────────────────────────────────────────────────
{                                       GraphQLResponse {
  "data": {                               data: Some(TransactionsData {
    "transactions": {                       transactions: TransactionConnection {
      "edges": [                              edges: vec![
        {                                       TransactionEdge {
          "node": { "id": "abc" },                node: TransactionNode { id: "abc" },
          "cursor": "xyz"                         cursor: "xyz"
        }                                       }
      ],                                      ],
      "pageInfo": {                           page_info: PageInfo {
        "hasNextPage": true                     has_next_page: true
      }                                       }
    }                                       }
  }                                       })
}                                       }
```

### 3.5 GraphQL クエリ構築の実装

```rust
// client.rs:182-210
fn build_graphql_query(&self, tags: &[Tag], cursor: Option<&str>) -> String {
    // 1. タグを GraphQL 形式に変換
    let tags_query: Vec<String> = tags
        .iter()
        .map(|t| format!(r#"{{ name: "{}", values: ["{}"] }}"#, t.name, t.value))
        .collect();
    // 例: [Tag::new("App-Name", "D-TPRES")]
    //   → [r#"{ name: "App-Name", values: ["D-TPRES"] }"#]

    // 2. ページネーション用カーソル
    let after_clause = cursor
        .map(|c| format!(r#", after: "{}""#, c))
        .unwrap_or_default();
    // 例: Some("abc") → r#", after: "abc""#
    // 例: None → ""

    // 3. クエリ文字列を構築
    format!(
        r#"{{
            transactions(
                tags: [{}]
                first: 100
                {}
            ) {{
                edges {{
                    node {{ id }}
                    cursor
                }}
                pageInfo {{ hasNextPage }}
            }}
        }}"#,
        tags_query.join(", "),  // タグをカンマ区切りで結合
        after_clause
    )
}
```

**生成されるクエリ例:**

```graphql
{
  transactions(
    tags: [{ name: "App-Name", values: ["D-TPRES"] }, { name: "Type", values: ["Capsule"] }]
    first: 100
  ) {
    edges {
      node { id }
      cursor
    }
    pageInfo { hasNextPage }
  }
}
```

### 3.6 query メソッドの実装詳細

```rust
// client.rs:364-429
async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError> {
    let mut all_tx_ids = Vec::new();
    let mut cursor: Option<String> = None;

    // ページネーションループ
    loop {
        // 1. GraphQL クエリを構築
        let query = self.build_graphql_query(&tags, cursor.as_deref());

        // 2. リクエストペイロード作成
        let payload = serde_json::json!({ "query": query });
        // → {"query": "{ transactions(...) { ... } }"}

        // 3. HTTP POST リクエスト
        let response = self
            .http_client
            .post(self.config.graphql_url())  // "https://arweave.net/graphql"
            .json(&payload)
            .send()
            .await?;

        // 4. レスポンスを Rust 型にデシリアライズ
        let graphql_response: GraphQLResponse = response.json().await?;

        // 5. エラーチェック
        if let Some(errors) = graphql_response.errors {
            if !errors.is_empty() {
                return Err(AdapterError::query_error(...));
            }
        }

        // 6. トランザクションIDを収集
        let data = graphql_response.data.ok_or(...)?;
        for edge in &data.transactions.edges {
            all_tx_ids.push(edge.node.id.clone());
        }

        // 7. 次のページがあれば続行
        if data.transactions.page_info.has_next_page {
            // 最後のエッジのカーソルを使用
            cursor = data.transactions.edges.last().map(|e| e.cursor.clone());
        } else {
            break;  // 全ページ取得完了
        }
    }

    Ok(all_tx_ids)
}
```

**ページネーションの流れ:**

```
1回目のリクエスト:
  query: { transactions(tags: [...], first: 100) { ... } }
  レスポンス:
    edges: [tx1, tx2, ..., tx100]
    pageInfo: { hasNextPage: true }
    cursor: "cursor_100"

2回目のリクエスト:
  query: { transactions(tags: [...], first: 100, after: "cursor_100") { ... } }
  レスポンス:
    edges: [tx101, tx102, ..., tx200]
    pageInfo: { hasNextPage: true }
    cursor: "cursor_200"

3回目のリクエスト:
  query: { transactions(tags: [...], first: 100, after: "cursor_200") { ... } }
  レスポンス:
    edges: [tx201, tx202, ..., tx250]
    pageInfo: { hasNextPage: false }  ← ループ終了

最終結果: [tx1, tx2, ..., tx250]
```

---

## 4. Base64URL エンコーディング

### 4.1 なぜ Base64URL か

Arweave はバイナリデータを **Base64URL** 形式でエンコードします。

```
通常の Base64:  SGVsbG8rV29ybGQ=
Base64URL:      SGVsbG8rV29ybGQ    ← パディング(=)なし、URL安全

違い:
  + → -
  / → _
  = → (削除)
```

### 4.2 実装

```rust
// client.rs:114-123
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

pub(crate) fn base64url_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn base64url_decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD.decode(encoded)
}
```

---

## 5. 全体アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           D-TPRES Client                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      ArweaveRepositoryImpl                        │   │
│  │  (Repository パターン: ドメインロジックから永続化を分離)           │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                       ArweaveClient trait                         │   │
│  │  ・get(tx_id) → Option<Vec<u8>>                                   │   │
│  │  ・post(data, tags) → String                                      │   │
│  │  ・query(tags) → Vec<String>                                      │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│              ┌─────────────────────┴─────────────────────┐              │
│              ▼                                           ▼               │
│  ┌─────────────────────────┐               ┌─────────────────────────┐  │
│  │   ArweaveClientImpl     │               │   MockArweaveClient     │  │
│  │   (本番実装)             │               │   (テスト用モック)       │  │
│  │                         │               │                         │  │
│  │   reqwest::Client       │               │   HashMap<String, Data> │  │
│  └─────────────────────────┘               └─────────────────────────┘  │
│              │                                                           │
└──────────────┼───────────────────────────────────────────────────────────┘
               │
               ▼ (HTTPS)
┌─────────────────────────────────────────────────────────────────────────┐
│                        Arweave Network                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│  │  Gateway Node   │    │  Gateway Node   │    │  Gateway Node   │     │
│  │  arweave.net    │    │  ar-io.net      │    │  g8way.io       │     │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘     │
│           │                     │                     │                  │
│           └─────────────────────┴─────────────────────┘                  │
│                                 │                                        │
│                                 ▼                                        │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                     Permanent Storage                              │  │
│  │  (Blockweave: データは200年以上保存される設計)                      │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 6. ArweaveClient 統合ガイド

### 6.1 基本的な使い方

```rust
use crate::adapter::external::arweave::{ArweaveClientConfig, ArweaveClientImpl};
use crate::adapter::repository_impl::{ArweaveClient, Tag};

// クライアント作成（デフォルト設定）
let config = ArweaveClientConfig::default();
let client = ArweaveClientImpl::new(config)?;
```

### 6.2 設定のカスタマイズ

```rust
// Builder パターンでカスタム設定
let config = ArweaveClientConfig::new()
    .with_gateway_url("https://arweave.net")
    .with_graphql_url("https://arweave.net/graphql")
    .with_timeout(60)
    .with_retries(5, 2000);

// 環境変数から設定を読み込む
let config = ArweaveClientConfig::from_env()?;
```

**環境変数:**
| 変数名 | デフォルト値 | 説明 |
|--------|-------------|------|
| `ARWEAVE_GATEWAY_URL` | `https://arweave.net` | Gateway URL |
| `ARWEAVE_GRAPHQL_URL` | `https://arweave.net/graphql` | GraphQL URL |
| `ARWEAVE_TIMEOUT_SECS` | `30` | タイムアウト秒数 |
| `ARWEAVE_MAX_RETRIES` | `3` | 最大リトライ回数 |
| `ARWEAVE_RETRY_BACKOFF_MS` | `1000` | リトライ間隔 (ms) |

### 6.3 データ取得 (`get`)

```rust
let tx_id = "BNttzDav3jHVnNiV7nYbQv-GY0HQ-4XXsdkE5K9ylHQ";
let result = client.get(tx_id).await?;

match result {
    Some(bytes) => println!("Data: {} bytes", bytes.len()),
    None => println!("Not found"),
}
```

### 6.4 トランザクション検索 (`query`)

```rust
let tags = vec![
    Tag::new("App-Name", "D-TPRES"),
    Tag::new("Type", "Capsule"),
];
let tx_ids = client.query(tags).await?;
```

### 6.5 データ投稿 (`post`) - ウォレット必須

```rust
use crate::adapter::external::arweave::ArweaveWallet;

// 環境変数からウォレット読み込み
let wallet = ArweaveWallet::from_env()?;

let client = ArweaveClientImpl::new(config)?
    .with_wallet(wallet);

let tx_id = client.post(data, tags).await?;
```

**ウォレット環境変数:**
```bash
export ARWEAVE_WALLET_JWK='{"kty":"RSA","n":"...","e":"AQAB","d":"..."}'
```

### 6.6 テスト実行

```bash
cargo test arweave -- --nocapture
```

---

## ファイル構成

```
client/src/adapter/external/arweave/
├── mod.rs      # モジュールエクスポート
├── config.rs   # ArweaveClientConfig (設定)
├── client.rs   # ArweaveClientImpl (本体)
├── wallet.rs   # ArweaveWallet (ウォレット)
└── tests.rs    # ユニットテスト & 統合テスト
```

## Test plan

- [x] `cargo test arweave` - 35テスト全て成功
- [x] 統合テスト（実Arweave接続）成功
- [x] `make lint` 成功
- [x] `make check` 成功

🤖 Generated with [Claude Code](https://claude.com/claude-code)
