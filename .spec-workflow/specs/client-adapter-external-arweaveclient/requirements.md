# ArweaveClient 外部アダプター要件定義

## はじめに

本仕様書は、`client/src/adapter/external/` ディレクトリにおける本番用 `ArweaveClient` の実装要件を定義します。`ArweaveClient` は、D-TPRES閾値プロキシ再暗号化システムで使用される暗号化データ、カプセル、鍵フラグメント（kFrag）、暗号化フラグメント（cFrag）をArweaveネットワークに保存・取得する役割を担います。

現在はテスト用の `MockArweaveClient` のみが存在しています。本機能は、D-TPRESクライアントライブラリの本番利用に必要な実際のArweaveネットワーク統合を実装します。

## プロダクトビジョンとの整合性

本機能は product.md に記載された以下のプロダクト目標を直接サポートします：

1. **Arweave永続ストレージ**: 暗号化データと暗号カプセルをArweaveに不変に保存するためのコアインフラを実装
2. **分散性優先**: 中央集権的なストレージプロバイダーに依存しない分散型データ永続化を実現
3. **検証可能性**: 保存されたすべてのデータはArweaveトランザクションを通じて取得・検証可能
4. **開発者フレンドリーなSDK**: D-TPRES SDKが透過的に使用する基盤ストレージメカニズムを提供

## 環境変数

本機能の実装に必要な環境変数を `.env.example` に追加すること：

```bash
# ===========================================
# ArweaveClient Configuration
# ===========================================

# Arweave Gateway URL (default: https://arweave.net)
ARWEAVE_GATEWAY_URL=https://arweave.net

# Arweave GraphQL Endpoint (default: https://arweave.net/graphql)
ARWEAVE_GRAPHQL_URL=https://arweave.net/graphql

# Arweave Wallet JWK JSON string (required for write operations)
# Leave empty for read-only mode
# Format: '{"kty":"RSA","n":"...","e":"AQAB","d":"..."}'
ARWEAVE_WALLET_JWK=

# HTTP Request Timeout in seconds (default: 30)
ARWEAVE_TIMEOUT_SECS=30

# Maximum retry attempts for failed requests (default: 3)
ARWEAVE_MAX_RETRIES=3

# Retry backoff base in milliseconds (default: 1000)
ARWEAVE_RETRY_BACKOFF_MS=1000
```

## 要件

### 要件1: ArweaveClient トレイト実装

**ユーザーストーリー:** D-TPRESクライアントライブラリ開発者として、本番用ArweaveClient実装が欲しい。これにより、リポジトリ層が実際のArweaveネットワークにデータを永続化・取得できるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN 有効なトランザクションIDで `ArweaveClient::get(tx_id)` が呼ばれる THEN システムはトランザクションデータを含む `Ok(Some(data))` を返す SHALL
2. WHEN 存在しないトランザクションIDで `ArweaveClient::get(tx_id)` が呼ばれる THEN システムは `Ok(None)` を返す SHALL
3. WHEN `ArweaveClient::get(tx_id)` の呼び出し時にネットワークエラーが発生する THEN システムは適切なエラーコンテキストを含む `Err(AdapterError::NetworkError)` を返す SHALL
4. WHEN 有効なデータとタグで `ArweaveClient::post(data, tags)` が呼ばれる THEN システムはデータをArweaveにアップロードし `Ok(transaction_id)` を返す SHALL
5. WHEN 2MBを超えるデータで `ArweaveClient::post(data, tags)` が呼ばれる THEN システムはサイズ制限メッセージを含む `Err(AdapterError::ValidationError)` を返す SHALL
6. WHEN 有効なタグで `ArweaveClient::query(tags)` が呼ばれる THEN システムはマッチするトランザクションIDを含む `Ok(Vec<String>)` を返す SHALL

### 要件2: Arweaveゲートウェイ設定

**ユーザーストーリー:** D-TPRES統合者として、Arweaveゲートウェイエンドポイントを設定したい。これにより、テスト、開発、本番環境で異なるゲートウェイを使用できるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN カスタム設定で `ArweaveClientImpl` が作成される THEN システムは指定されたゲートウェイURLを使用する SHALL
2. IF 設定が提供されない THEN システムはデフォルトのArweaveゲートウェイ（`https://arweave.net`）を使用する SHALL
3. WHERE 設定にカスタムタイムアウト値が含まれる THE システムはすべてのHTTP操作でそのタイムアウトを尊重する SHALL
4. WHERE 設定にカスタムリトライ設定が含まれる THE システムはその設定に従って失敗したリクエストをリトライする SHALL

### 要件3: GraphQLクエリサポート

**ユーザーストーリー:** D-TPRESクライアントライブラリ開発者として、ArweaveのGraphQL APIを使用してタグでトランザクションをクエリしたい。これにより、リポジトリ実装がメタデータでエンティティを効率的に検索できるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN `query(tags)` が呼ばれる THEN システムはArweaveゲートウェイのGraphQLエンドポイントに対する有効なGraphQLクエリを構築する SHALL
2. WHEN 複数のタグが提供される THEN システムは提供されたすべてのタグにマッチするトランザクションをフィルタリングする SHALL（AND論理）
3. WHEN クエリが結果を返す THEN システムはブロック高さでソートされたトランザクションIDを返す SHALL（最新が先頭）
4. WHEN クエリが結果を返さない THEN システムは空の `Vec<String>` を返す SHALL（エラーではない）

