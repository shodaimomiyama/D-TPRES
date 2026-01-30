# Requirements Document

## Introduction

D-TPRESクライアントライブラリのActions層API（init, share, recover）のDeveloper Experience改善。現状の8引数関数をビルダーパターンに置き換え、init関数でAO Process初期化とネットワーク設定を行うことで、開発者がより直感的かつ型安全にAPIを利用できるようにする。本ライブラリはRust Crate（`dtpres-client`）として配布し、`cargo add`で導入可能にする。

## Alignment with Product Vision

- **開発者フレンドリーなSDKの提供**: 閾値暗号の統合障壁を低減
- **設計によるセキュリティ**: 型安全なAPIでミスを防止
- **複雑な暗号実装の隠蔽**: シンプルなAPIで内部の複雑さを抽象化
- **AO Networkとの統合**: Process管理を透過的に処理

## Requirements

### Requirement 1: init関数によるクライアント初期化

**User Story:** システム開発者として、D-TPRESクライアントを初期化したい。AO Processの管理とネットワーク設定を簡単に行うために。

#### Acceptance Criteria

1. WHEN initがwallet_pathと共に呼ばれた THEN システムはJWKファイルを読み込みウォレットを設定する SHALL
2. WHEN ウォレットに紐付く既存Processが存在する THEN システムは自動検出して接続する SHALL
3. WHEN ウォレットに紐付くProcessが存在しない THEN システムは新規Processをスポーンする SHALL
4. WHEN ao_gateway_urlが指定されない THEN システムはデフォルト値を使用する SHALL
5. WHEN arweave_gateway_urlが指定されない THEN システムはデフォルト値を使用する SHALL
6. WHEN initが成功した THEN システムはDTpresClientインスタンスを返す SHALL
7. WHEN DTpresClientが返された THEN process_id, wallet_address, gateway_urlsが取得可能である SHALL

#### Notes

- OwnerとRequesterは同一Processの異なる振る舞い
- JWKウォレットに紐付くユーザー固有のProcessが1つ存在

### Requirement 2: shareのビルダーパターン

**User Story:** システム開発者として、秘密共有のためにfluent builder APIを使いたい。引数の順序ミスを防ぎ、コンパイル時の型安全性を得るために。

#### Acceptance Criteria

1. WHEN shareが呼ばれた THEN システムはフィールド未設定のビルダーインスタンスを返す SHALL
2. WHEN すべての必須フィールド（secret, threshold, total_shares, owner_key, requester_key）が設定された THEN executeメソッドが利用可能になる SHALL
3. WHEN 必須フィールドが欠けた状態でexecuteが呼ばれた THEN システムはコンパイルエラーを生成する SHALL
4. WHEN ビルダーメソッドが任意の順序で呼ばれた THEN システムはエラーなく受け入れる SHALL
5. WHEN metadataオプションフィールドが未設定 THEN システムはNoneで処理を続行する SHALL
6. WHEN thresholdが設定される THEN .threshold(k).total_shares(n)の形式で指定する SHALL

#### Notes

- process_idはDTpresClientから自動取得（init時に確定）
- owner_public_keyはowner_secret_keyから内部で自動導出

### Requirement 3: share実行結果

**User Story:** システム開発者として、share実行後に必要な情報を取得したい。秘密の管理とHolder Process情報の確認のために。

#### Acceptance Criteria

1. WHEN shareが成功した THEN システムはsecret_idを返す SHALL
2. WHEN shareが成功した THEN システムはcapsule_infoを返す SHALL
3. WHEN shareが成功した THEN システムはencrypted_sharesを返す SHALL
4. WHEN shareが成功した THEN システムはholder_process_idsを返す SHALL

#### Notes

- secret_idはUUID v4で自動生成（secret内容とは直接紐付かない）
- holder_process_idsは現在固定値、将来RandAOで自動選出

### Requirement 4: recoverのビルダーパターン

**User Story:** システム開発者として、秘密復元のためにfluent builder APIを使いたい。shareと一貫性のあるAPIにするために。

#### Acceptance Criteria

1. WHEN recoverが呼ばれた THEN システムはフィールド未設定のビルダーインスタンスを返す SHALL
2. WHEN すべての必須フィールド（secret_id, requester_key）が設定された THEN executeメソッドが利用可能になる SHALL
3. WHEN 必須フィールドが欠けた状態でexecuteが呼ばれた THEN システムはコンパイルエラーを生成する SHALL
4. WHEN recoverが成功した THEN システムはrecovered_secret（Vec<u8>）のみを返す SHALL

