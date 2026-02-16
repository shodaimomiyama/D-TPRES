# Requirements Document: production-ao-client

## Introduction

ProductionAOClientは、FORMIXクライアントライブラリ（`client/`）がAO Network上のOwner-ProcessおよびRequester-Processと**実際のHTTP通信**を行うための本番用AOClient実装を提供する。既存のMockAOClient（Issue #47、ao-network-communication spec）で定義されたAOClient traitを実装し、MU（Messenger Unit）およびCU（Compute Unit）のHTTP APIを通じて実際のAOプロセスとメッセージを送受信する。

**通信対象プロセス:**
- **Owner-Process**: clientからkFrag委譲メッセージ（DelegateKFrag）とCapsule委譲メッセージ（DelegateCapsule）を送信
- **Requester-Process**: clientからcFrag取得クエリ（GetCFrag）を送信
- **Holder-Process**: clientからは直接通信しない（Owner-ProcessまたはRequester-Processからのプロセス間メッセージングで動作）

**価値**:
- MockAOClientから本番環境への移行を実現
- AO Network上のOwner-Process / Requester-Processとの実際のHTTP通信
- Phase 1（kFrag配布: client → Owner-Process）とPhase 3（cFrag取得: client → Requester-Process）のE2E動作

**前提条件（ao-network-communication specで実装済み）**:
- AOClient trait（`execute`, `query`, `dry_run`メソッド）
- AOCommunicationError型
- ExecuteMsg / QueryMsg / AOResponse型
- MockAOClient（テスト用）
- KFragRepository / CFragRepository AO拡張

## Alignment with Product Vision

この機能は`product.md`で定義された以下の目標を直接実現する：

1. **分散性優先**: AO Network上の独立プロセスとの実際のHTTP通信を実現し、中央機関を介さない鍵管理を完成
2. **ステートレス実行**: AOプロセスのステートレスメッセージベース通信をHTTPレベルで実装
3. **開発者フレンドリーなSDK提供**: ProductionAOClientをDIで切り替えるだけで本番動作可能

**AO Network通信アーキテクチャ**:
```
client/ (Rust → WASM via wasm-pack)
  AOClient trait
    └─ ProductionAOClient
          └─ reqwest (wasm feature)
              │ HTTPS
              ▼
AO Network
  ├─ MU (Messenger Unit): POST / (signed DataItem)
  ├─ SU (Scheduler Unit): メッセージ順序管理
  └─ CU (Compute Unit): GET /result/:id, POST /dry-run
       ├─ Owner-Process    ← client直接通信 (DelegateKFrag, DelegateCapsule)
       ├─ Requester-Process ← client直接通信 (GetCFrag)
       └─ Holder-Process   ← プロセス間メッセージング (clientからは直接通信しない)
```

## Requirements

### Requirement 1: ProductionAOClient Config構造体

**User Story:** As a クライアントライブラリ開発者, I want AO Network接続先を構成可能なConfig構造体, so that テストネット・メインネット・カスタムノードに柔軟に接続できる

#### Design Note

Config構造体はMU URL、CU URL、Gateway URL、タイムアウト設定を保持する。デフォルト値としてaoconnect SDKと同じエンドポイントを使用する。`client/src/adapter/external/production_ao_client.rs`に配置。

デフォルトエンドポイント（aoconnect SDK準拠）:
- MU: `https://mu.ao-testnet.xyz`
- CU: `https://cu.ao-testnet.xyz`
- Gateway: `https://arweave.net`

#### Acceptance Criteria