### 要件4: トランザクションデータ取得

**ユーザーストーリー:** D-TPRESクライアントライブラリ開発者として、IDでトランザクションデータを取得したい。これにより、リポジトリ実装が永続化されたエンティティをロードできるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN 確認済みのトランザクションIDで `get(tx_id)` が呼ばれる THEN システムは完全なトランザクションデータを返す SHALL
2. WHEN 保留中のトランザクションIDで `get(tx_id)` が呼ばれる THEN システムは `Ok(None)` を返す SHALL（トランザクションがまだ利用可能でない）
3. WHEN トランザクションデータが取得される THEN システムは変換なしで生のバイトを返す SHALL

### 要件5: トランザクション投稿

**ユーザーストーリー:** D-TPRESクライアントライブラリ開発者として、タグ付きでデータをArweaveに投稿したい。これにより、リポジトリ実装がエンティティを永続化できるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN `post(data, tags)` が呼ばれる THEN システムは提供されたデータとタグでArweaveトランザクションを作成する SHALL
2. WHEN トランザクションを投稿する THEN システムはD-TPRESアプリケーションタグ（`App-Name: D-TPRES`）を自動的に含める SHALL
3. WHEN トランザクションが正常に送信される THEN システムは確認を待たずに即座にトランザクションIDを返す SHALL
4. IF ウォレット/署名資格情報が設定されていない THEN システムは明確なメッセージを含む `Err(AdapterError::ConfigurationError)` を返す SHALL

### 要件6: ウォレット統合

**ユーザーストーリー:** D-TPRES統合者として、トランザクション署名用のウォレット資格情報を設定したい。これにより、Arweaveにデータを投稿できるようになる。

#### 受け入れ基準（EARS形式）

1. WHEN JWKウォレットで `ArweaveClientImpl` が作成される THEN システムはそのウォレットをトランザクション署名に使用する SHALL
2. WHEN ウォレットなしで `ArweaveClientImpl` が作成される THEN システムは読み取り専用操作（`get`、`query`）をサポートするが `post` 操作は拒否する SHALL
3. IF ウォレット資格情報が無効である THEN システムはクライアント初期化時に `Err(AdapterError::ConfigurationError)` を返す SHALL

### 要件7: エラーハンドリングと耐障害性

**ユーザーストーリー:** D-TPRES統合者として、堅牢なエラーハンドリングと自動リトライが欲しい。これにより、一時的なネットワーク問題がアプリケーション障害を引き起こさないようになる。

#### 受け入れ基準（EARS形式）

1. WHEN ネットワークリクエストがリトライ可能なエラー（5xx、タイムアウト）で失敗する THEN システムは指数バックオフで設定された最大試行回数までリトライする SHALL
2. WHEN ネットワークリクエストがリトライ不可能なエラー（4xx）で失敗する THEN システムはリトライせずに即座にエラーを返す SHALL
3. WHEN すべてのリトライ試行が尽きる THEN システムは最後のエラー詳細を含む `Err(AdapterError::NetworkError)` を返す SHALL
4. WHEN エラーが発生する THEN エラータイプはネットワーク、設定、またはデータの問題かを明確に示す SHALL

## 非機能要件

### コードアーキテクチャとモジュール性

- **単一責任原則**: `ArweaveClientImpl` はArweave HTTP/GraphQL通信のみに焦点を当てる。シリアライゼーションとエンティティマッピングはリポジトリ実装に残る
- **モジュラー設計**: 外部アダプターは `ArweaveClient` トレイトを通じて注入可能であり、テストとモック置換を容易にする
- **依存関係管理**: 外部依存関係（HTTPクライアント、JSON解析）は可能な限り抽象化する
- **明確なインターフェース**: `repository_impl/mod.rs` で定義された `ArweaveClient` トレイトインターフェースは変更なしで実装する

### パフォーマンス

- 効率的なゲートウェイ通信のためのHTTPコネクションプーリング
- 大きなデータ取得（>1MB）のためのレスポンスストリーミング
- 設定可能なタイムアウト（デフォルト: 読み取り30秒、書き込み60秒）
- 大きな結果セットのためのGraphQLクエリページネーション（デフォルト: ページあたり100アイテム）

### セキュリティ

- ウォレット資格情報（JWK）はログやエラーメッセージに決して露出させない
- すべてのゲートウェイ通信でHTTPSを強制する
- インジェクション攻撃を防ぐためにすべての公開APIメソッドで入力検証を行う

### 信頼性

- ネットワーク障害に対するジッター付き指数バックオフリトライ戦略
- ゲートウェイの健全性のためのサーキットブレーカーパターンの考慮
- Arweaveネットワークが混雑している場合のグレースフルデグラデーション

### 互換性

- ブラウザ使用のためのWASMターゲット（`wasm32-unknown-unknown`）互換性
- WASM環境で失敗するブロッキングI/O操作を使用しない
- 既存のトレイト定義に合わせて `async_trait` を使用した非同期/待機