#### Notes

- process_idはDTpresClientから自動取得
- 返り値はシンプルに復元された秘密のみ

### Requirement 5: owner_public_keyの自動導出

**User Story:** システム開発者として、owner_public_keyがowner_secret_keyから自動導出されてほしい。冗長なパラメータと鍵ペア不整合エラーを避けるために。

#### Acceptance Criteria

1. WHEN shareが使用された THEN システムはowner_public_keyをパラメータとして要求しない SHALL
2. WHEN owner_secret_keyが提供された THEN システムはumbral_preを使用して内部でowner_public_keyを導出する SHALL

### Requirement 7: Rust Crate配布

**User Story:** Rustの開発者として、`cargo add dtpres-client`でD-TPRESクライアントを導入したい。既存のRustプロジェクトに最小限の手順で統合するために。

#### Acceptance Criteria

1. WHEN `cargo add dtpres-client` THEN crateがプロジェクトの依存関係に追加される SHALL
2. WHEN crateが導入された THEN DTpresClient, ShareBuilder, RecoverBuilderがトップレベルの公開APIとして利用可能である SHALL
3. WHEN 公開APIが使用された THEN rustdocドキュメントが提供される SHALL
4. WHEN 内部実装（DefaultActionsContainer, AOClient等）が参照された THEN 非公開として隠蔽される SHALL

#### Notes

- crate名: `dtpres-client`
- 公開API: DTpresClient, InitConfig, ShareBuilder, RecoverBuilder, ShareResult, エラー型
- 内部実装の詳細はre-exportしない

### Requirement 6: 後方互換性

**User Story:** 旧APIを使用している開発者として、旧share/recover関数が引き続き動作してほしい。段階的に移行できるようにするために。

#### Acceptance Criteria

1. WHEN 8引数の旧share関数が呼ばれた THEN システムは非推奨警告と共に受け入れる SHALL
2. WHEN 4引数の旧recover関数が呼ばれた THEN システムは非推奨警告と共に受け入れる SHALL
3. WHEN 非推奨関数が使用された THEN システムは内部でビルダー実装に委譲する SHALL

## Non-Functional Requirements

### Crate Design

- **公開API最小化**: トップレベルの`pub use`で必要な型のみをre-export
- **内部実装隠蔽**: DefaultActionsContainer, AOClient等は`pub(crate)`で非公開
- **Feature Flags**: オプション機能（例: テスト用モック）はfeature flagsで制御
- **Semver準拠**: 公開APIの変更はSemantic Versioningに従う

### Code Architecture and Modularity

- **単一責任原則**: ビルダー型は独立したbuilder.rsモジュールに配置
- **モジュラー設計**: Type-stateマーカー型はビルダー間で再利用可能に
- **依存関係管理**: ビルダー層はActions層とController層のみに依存
- **明確なインターフェース**: ビルダーメソッドは明確で説明的な名前を持つ

### Performance

- ビルダーパターンのオーバーヘッドはゼロコスト（コンパイル時type-state）
- 鍵導出は既存のCryptoService実装を使用
- 必須フィールドチェックにランタイムリフレクションや動的ディスパッチを使用しない

### Security

- 既存のセキュリティ要件（Zeroize、定数時間操作）をすべて維持
- SecretKeyパラメータはZeroizeとZeroizeOnDropの実装を継続
- API変更によるセキュリティ退行なし
- JWKウォレットファイルの安全な読み込みと管理

### Reliability

- 移行後もすべての既存テストがパス
- 新APIは同等以上のテストカバレッジを持つ
- 統合テストでビルダーAPIによるshare-recoverラウンドトリップを検証
- AO Process接続の自動リカバリ

### Usability

- IDEオートコンプリートが開発者を必須フィールドへ導く
- 欠落フィールドのコンパイルエラーは明確でアクション可能
- ドキュメント（rustdoc）にすべての新APIの使用例を含める

## API Usage Example

### 1. Cargo.toml での導入

```toml
[dependencies]
dtpres = "0.1"
```

```rust
use dtpres::{DTpresClient, InitConfig};
use dtpres::builder::{ShareBuilder, RecoverBuilder};
use dtpres::error::{ClientError, ShareError, RecoverError};
use dtpres::types::{ShareResult, SecretMetadata};
```