1. WHEN AOConfig構造体が作成された THEN system SHALL MU URL、CU URL、Gateway URLフィールドを保持する
2. WHEN AOConfig::default()が呼び出された THEN system SHALL aoconnect SDKと同じデフォルトエンドポイントを返す
3. WHEN AOConfigにtimeout_msが設定された THEN system SHALL HTTP通信でそのタイムアウト値を使用する
4. IF MU URLが空文字列の場合 THEN system SHALL ConfigError::InvalidMuUrlを返す
5. IF CU URLが空文字列の場合 THEN system SHALL ConfigError::InvalidCuUrlを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_ao_config_fields` | Config構造体が全フィールドを保持することを検証 | Success |
| 2 | `test_ao_config_default_endpoints` | デフォルトエンドポイントがaoconnect準拠であることを検証 | Success |
| 3 | `test_ao_config_custom_timeout` | カスタムタイムアウト設定が反映されることを検証 | Success |
| 4 | `test_ao_config_invalid_mu_url` | 空MU URLでエラーが返ることを検証 | Validation |
| 5 | `test_ao_config_invalid_cu_url` | 空CU URLでエラーが返ることを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_ao_config_custom_endpoints` - カスタムエンドポイント設定の動作を検証
- `test_ao_config_builder_pattern` - ビルダーパターンでの構築を検証

---

### Requirement 2: ProductionAOClient構造体とAOClient trait実装

**User Story:** As a クライアントライブラリ開発者, I want 本番AO Networkと通信するAOClient実装, so that MockAOClientと差し替えるだけでE2E動作を実現できる

#### Design Note

ProductionAOClientはAOClient traitを実装し、reqwest（wasm feature有効）を使用してMU/CU HTTP APIと通信する。`client/src/adapter/external/production_ao_client.rs`に配置。既存のAOClient trait（`execute`, `query`, `dry_run`）をそのまま実装する。

MU API:
- `POST {MU_URL}/` - 署名済みDataItemをoctet-stream形式で送信
- ヘッダー: `Content-Type: application/octet-stream`, `Accept: application/json`

CU API:
- `POST {CU_URL}/dry-run?process-id={target}` - 読み取り専用実行
- `GET {CU_URL}/result/{message_id}?process-id={process_id}` - 結果取得
- ヘッダー: `Content-Type: application/json`, `Accept: application/json`

#### Acceptance Criteria

1. WHEN ProductionAOClient::new(config)が呼び出された THEN system SHALL AOConfigに基づいてHTTPクライアントを初期化する
2. WHEN execute()が呼び出された THEN system SHALL ExecuteMsgをDataItemに変換しMU APIにPOSTする
3. WHEN query()が呼び出された THEN system SHALL CU dry-run APIを使用してQueryMsgを送信しBinaryレスポンスを返す
4. WHEN dry_run()が呼び出された THEN system SHALL CU dry-run APIを使用してExecuteMsgを読み取り専用で実行しAOResponseを返す
5. WHEN execute()の結果を取得する際 THEN system SHALL CU result APIでメッセージ処理結果を取得する
6. WHEN execute()が成功した THEN system SHALL AOResponseのmessage_idフィールドにMUから返されたメッセージIDを設定する（AO Link検証用）
7. IF HTTP通信エラーが発生した THEN system SHALL AOCommunicationError::ConnectionErrorを返す
8. IF HTTPタイムアウトが発生した THEN system SHALL AOCommunicationError::Timeoutを返す
9. IF CU resultでエラーが返された THEN system SHALL AOCommunicationError::ExecutionErrorを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_production_ao_client_new` | AOConfigに基づく初期化を検証 | Success |
| 2 | `test_production_ao_client_execute` | MU APIへのPOST送信を検証（モックHTTPサーバー使用） | Success |
| 3 | `test_production_ao_client_query` | CU dry-run APIでのクエリを検証 | Success |
| 4 | `test_production_ao_client_dry_run` | CU dry-run APIでの読み取り専用実行を検証 | Success |
| 5 | `test_production_ao_client_result_fetch` | CU result APIでの結果取得を検証 | Success |
| 6 | `test_production_ao_client_message_id_in_response` | AOResponseにmessage_idが設定されることを検証 | Success |
| 7 | `test_production_ao_client_connection_error` | HTTP通信エラー時のConnectionErrorを検証 | Validation |
| 8 | `test_production_ao_client_timeout` | タイムアウト時のTimeoutエラーを検証 | Validation |
| 9 | `test_production_ao_client_execution_error` | CU実行エラー時のExecutionErrorを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_production_ao_client_implements_ao_client_trait` - AOClient trait実装の互換性を検証
- `test_production_ao_client_redirect_follow` - HTTPリダイレクト追跡を検証

---

### Requirement 3: AOメッセージのDataItem変換

**User Story:** As a クライアントライブラリ開発者, I want ExecuteMsg/QueryMsgをAO互換のDataItem形式に変換する機能, so that MUが受理可能なANS-104形式でメッセージを送信できる

#### Design Note

AO NetworkへのメッセージはArweaveのDataItem（ANS-104 bundled transaction）形式で送信する必要がある。DataItemにはAO固有のタグ（Action、Data-Protocol、Type等）を付与する。DataItem構築にはRust Arweaveクレート（bundles-rsまたは同等品）の調査・活用、もしくは最小限の自前実装を行う。

AO DataItemに必要なタグ:
- `Data-Protocol`: `ao`
- `Variant`: `ao.TN.1`（テストネット）
- `Type`: `Message`
- `SDK`: `ao`
- `Action`: ExecuteMsg/QueryMsgに対応するアクション名
- `Target`: 送信先プロセスID

#### Acceptance Criteria

1. WHEN ExecuteMsgが変換された THEN system SHALL ANS-104 DataItem形式のバイト列を生成する
2. WHEN DataItemが生成された THEN system SHALL `Data-Protocol: ao`、`Type: Message`、`Action: {action_name}`タグを含む
3. WHEN ExecuteMsg::DelegateKFragが変換された THEN system SHALL `Action: DelegateKFrag`タグとkFragデータをペイロードに含む
4. WHEN QueryMsg::GetCFragが変換された THEN system SHALL `Action: GetCFrag`タグとクエリパラメータをペイロードに含む
5. WHEN DataItemが生成された THEN system SHALL Targetタグに送信先プロセスIDを設定する
6. IF メッセージのシリアライズに失敗した THEN system SHALL AOCommunicationError::SerializationErrorを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_execute_msg_to_data_item` | ExecuteMsgからDataItemへの変換を検証 | Success |
| 2 | `test_data_item_ao_tags` | AO固有タグの付与を検証 | Success |
| 3 | `test_delegate_kfrag_data_item` | DelegateKFragメッセージのDataItem変換を検証 | Success |
| 4 | `test_get_cfrag_data_item` | GetCFragクエリのDataItem変換を検証 | Success |
| 5 | `test_data_item_target_tag` | Targetタグの設定を検証 | Success |
| 6 | `test_data_item_serialization_error` | シリアライズ失敗時のエラーを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_data_item_all_execute_msg_variants` - 全ExecuteMsgバリアントの変換を検証
- `test_data_item_binary_payload` - バイナリペイロードの正確性を検証

---

### Requirement 4: DataItemのArweaveウォレット署名

**User Story:** As a データ所有者/アクセス者, I want DataItemをArweaveウォレット（JWK）で署名する機能, so that AO MUが受理可能な認証済みメッセージを送信できる

#### Design Note

AOメッセージはArweave DataItemとして署名される必要がある。RSA-4096 JWK形式のArweaveウォレットを使用し、DataItemに署名を付与する。署名はProductionAOClientのコンストラクタでウォレットを受け取り、execute()時に自動的に適用する。

署名方式: RSA-PSS with SHA-256（Arweave標準）

#### Acceptance Criteria