### 2. クライアント初期化

```rust
// デフォルトゲートウェイ
let client = DTpresClient::init(InitConfig {
    wallet_path: "path/to/wallet.json",
    ao_gateway_url: None,
    arweave_gateway_url: None,
})?;

// カスタムゲートウェイ指定
let client = DTpresClient::init(InitConfig {
    wallet_path: "path/to/wallet.json",
    ao_gateway_url: Some("https://custom-ao.example.com".into()),
    arweave_gateway_url: Some("https://custom-arweave.example.com".into()),
})?;

println!("Process ID: {}", client.process_id());
println!("Wallet Address: {}", client.wallet_address());
```

### 3. 鍵ペア生成

```rust
let (owner_sk, owner_pk) = client.generate_keypair()?;
let (requester_sk, requester_pk) = client.generate_keypair()?;
```

### 4. 秘密共有（share） — ビルダーパターン

```rust
// メタデータ付きフルチェーン
let metadata = SecretMetadata {
    label: "medical-record-2024".into(),
    content_type: Some("application/json".into()),
};

let share_result: ShareResult = client.share()
    .secret(b"my secret data".to_vec())
    .threshold(3)
    .total_shares(5)
    .owner_key(owner_sk)
    .requester_key(requester_pk)
    .metadata(Some(metadata))
    .execute()?;
```

### 5. ShareResult の利用

```rust
println!("Secret ID: {}", share_result.secret_id);
println!("Capsule TX: {}", share_result.capsule_tx_id);
println!("Holder Process IDs: {:?}", share_result.holder_process_ids);
println!("Threshold: {}/{}", share_result.threshold, share_result.total_shares);
```

### 6. 秘密復元（recover） — ビルダーパターン

```rust
let recovered: Vec<u8> = client.recover()
    .secret_id(&share_result.secret_id)
    .requester_key(requester_sk)
    .execute()?;

assert_eq!(recovered, b"my secret data");
```

### 7. エラーハンドリング

```rust
// クライアント初期化エラー
match DTpresClient::init(config) {
    Ok(c) => { /* ... */ }
    Err(ClientError::WalletNotFound(path)) => eprintln!("Wallet not found: {path}"),
    Err(ClientError::ConnectionFailed(e)) => eprintln!("AO connection failed: {e}"),
    Err(e) => eprintln!("Init error: {e}"),
}

// share エラー
match client.share().secret(data).threshold(3).total_shares(5)
    .owner_key(sk).requester_key(pk).execute()
{
    Ok(result) => { /* ... */ }
    Err(ShareError::InvalidThreshold { threshold, total }) => {
        eprintln!("threshold {threshold} must be <= total {total}");
    }
    Err(ShareError::HolderSpawnFailed(e)) => eprintln!("Failed to spawn holders: {e}"),
    Err(e) => eprintln!("Share error: {e}"),
}

// recover エラー
match client.recover().secret_id(&id).requester_key(sk).execute() {
    Ok(secret) => { /* ... */ }
    Err(RecoverError::SecretNotFound(id)) => eprintln!("Secret {id} not found"),
    Err(RecoverError::InsufficientFragments { got, need }) => {
        eprintln!("Only {got}/{need} fragments collected");
    }
    Err(e) => eprintln!("Recover error: {e}"),
}
```

### 8. コンパイルエラー例（型状態パターン）

必須フィールドが未設定の場合、`execute()` メソッドが存在しないためコンパイルエラーになる。

```rust
// コンパイルエラー: threshold() が未呼び出し
let result = client.share()
    .secret(b"data".to_vec())
    // .threshold(3)          // <- 未設定
    .total_shares(5)
    .owner_key(owner_sk)
    .requester_key(requester_pk)
    .execute();               // error[E0599]: no method named `execute`
                              // found for `ShareBuilder<..., Missing, ...>`
```

### 9. 旧API（非推奨）

```rust
// 非推奨: 将来のバージョンで削除予定
#[allow(deprecated)]
let result = client.share_secret(
    b"data".to_vec(),
    3,           // threshold
    5,           // total_shares
    owner_sk,
    requester_pk,
    None,        // metadata
)?;
// warning: use of deprecated method `DTpresClient::share_secret`:
//          use `share()` builder instead
```