1. WHEN ProductionAOClient::new()にJWKウォレットが渡された THEN system SHALL 以降のexecute()で自動的にDataItemに署名する
2. WHEN DataItemが署名された THEN system SHALL RSA-PSS SHA-256アルゴリズムで署名する
3. WHEN 署名済みDataItemが生成された THEN system SHALL 署名バイト列とOwnerフィールドを含む
4. IF JWKの形式が無効な場合 THEN system SHALL AOCommunicationError::ValidationErrorを返す
5. IF 署名処理に失敗した THEN system SHALL AOCommunicationError::SerializationErrorを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_production_client_with_wallet` | JWKウォレットでの初期化を検証 | Success |
| 2 | `test_data_item_rsa_pss_signature` | RSA-PSS SHA-256署名の正確性を検証 | Crypto |
| 3 | `test_signed_data_item_fields` | 署名バイト列とOwnerフィールドの存在を検証 | Success |
| 4 | `test_invalid_jwk_format` | 無効なJWK形式でのエラーを検証 | Validation |
| 5 | `test_signing_failure` | 署名失敗時のエラーを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_signed_data_item_verification` - 署名済みDataItemの検証が成功することを確認
- `test_owner_address_derivation` - JWKからOwnerアドレスが正しく導出されることを検証

---

### Requirement 5: CU Responseのパース

**User Story:** As a クライアントライブラリ開発者, I want CU APIレスポンスを既存のAOResponse/Binary型にパースする機能, so that 既存のRepository実装と互換性を保ったまま本番レスポンスを処理できる

#### Design Note

CU APIのレスポンスはJSON形式で、`Output`、`Messages`、`Spawns`、`Error`フィールドを含む。これを既存の`AOResponse`型および`Binary`型に変換する。dry-runレスポンスとresultレスポンスの両方に対応する。

CU Resultレスポンス構造:
```json
{
  "Output": { "data": "...", "tags": [...] },
  "Messages": [...],
  "Spawns": [...],
  "Error": "..."  // オプション
}
```

#### Acceptance Criteria

1. WHEN CU resultレスポンスが受信された THEN system SHALL JSON形式のレスポンスをAOResponse型にパースする
2. WHEN CU dry-runレスポンスが受信された THEN system SHALL Output.dataフィールドをBinary型として返す
3. WHEN レスポンスにMessagesフィールドが含まれる THEN system SHALL AOResponseのeventsフィールドに変換する
4. IF レスポンスにErrorフィールドが含まれる THEN system SHALL AOCommunicationError::ExecutionErrorを返す
5. IF レスポンスのJSONパースに失敗した THEN system SHALL AOCommunicationError::DeserializationErrorを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_parse_cu_result_response` | CU resultレスポンスのAOResponseパースを検証 | Success |
| 2 | `test_parse_cu_dryrun_response` | CU dry-runレスポンスのBinaryパースを検証 | Success |
| 3 | `test_parse_response_messages` | MessagesフィールドのAOEvent変換を検証 | Success |
| 4 | `test_parse_response_with_error` | Errorフィールド時のExecutionErrorを検証 | Validation |
| 5 | `test_parse_invalid_json` | 不正JSONでのDeserializationErrorを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_parse_response_with_tags` - Output.tagsのパースを検証
- `test_parse_empty_output` - 空Outputの処理を検証

---

### Requirement 6: DIコンテナ統合

**User Story:** As a クライアントライブラリ開発者, I want ProductionAOClientをDIコンテナに統合する機能, so that MockAOClientとの切り替えが容易にできる

#### Design Note

既存のDIコンテナ（`client/src/di.rs`）を拡張し、AOClientの実装をMockとProductionで切り替え可能にする。feature flagまたは設定による切り替えをサポート。

#### Acceptance Criteria

1. WHEN feature flag `production-ao`が有効な場合 THEN system SHALL ProductionAOClientをDIコンテナに登録する
2. WHEN feature flag `production-ao`が無効な場合 THEN system SHALL MockAOClientをDIコンテナに登録する（既存動作を維持）
3. WHEN ProductionAOClientがDIコンテナに登録された THEN system SHALL 既存のKFragRepository/CFragRepository実装から利用可能である

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_di_production_ao_client` | production-ao feature flagでProductionAOClientが登録されることを検証 | Success |
| 2 | `test_di_mock_ao_client_default` | デフォルトでMockAOClientが登録されることを検証 | Success |
| 3 | `test_di_repository_with_production_client` | ProductionAOClientでRepository操作が動作することを検証 | Success |

---

### Requirement 7: E2E統合テスト（ArLocal + AO Mainnet）

**User Story:** As a クライアントライブラリ開発者, I want ProductionAOClientが実際のAOプロセスと正しく通信できることをローカル環境およびAO Mainnetで検証するE2Eテスト, so that 本番環境での通信の正確性をAO Link（Explorer）で確認できる

#### Design Note

E2Eテストは2段階で実施する：

**Stage 1: ArLocal（ローカル環境）** — 自動化テスト
`ao/`配下のインフラを使用。`ao/scripts/start.js`でArLocal + cwao-unitsを起動し、コントラクトWASMをデプロイ・インスタンス化した上で、ProductionAOClientからHTTP通信を行う。

**Stage 2: AO Mainnet** — 手動検証
既存の`ao/scripts/deploy.js`と`ao/scripts/instantiate.js`を使用してAO Mainnetにデプロイし、ProductionAOClientから通信を実行。返却された`message_id`と`process_id`を使用して、AO Link（https://ao.link）で手動検証を行う。

**ローカル環境構成（Stage 1）:**
```
ArLocal (port 1984)     ← ローカルArweaveインスタンス
MU (port 1995)          ← Messenger Unit
SU (port 1996)          ← Scheduler Unit
CU (port 1997)          ← Compute Unit
```

**AO Mainnet構成（Stage 2）:**
```
MU: https://mu.ao-testnet.xyz
CU: https://cu.ao-testnet.xyz
Gateway: https://arweave.net
AO Link: https://ao.link  ← Explorer for manual verification
```

**テスト手順（Stage 1: ArLocal）:**
1. `ao/scripts/start.js`でローカル環境を起動
2. `ao/scripts/deploy.js`でコントラクトWASMをArLocalにアップロード → Module ID取得
3. `ao/scripts/instantiate.js`でAOプロセスを作成 → Process ID取得
4. ProductionAOClientにローカルMU/CU URLを設定（`http://localhost:1995`, `http://localhost:1997`）
5. 実際のexecute/query/dry_runを実行して検証

**テスト手順（Stage 2: AO Mainnet）:**
1. `ao/scripts/deploy.js`でコントラクトWASMをAO Mainnetにデプロイ → Module ID取得
2. `ao/scripts/instantiate.js`でAOプロセスを作成 → Process ID取得
3. ProductionAOClientにMainnetエンドポイントを設定
4. execute()を実行し、AOResponseからmessage_idを取得
5. `message_id`と`process_id`をコンソールに出力
6. AO Link（`https://ao.link/#/message/{message_id}`）で手動検証

**AOコントラクトのメッセージハンドラー（`ao/contracts/src/handlers.rs`）:**
- `handle_delegate_kfrag()`: kFragをHolderに委譲（CosmWasm SubMsg生成）
- `handle_delegate_capsule()`: CapsuleをHolderに委譲
- `handle_submit_kfrag()`: kFrag受信・保存（冪等性チェック付き）
- `handle_submit_capsule()`: Capsule受信・再暗号化パイプライン開始
- `handle_reencrypt()`: kFrag + Capsuleから cFragを生成
- `handle_get_cfrag()`: cFrag取得（INDEX_CAPS_TO_CFRAGで存在確認後、HOLDER_CFRAGSから取得）
- `handle_list_capsules_by_kfrag()`: ページネーション付きCapsule一覧

**メッセージ型（`ao/contracts/src/msg.rs`）:**
- ExecuteMsg: DelegateKFrag, DelegateCapsule, SubmitKFrag, SubmitCapsule, Reencrypt
- QueryMsg: GetCFrag, ListCapsulesByKFrag
- ValidateMessage: ID検証（1-128文字、英数字+_-）、Binary検証（非空、最大128KB）

**状態管理（`ao/contracts/src/state.rs`）:**
- OWNER_KFRAGS: `Map<(process_id, kfrag_id), OwnerKFragData>`
- OWNER_CAPSULES: `Map<(process_id, kfrag_id, capsule_id), OwnerCapsuleData>`
- HOLDER_CFRAGS: `Map<(process_id, kfrag_id, capsule_id), HolderCFragData>`
- CapsuleStatus: Received → ReencInProgress → CFragReady / Error

#### Acceptance Criteria

**Stage 1: ArLocal（自動化テスト）**

1. WHEN ローカルAO環境が起動している THEN system SHALL ProductionAOClientからローカルMU（`http://localhost:1995`）にExecuteMsgを送信できる
2. WHEN DelegateKFragメッセージが送信された THEN system SHALL AOプロセスのOwner-ProcessがkFragを受理しOwnerKFragDataに保存する
3. WHEN DelegateCapsuleメッセージが送信された THEN system SHALL AOプロセスがCapsuleを受理し再暗号化パイプラインを開始する
4. WHEN GetCFragクエリが送信された THEN system SHALL CU dry-run APIを通じてcFragデータ（Binary + BlobMeta）を取得できる
5. WHEN ListCapsulesByKFragクエリが送信された THEN system SHALL ページネーション付きCapsule一覧（CapsuleInfo + CapsuleStatus）を取得できる
6. WHEN ProductionAOClientの結果をAOResponseにパースした THEN system SHALL 既存MockAOClientのレスポンス形式と互換性がある
7. IF AOプロセスがデプロイされていない THEN system SHALL AOCommunicationError::ProcessNotFoundまたは適切なエラーを返す

**Stage 2: AO Mainnet（手動検証）**

8. WHEN AO MainnetにコントラクトWASMがデプロイされた THEN system SHALL `ao/scripts/deploy.js`で取得したModule IDでプロセスをインスタンス化できる
9. WHEN ProductionAOClientからAO Mainnetにexecute()を実行した THEN system SHALL AOResponseにmessage_idを含めて返す
10. WHEN execute()が成功した THEN system SHALL message_idとprocess_idをコンソールに出力しAO Link（`https://ao.link/#/message/{message_id}`）で手動検証可能にする

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_e2e_execute_to_local_mu` | ローカルMUへのExecuteMsg送信を検証 | E2E |
| 2 | `test_e2e_delegate_kfrag_accepted` | DelegateKFragがAOプロセスに受理されることを検証 | E2E |
| 3 | `test_e2e_delegate_capsule_reencryption` | DelegateCapsule後の再暗号化パイプライン動作を検証 | E2E |
| 4 | `test_e2e_get_cfrag_query` | GetCFragクエリでcFragが取得できることを検証 | E2E |
| 5 | `test_e2e_list_capsules_pagination` | ListCapsulesByKFragのページネーションを検証 | E2E |
| 6 | `test_e2e_response_format_compatibility` | レスポンスがMockAOClient互換であることを検証 | E2E |
| 7 | `test_e2e_invalid_process_error` | 存在しないプロセスへの通信エラーを検証 | E2E |
| 8 | `test_e2e_mainnet_deploy_and_instantiate` | AO Mainnetへのデプロイ・インスタンス化を検証 | E2E (Manual) |
| 9 | `test_e2e_mainnet_execute_returns_message_id` | Mainnet execute()がmessage_idを返すことを検証 | E2E (Manual) |
| 10 | `test_e2e_mainnet_ao_link_verification` | message_id/process_idのコンソール出力でAO Link検証可能を確認 | E2E (Manual) |

**Additional Tests (Beyond AC):**
- `test_e2e_full_kfrag_delegation_flow` - kFrag委譲→保存→cFrag生成→取得のフルフロー
- `test_e2e_idempotency` - 同一メッセージの重複送信に対する冪等性検証
- `test_e2e_capsule_status_transitions` - CapsuleStatusの状態遷移（Received→CFragReady）を検証
- `test_e2e_mainnet_full_flow` - AO Mainnetでのフルフロー（deploy→instantiate→execute→AO Link検証）

#### E2Eテスト環境セットアップ手順

**Stage 1: ArLocal（自動化テスト）**

```bash
# 1. ローカルAO環境を起動（ao/ディレクトリから）
cd ao && yarn start
# → ArLocal(1984) + MU(1995) + SU(1996) + CU(1997) 起動

# 2. コントラクトWASMをコンパイル
cd ao && yarn compile
# → contracts/target/wasm32-unknown-unknown/release/contract.wasm

# 3. WASMモジュールをArLocalにデプロイ
cd ao && yarn deploy --wallet test_wallet
# → Module ID 取得

# 4. AOプロセスをインスタンス化
cd ao && yarn instantiate --wallet test_wallet --module_id <MODULE_ID> --scheduler <SU_ADDR>
# → Process ID 取得

# 5. ProductionAOClientでE2Eテスト実行
# AOConfig { mu_url: "http://localhost:1995", cu_url: "http://localhost:1997", ... }
cargo test --features production-ao -- --test-threads=1 e2e_
```

**Stage 2: AO Mainnet（手動検証）**

```bash
# 1. コントラクトWASMをAO Mainnetにデプロイ
cd ao && yarn deploy --wallet <MAINNET_WALLET>
# → Module ID 取得

# 2. AOプロセスをインスタンス化
cd ao && yarn instantiate --wallet <MAINNET_WALLET> --module_id <MODULE_ID> --scheduler <SU_ADDR>
# → Process ID 取得

# 3. ProductionAOClientでMainnet E2Eテスト実行
# AOConfig::default() (mu: mu.ao-testnet.xyz, cu: cu.ao-testnet.xyz)
cargo test --features production-ao -- --test-threads=1 e2e_mainnet_
# → コンソールにmessage_idとprocess_idが出力される

# 4. AO Linkで手動検証
# https://ao.link/#/message/<MESSAGE_ID>
# https://ao.link/#/entity/<PROCESS_ID>
```

---

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: ProductionAOClientはHTTP通信のみを担当し、ビジネスロジックは含まない
- **Modular Design**: ProductionAOClient、Config、DataItem変換は独立したモジュールとして実装
- **Dependency Management**: AOClient traitを通じた抽象化を維持
- **Clean Architecture準拠**: Adapter層の`external/`ディレクトリに配置
- **既存パターン互換**: ao-network-communication specで確立したパターンを踏襲

### Performance
- **レスポンス時間**: MU API送信 + CU result取得の合計が10秒以内
- **タイムアウト**: 設定可能なタイムアウト（デフォルト30秒）
- **WASMバイナリサイズ**: reqwest wasm featureによるサイズ増加を最小限に抑える

### Security
- **JWK秘密鍵の保護**: JWKウォレットの秘密鍵がログやエラーメッセージに露出しない
- **HTTPS通信**: MU/CUへの全通信はHTTPS
- **メッセージ検証**: ValidateMessageトレイトによる整合性検証を維持
- **エラー情報の制限**: HTTP応答の内部詳細を外部に漏洩しない

### Reliability
- **リトライなし**: 一時的な通信エラー時のリトライは実装しない（呼び出し側の責任）
- **接続状態管理**: 接続エラーの明確なエラー報告
- **グレースフルエラーハンドリング**: 全HTTPエラーをAOCommunicationError型にマッピング

### Usability
- **ゼロコンフィグ動作**: AOConfig::default()でテストネット接続可能
- **明確なエラーメッセージ**: HTTPステータスコードとエラー本文を含むエラー
- **非同期API**: 既存のasync/awaitパターンを維持

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. AOConfig構造体 | 5 | 7 | 100% |
| 2. ProductionAOClient AOClient実装 | 9 | 11 | 100% |
| 3. DataItem変換 | 6 | 8 | 100% |
| 4. Arweaveウォレット署名 | 5 | 7 | 100% |
| 5. CU Responseパース | 5 | 7 | 100% |
| 6. DIコンテナ統合 | 3 | 3 | 100% |
| 7. E2E統合テスト（ArLocal + AO Mainnet） | 10 | 14 | 100% |
| **Total** | **43** | **57** | **100%** |

### Test Patterns Used

1. **Success Pattern**: 正常系の動作検証（HTTP通信、レスポンスパース）
2. **Validation Pattern**: 入力検証とエラーケース（不正URL、不正JWK）
3. **Crypto Pattern**: 暗号署名の正確性検証（RSA-PSS）
4. **Integration Pattern**: モックHTTPサーバーを使用した統合テスト
5. **E2E Pattern**: ローカルAO環境（ArLocal + cwao-units）を使用した実プロセス通信テスト

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test or N/A
- ❌ **Missing**: Test not implemented

## 参照ドキュメント

### AO Network通信
- **AO Cookbook**: https://cookbook_ao.arweave.net/
  - aoconnect接続ガイド: https://cookbook_ao.arweave.net/guides/aoconnect/connecting.html
- **aoconnect SDK**: https://www.npmjs.com/package/@permaweb/aoconnect
- **aoconnect source (MU/CU client)**: https://github.com/permaweb/ao/tree/main/connect/src/client

### MU/CU API仕様
- **MU API**: `POST {MU_URL}/` - Content-Type: application/octet-stream, Body: signed DataItem raw bytes
- **CU dry-run API**: `POST {CU_URL}/dry-run?process-id={target}` - Content-Type: application/json
- **CU result API**: `GET {CU_URL}/result/{message_id}?process-id={process_id}`
- **CU results API**: `GET {CU_URL}/results/{process}` - Query params: from, to, sort, limit

### デフォルトエンドポイント（aoconnect SDK準拠）
- MU: `https://mu.ao-testnet.xyz`
- CU: `https://cu.ao-testnet.xyz`
- Gateway: `https://arweave.net`

### DataItem/ANS-104
- **ANS-104仕様**: https://github.com/ArweaveTeam/arweave-standards/blob/master/ans/ANS-104.md
- **bundles-rs**: https://blog.decent.land/bundles-rs/ - Rust ANS-104クライアント（候補）

### AOコントラクト（ao/配下）
- **コントラクトソース**: `ao/contracts/src/` - CosmWasm契約（lib.rs, contract.rs, handlers.rs, msg.rs, state.rs）
- **デプロイスクリプト**: `ao/scripts/deploy.js` - WASMモジュールをArweaveにアップロード
- **インスタンス化**: `ao/scripts/instantiate.js` - AOプロセス作成（Process ID取得）
- **ローカル環境起動**: `ao/scripts/start.js` - ArLocal + cwao-units（MU/SU/CU）起動
- **テストユーティリティ**: `ao/test/utils.js` - ローカル環境ブートストラップ + ウォレット生成
- **テストスイート**: `ao/test/test.js` - Mocha E2Eテスト
- **依存パッケージ**: cwao ^0.5.3, cwao-units ^0.3.4, arlocal ^1.1.66
- **ローカルポート**: ArLocal(1984), MU(1995), SU(1996), CU(1997)

### 前提（ao-network-communication specで実装済み）
- AOClient trait: `client/src/adapter/external/ao_client.rs`
- AOCommunicationError: `client/src/adapter/errors.rs`
- AO Message types: `client/src/adapter/external/ao_message.rs`
- AOコントラクトメッセージ型: `ao/contracts/src/msg.rs`
