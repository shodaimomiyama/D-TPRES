# Requirements Document: ao-network-communication

## Introduction

AO Network通信基盤は、FORMIXクライアントライブラリ（`client/`）がAOネットワーク上のプロセス（Owner-Process、Holder-Process、Requester-Process）とデータ交換を行うための**インターフェース定義とMock実装**を提供する。この機能により、Phase 1（`dtpres.share()`）での鍵フラグメント（kFrag）のOwner-Processへの送信と、Phase 3（`dtpres.recover()`）での暗号化フラグメント（cFrag）のRequester-Processからの取得が可能になる。

**価値**:
- クライアントライブラリとAOプロセス間の通信を抽象化したインターフェース
- Phase 1とPhase 3のワークフロー開発・テストを可能にするMock実装
- Clean Architectureに準拠した拡張性の高い設計

## スコープ

**Issue #47 対応範囲:**
- AOClient trait（インターフェース）定義
- MockAOClient実装（テスト・開発用）
- AOCommunicationErrorエラー型
- KFragRepository/CFragRepository AOメソッド拡張

**別Issueで対応（新規Issue作成予定）:**
- ProductionAOClient実装（HTTP直接通信、MU/CU API）
- 実際のAO Networkとの通信

**関連Issue（依存関係）:**
- #51: SecretRecoveryWorkflowService - StorageService統合 → #47完了後に対応
- #52: SecretSharingWorkflowService - StorageService統合 → #47完了後に対応

**注記:**
現時点でAO公式のRust SDKは存在しないため、Production実装ではHTTP直接通信（MU/CU API）を自前実装する方針。本Issueではインターフェース抽象化により、将来のProduction実装に備える。

### Clean Architectureへの準拠

本機能は`tech.md`で定義されたClean Architecture（6層構成）に厳密に準拠し、以下の設計原則を適用する。

#### レイヤー構成と責務分離

```
┌─────────────────────────────────────────────────────────────────┐
│ Actions層                                                        │
│   share(), recover() が StorageService を呼び出す               │
├─────────────────────────────────────────────────────────────────┤
│ Controller層                                                     │
│   入力バリデーション、DTO変換                                    │
├─────────────────────────────────────────────────────────────────┤
│ UseCase層                                                        │
│   StorageService: send_kfrags_to_owner_process(),               │
│                   retrieve_cfrags_from_requester_process()      │
├─────────────────────────────────────────────────────────────────┤
│ Domain層                                                         │
│   KFrag, CFrag, Capsule エンティティ定義                        │
├─────────────────────────────────────────────────────────────────┤
│ Repository層（Interface）                                        │
│   KFragRepository trait: send_to_ao_process()                   │
│   CFragRepository trait: retrieve_from_ao_process()             │
├─────────────────────────────────────────────────────────────────┤
│ Adapter層（Implementation）                                      │
│   AOClient trait → MockAOClient / ProductionAOClient            │
│   KFragRepositoryImpl, CFragRepositoryImpl                      │
└─────────────────────────────────────────────────────────────────┘
```

#### 依存性逆転原則（DIP）の適用

1. **Repository Interface**: `repositories/kfrag_interface.rs`と`repositories/cfrag_interface.rs`でAO通信の抽象を定義
2. **Adapter Implementation**: `adapter/repository_impl/`でAOClientを使用した具体実装を提供
3. **依存方向**: UseCase層 → Repository Interface ← Adapter層（実装）

```rust
// Repository Interface (repositories/kfrag_interface.rs)
#[async_trait]
pub trait KFragRepository: Repository<KFrag, KFragId> {
    async fn send_to_ao_process(&self, process_id: &str, kfrag: &KFrag) -> DomainResult<()>;
    async fn batch_send_to_ao_process(&self, process_id: &str, kfrags: &[KFrag]) -> DomainResult<Vec<KFragId>>;
}

// Adapter Implementation (adapter/repository_impl/kfrag_impl.rs)
pub struct ArweaveKFragRepository<C: AOClient> {
    ao_client: Arc<C>,
    arweave_client: Arc<dyn ArweaveClient>,
}
```

#### 拡張性の実現

| 拡張ポイント | Issue #47実装 | 将来の拡張（別Issue） |
|-------------|--------------|---------------------|
| AOClient | MockAOClient（テスト・開発用） | ProductionAOClient（HTTP直接通信） |
| Repository | ArweaveKFragRepository | CachedKFragRepository（キャッシュ層追加） |
| 通信方式 | Mock（インメモリ） | HTTP直接通信（MU/CU API） |
| エラーハンドリング | AOCommunicationError | RetryableAOError（リトライ戦略） |

#### テスタビリティの確保

- **Mock注入**: AOClientトレイトにより、テスト時にMockAOClientを注入可能
- **レイヤー分離テスト**: 各レイヤーを独立してユニットテスト可能
- **統合テスト**: DIコンテナを通じて実装を差し替えて統合テスト可能

```rust
// テスト時のDI例
let mock_ao_client = Arc::new(MockAOClient::new());
let kfrag_repo = ArweaveKFragRepository::new(mock_ao_client.clone(), arweave_client);
let storage_service = StorageService::new(kfrag_repo, cfrag_repo);
```

## Alignment with Product Vision

この機能は`product.md`で定義された以下の目標を支援する：

1. **分散性優先**: AOネットワーク上の独立したプロセスと通信することで、中央機関を介さない鍵管理を実現
2. **検証可能性**: AO通信を通じてすべての鍵フラグメント配布と取得が追跡可能
3. **ステートレス実行**: AOプロセスとのメッセージベース通信により、ステートレス計算モデルに対応

**関連するプロダクト目標**:
- k-of-n閾値プロキシ再暗号化の実現（kFrag配布機能）
- 再暗号化レイテンシ1秒未満の達成（効率的なAO通信）
- 開発者フレンドリーなSDK提供（抽象化されたAO通信API）

## クライアント-AOプロセス間通信アーキテクチャ

本機能はFORMIXクライアントライブラリ（`client/`）からAOネットワーク上のプロセスを呼び出すための**通信インターフェース**を定義する。

### アーキテクチャ方針

**PRD.md（8.1節）との関係:**
- PRDでは`local/` + `src/` + `dtpres-sdk/`の3層分離を記載
- 本Issueでは`client/`にAO通信インターフェースを統合する方針を採用
- `dtpres-sdk/`（JavaScript統合層）は将来の拡張として対応

**選択理由:**
- Rust単体で暗号処理とAO通信を統合（型安全性）
- ブラウザ優先ターゲット（wasm-pack）
- AO公式Rust SDKが存在しないため、インターフェース抽象化で将来に備える

### 通信の仕組み

**Issue #47 実装範囲（Mock）:**

```
┌──────────────────────────────────────────────────────────────────┐
│ client/ (Rust → WASM via wasm-pack)                               │
│   UseCase層: StorageService                                       │
│     └─ Repository層: KFragRepository / CFragRepository           │
│           └─ Adapter層: AOClient trait                           │
│                 └─ MockAOClient（インメモリ、テスト用）           │
└──────────────────────────────────────────────────────────────────┘
```

**将来のProduction実装（別Issue）:**

```
┌──────────────────────────────────────────────────────────────────┐
│ client/ (Rust → WASM)                                             │
│   AOClient trait                                                  │
│     └─ ProductionAOClient                                        │
│           └─ HTTP直接通信（web-sys::fetch / reqwest）            │
└──────────────────────────────────────────────────────────────────┘
            │ HTTPS
            ▼
┌──────────────────────────────────────────────────────────────────┐
│ AO Network                                                        │
│   ├─ MU (Messenger Unit): POST /message                          │
│   ├─ SU (Scheduler Unit): メッセージ順序管理                      │
│   └─ CU (Compute Unit): POST /dryrun, GET /result/:id            │
│        └─ FORMIX AO Process (Owner/Holder/Requester)            │
└──────────────────────────────────────────────────────────────────┘
            │ Arweaveトランザクション
            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Arweave Storage                                                   │
│   └─ Capsule, 暗号化シェア, kFrag, cFrag 永続化                  │
└──────────────────────────────────────────────────────────────────┘
```

**ソース:**
- `client/docs/PRD.md` セクション8「アーキテクチャ戦略」
- AO公式ドキュメント: https://cookbook_ao.g8way.io/
- AO公式リポジトリ: https://github.com/permaweb/ao

### 通信プロトコル詳細（将来のProduction実装用）

| 項目 | 仕様 | ソース |
|------|------|--------|
| **トランスポート** | HTTPS（MU/CU直接通信） | [AO Cookbook](https://cookbook_ao.g8way.io/guides/aoconnect/connecting.html) |
| **メッセージ形式** | JSON (serde snake_case) | `ao/contracts/src/msg.rs` ExecuteMsg/QueryMsg |
| **MU API** | POST /message | [@permaweb/aoconnect](https://www.npmjs.com/package/@permaweb/aoconnect)参考 |
| **CU API** | POST /dryrun, GET /result/:id | 同上 |
| **永続化** | Arweaveトランザクション | `client/docs/PRD.md` |

**注記:** AO公式Rust SDKは現時点で存在しない。Production実装では`@permaweb/aoconnect`のソースコードを参考にHTTP APIを自前実装する。

### AOプロセスが公開するAPI（呼び出し先）

クライアントは`ao/contracts/src/msg.rs`で定義された以下のAPIを呼び出す：

1. **状態変更API（message経由）**
   - `ExecuteMsg::DelegateKFrag`: Owner-ProcessにkFragを委譲
   - `ExecuteMsg::SubmitKFrag`: Holder-ProcessにkFragを送信
   - `ExecuteMsg::DelegateCapsule`: Capsuleを委譲

2. **読み取りAPI（dryrun経由）**
   - `QueryMsg::GetCFrag`: Requester-ProcessからcFragを取得
   - `QueryMsg::ListCapsulesByKFrag`: kFragに関連するCapsule一覧を取得

3. **レスポンス形式**
   - `Response`型: 実行結果 + Events + Attributes（トレーサビリティ用）
   - `Binary`型: クエリ結果（JSON/バイナリ）

### AOMessageTags（メッセージメタデータ）

`ao/contracts/src/msg.rs:216-250`で定義されたタグ構造を使用：

```rust
pub struct AOMessageTags {
    pub app_name: String,      // "cwao"
    pub action: String,        // "DelegateKFrag" / "GetCFrag" 等
    pub read_only: String,     // "True" / "False"
    pub input: String,         // JSONペイロード
    pub process_id: String,    // ターゲットプロセスID
    pub actor: String,         // 送信者アドレス
    pub ts: String,            // タイムスタンプ
}
```

### バリデーション

クライアント側でも`ValidateMessage`トレイト（`ao/contracts/src/msg.rs:88-166`）に準拠したバリデーションを実行：
- `validate_kfrag_id`: 最大128文字、英数字+アンダースコア+ハイフン
- `validate_capsule_id`: 同上
- `Binary data`: 最大128KB

### エラー処理

`thiserror`ベースのエラー型を使用し、AOプロセス側の`ContractError`との互換性を維持

## Requirements

### Requirement 1: AOClient基盤インターフェース

**User Story:** As a クライアントライブラリ開発者, I want AOネットワークとの通信を抽象化したクライアントインターフェース, so that AO固有の実装詳細を隠蔽しつつ、テスト可能な設計でkFrag送信・cFrag取得ができる

#### Design Note

AOClientはAdapter層の`external/`ディレクトリに配置する。既存の`AOMessageTags`構造体を活用し、CosmWasmの`ExecuteMsg`/`QueryMsg`パターンに準拠したインターフェースを定義する。

**Issue #47 スコープ:**
- `AOClient` trait定義（async trait）
- `MockAOClient`実装（インメモリ、テスト・開発用）

**別Issue対応（新規Issue作成予定）:**
- `ProductionAOClient`実装（HTTP直接通信、MU/CU API）

#### Acceptance Criteria

1. WHEN AOClientトレイトが定義された THEN system SHALL `execute`メソッドで`ExecuteMsg`をAOプロセスに送信し`Response`を受信できる
2. WHEN AOClientトレイトが定義された THEN system SHALL `query`メソッドで`QueryMsg`をAOプロセスに送信し`Binary`レスポンスを受信できる
3. WHEN AOClientトレイトが定義された THEN system SHALL `dry_run`メソッドでプロセスの状態を読み取りのみで取得できる
4. WHEN メッセージ送信時 THEN system SHALL `AOMessageTags`構造体を使用してメッセージメタデータを構築する
5. IF 接続エラーが発生した THEN system SHALL `AOCommunicationError::ConnectionError`を返す
6. IF タイムアウトが発生した THEN system SHALL `AOCommunicationError::Timeout`を返す
7. IF プロセスが存在しない THEN system SHALL `AOCommunicationError::ProcessNotFound`を返す
8. WHEN MockAOClientが実装された THEN system SHALL インメモリでkFrag/cFragを管理しテスト・開発に使用できる
9. WHEN MockAOClientが実装された THEN system SHALL 設定可能なレスポンス遅延・エラー注入機能を持つ

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_ao_client_execute_delegate_kfrag` | ExecuteMsgによるkFrag委譲が正常に完了することを検証（Mock使用） | Success |
| 2 | `test_ao_client_query_get_cfrag` | QueryMsgによるcFrag取得が正常に完了することを検証（Mock使用） | Success |
| 3 | `test_ao_client_dry_run_success` | dry_run呼び出しが読み取りのみで実行されることを検証（Mock使用） | Success |
| 4 | `test_ao_client_message_tags_construction` | AOMessageTagsが正しく構築されることを検証 | Success |
| 5 | `test_ao_client_connection_error` | MockAOClientで接続エラーを注入しConnectionErrorが返ることを検証 | Validation |
| 6 | `test_ao_client_timeout_error` | MockAOClientでタイムアウトを注入しTimeoutエラーが返ることを検証 | Validation |
| 7 | `test_ao_client_process_not_found` | 存在しないプロセスへのアクセス時にProcessNotFoundが返ることを検証 | Validation |
| 8 | `test_mock_ao_client_kfrag_storage` | MockAOClientがインメモリでkFragを管理できることを検証 | Success |
| 9 | `test_mock_ao_client_error_injection` | MockAOClientでエラー注入が正しく動作することを検証 | Success |

**Additional Tests (Beyond AC):**
- `test_ao_client_message_serialization` - ExecuteMsg/QueryMsgのシリアライズ/デシリアライズが正しく行われることを検証
- `test_ao_client_validate_message_trait` - ValidateMessageトレイトによるバリデーションが正しく動作することを検証

---

### Requirement 2: KFragのAO送信機能

**User Story:** As a データ所有者（Owner）, I want Phase 1で生成したkFragをOwner-Processに送信する機能, so that Holder-Processへの配布準備ができる

#### Design Note

StorageServiceを拡張し、`send_kfrags_to_owner_process`メソッドを追加する。既存の`ExecuteMsg::DelegateKFrag`メッセージ型を使用し、AOClientを通じてOwner-ProcessにkFragを送信する。送信成功時は`Response`のEvents/Attributesでトレーサビリティを確保する。

#### Acceptance Criteria

1. WHEN `send_kfrags_to_owner_process`が呼び出された THEN system SHALL `ExecuteMsg::DelegateKFrag`メッセージを使用して全てのkFragをOwner-Processに送信する
2. WHEN kFragが正常に送信された THEN system SHALL `Response`に`kfrag_delegated`イベントと関連属性を含めて返す
3. WHEN kFrag送信前 THEN system SHALL `ValidateMessage`トレイトを使用してメッセージバリデーションを実行する
4. IF Owner-ProcessのプロセスIDが無効な場合 THEN system SHALL `AOCommunicationError::InvalidProcessId`を返す
5. IF kFragのシリアライズに失敗した場合 THEN system SHALL `AOCommunicationError::SerializationError`を返す
6. WHEN 部分的な送信失敗が発生した THEN system SHALL 送信成功したkFragと失敗したkFragの情報を含むエラーを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_send_kfrags_delegate_kfrag_message` | DelegateKFragメッセージが正しく構築・送信されることを検証 | Success |
| 2 | `test_send_kfrags_response_events` | 送信成功時にkfrag_delegatedイベントが含まれることを検証 | Success |
| 3 | `test_send_kfrags_validate_message` | ValidateMessageによるバリデーションが実行されることを検証 | Success |
| 4 | `test_send_kfrags_invalid_process_id` | 無効なプロセスIDでInvalidProcessIdエラーが返ることを検証 | Validation |
| 5 | `test_send_kfrags_serialization_error` | シリアライズ失敗時にSerializationErrorが返ることを検証 | Validation |
| 6 | `test_send_kfrags_partial_failure` | 部分的失敗時に詳細なエラー情報が返ることを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_send_kfrags_empty_list` - 空のkFragリスト送信時の動作を検証
- `test_send_kfrags_batch_performance` - 大量kFrag送信時のパフォーマンスを検証

---

### Requirement 3: CFragのAO取得機能

**User Story:** As a データアクセス者（Requester）, I want Phase 3でRequester-ProcessからcFragを取得する機能, so that 秘密の復元に必要な再暗号化フラグメントを収集できる

#### Design Note

StorageServiceを拡張し、`retrieve_cfrags_from_requester_process`メソッドを追加する。既存の`QueryMsg::GetCFrag`メッセージ型を使用し、AOClientを通じてRequester-ProcessからcFragを取得する。閾値（k）以上のcFragが必要であり、取得数の検証も行う。

#### Acceptance Criteria

1. WHEN `retrieve_cfrags_from_requester_process`が呼び出された THEN system SHALL `QueryMsg::GetCFrag`メッセージを使用してRequester-Processから利用可能なcFragを取得する
2. WHEN cFragが正常に取得された THEN system SHALL `GetCFragResponse`型でcFragとメタデータを返す
3. WHEN cFrag取得前 THEN system SHALL `ValidateMessage`トレイトを使用してクエリメッセージのバリデーションを実行する
4. IF 取得したcFrag数が閾値（k）未満の場合 THEN system SHALL `AOCommunicationError::InsufficientCFrags`を返す
5. IF Requester-ProcessのプロセスIDが無効な場合 THEN system SHALL `AOCommunicationError::InvalidProcessId`を返す
6. IF cFragのデシリアライズに失敗した場合 THEN system SHALL `AOCommunicationError::DeserializationError`を返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_retrieve_cfrags_get_cfrag_query` | GetCFragクエリが正しく構築・送信されることを検証 | Success |
| 2 | `test_retrieve_cfrags_response_format` | GetCFragResponseが正しい形式で返ることを検証 | Success |
| 3 | `test_retrieve_cfrags_validate_message` | ValidateMessageによるクエリバリデーションが実行されることを検証 | Success |
| 4 | `test_retrieve_cfrags_insufficient_threshold` | 閾値未満の場合にInsufficientCFragsエラーが返ることを検証 | Validation |
| 5 | `test_retrieve_cfrags_invalid_process_id` | 無効なプロセスIDでInvalidProcessIdエラーが返ることを検証 | Validation |
| 6 | `test_retrieve_cfrags_deserialization_error` | デシリアライズ失敗時にDeserializationErrorが返ることを検証 | Validation |

**Additional Tests (Beyond AC):**
- `test_retrieve_cfrags_exactly_threshold` - 閾値ちょうどのcFrag取得時の動作を検証
- `test_retrieve_cfrags_list_capsules_by_kfrag` - ListCapsulesByKFragクエリの動作を検証

---

### Requirement 4: CapsuleのArweave取得機能

**User Story:** As a データアクセス者（Requester）, I want ArweaveからCapsuleを取得する機能, so that 秘密復元に必要な暗号カプセルを入手できる

#### Design Note

既存のArweaveStorageServiceを活用し、`retrieve_capsule`メソッドを追加する。CapsuleRepositoryを通じてArweaveClientを使用し、SecretIdに対応するCapsuleを取得する。

#### Acceptance Criteria

1. WHEN `retrieve_capsule`がSecretIdで呼び出された THEN system SHALL ArweaveからCapsuleを取得する
2. WHEN Capsuleが正常に取得された THEN system SHALL 取得したCapsuleを返す
3. IF 指定されたSecretIdに対応するCapsuleが存在しない場合 THEN system SHALL `StorageError::NotFound`を返す
4. IF Arweaveへの接続に失敗した場合 THEN system SHALL `StorageError::ConnectionError`を返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_retrieve_capsule_by_secret_id_success` | SecretIdでCapsuleが正常に取得されることを検証 | Success |
| 2 | `test_retrieve_capsule_returns_valid_capsule` | 取得したCapsuleが有効な形式であることを検証 | Success |
| 3 | `test_retrieve_capsule_not_found` | 存在しないSecretIdでNotFoundエラーが返ることを検証 | Validation |
| 4 | `test_retrieve_capsule_connection_error` | 接続失敗時にConnectionErrorが返ることを検証 | Validation |

---

### Requirement 5: AOCommunicationErrorエラー型

**User Story:** As a クライアントライブラリ開発者, I want AO通信に特化したエラー型, so that AO関連のエラーを適切にハンドリングし、開発者に明確なエラー情報を提供できる

#### Design Note

`adapter/errors.rs`を拡張し、AOCommunicationError enumを追加する。CosmWasmの`thiserror`パターンに準拠し、`#[error]`属性でDisplayトレイトを実装、`#[from]`属性でFrom変換を自動生成する。AdapterErrorとDomainErrorへの変換トレイトも実装する。

#### Acceptance Criteria

1. WHEN AO通信でエラーが発生した THEN system SHALL 適切なAOCommunicationErrorバリアントを返す
2. WHEN AOCommunicationErrorが定義された THEN system SHALL `thiserror`の`#[error]`属性でDisplayトレイトを実装する
3. WHEN AOCommunicationErrorが発生した THEN system SHALL `From`トレイトでAdapterErrorに変換可能である
4. WHEN AOCommunicationErrorが発生した THEN system SHALL `From`トレイトでDomainErrorに変換可能である
5. IF エラーが発生した THEN system SHALL エラーメッセージに操作種別と詳細情報を含む

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_ao_communication_error_variants` | 全エラーバリアントが定義されていることを検証 | Success |
| 2 | `test_ao_error_display_trait` | Displayトレイトが正しく実装されていることを検証 | Success |
| 3 | `test_ao_error_to_adapter_error_conversion` | AdapterErrorへの変換が正常に行われることを検証 | Success |
| 4 | `test_ao_error_to_domain_error_conversion` | DomainErrorへの変換が正常に行われることを検証 | Success |
| 5 | `test_ao_error_message_contains_details` | エラーメッセージに詳細情報が含まれることを検証 | Success |

---

### Requirement 6: KFragRepository AOインターフェース拡張

**User Story:** As a クライアントライブラリ開発者, I want KFragRepositoryにAO通信用のメソッドを追加, so that リポジトリパターンを維持しながらAO操作を抽象化できる

#### Design Note

既存の`repositories/kfrag_interface.rs`を拡張し、AO通信用のメソッドを追加する。`ExecuteMsg::DelegateKFrag`を使用し、Arweave永続化とAO送信を分離する。依存性逆転原則を維持する。

#### Acceptance Criteria

1. WHEN KFragRepositoryが拡張された THEN system SHALL `send_to_ao_process`メソッドで`ExecuteMsg::DelegateKFrag`を使用してkFragをAOプロセスに送信できる
2. WHEN KFragRepositoryが拡張された THEN system SHALL `batch_send_to_ao_process`メソッドで複数のkFragを一括送信できる
3. IF AOプロセスへの送信に失敗した THEN system SHALL `DomainError`を返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_kfrag_repository_send_to_ao_process` | 単一kFragのAO送信が正常に動作することを検証 | Success |
| 2 | `test_kfrag_repository_batch_send_to_ao_process` | 複数kFragの一括送信が正常に動作することを検証 | Success |
| 3 | `test_kfrag_repository_ao_send_failure` | AO送信失敗時にDomainErrorが返ることを検証 | Validation |

---

### Requirement 7: CFragRepository AOインターフェース拡張

**User Story:** As a クライアントライブラリ開発者, I want CFragRepositoryにAO取得用のメソッドを追加, so that リポジトリパターンを維持しながらAO操作を抽象化できる

#### Design Note

既存の`repositories/cfrag_interface.rs`を拡張し、AO通信用のメソッドを追加する。`QueryMsg::GetCFrag`を使用し、Requester-ProcessからのcFrag取得機能を実装する。

#### Acceptance Criteria

1. WHEN CFragRepositoryが拡張された THEN system SHALL `retrieve_from_ao_process`メソッドで`QueryMsg::GetCFrag`を使用してAOプロセスからcFragを取得できる
2. WHEN CFragRepositoryが拡張された THEN system SHALL SecretIdでフィルタリングしてcFragを取得できる
3. IF AOプロセスからの取得に失敗した THEN system SHALL `DomainError`を返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_cfrag_repository_retrieve_from_ao_process` | AOプロセスからcFragが正常に取得されることを検証 | Success |
| 2 | `test_cfrag_repository_retrieve_with_secret_id_filter` | SecretIdフィルタリングが正常に動作することを検証 | Success |
| 3 | `test_cfrag_repository_ao_retrieve_failure` | AO取得失敗時にDomainErrorが返ることを検証 | Validation |

---

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: AOClientは通信のみを担当し、ビジネスロジックは含まない
- **Modular Design**: AOClient、エラー型、Repository拡張は独立したモジュールとして実装
- **Dependency Management**: AOClientはtraitとして定義し、実装詳細を隠蔽
- **Clear Interfaces**: Repository InterfaceとAOClient Interfaceを明確に分離
- **CosmWasm準拠**: 既存のメッセージ型（`ExecuteMsg`, `QueryMsg`）を再利用

### Performance
- **レスポンス時間**: AO通信のラウンドトリップは5秒以内
- **バッチ処理**: 複数kFrag送信時は並列処理で効率化
- **タイムアウト**: 設定可能なタイムアウト（デフォルト30秒）

### Security
- **秘密情報の保護**: kFrag送信時に秘密鍵が露出しないよう設計
- **メッセージ検証**: `ValidateMessage`トレイトによる整合性検証
- **エラー情報の制限**: エラーメッセージに秘密情報を含めない

### Reliability
- **リトライ機能**: 一時的な通信エラー時の自動リトライ（最大3回）
- **部分的失敗の処理**: バッチ操作での部分的失敗を適切にハンドリング
- **接続状態管理**: 接続の健全性チェック機能

### Usability
- **明確なエラーメッセージ**: `thiserror`の`#[error]`属性による開発者フレンドリーなエラー
- **非同期API**: async/awaitによる非ブロッキング操作
- **設定可能性**: タイムアウト、リトライ回数などを設定可能

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. AOClient基盤インターフェース + Mock | 9 | 11 | 100% |
| 2. KFragのAO送信機能 | 6 | 8 | 100% |
| 3. CFragのAO取得機能 | 6 | 8 | 100% |
| 4. CapsuleのArweave取得機能 | 4 | 4 | 100% |
| 5. AOCommunicationErrorエラー型 | 5 | 5 | 100% |
| 6. KFragRepository AOインターフェース拡張 | 3 | 3 | 100% |
| 7. CFragRepository AOインターフェース拡張 | 3 | 3 | 100% |
| **Total** | **36** | **42** | **100%** |

### Test Patterns Used

1. **Success Pattern**: 正常系の動作検証（ExecuteMsg/QueryMsg送受信）
2. **Validation Pattern**: ValidateMessageトレイトによる入力検証とエラーケース
3. **State Machine Pattern**: 通信状態の遷移検証
4. **Security Pattern**: 秘密情報の非露出確認
5. **Crypto Pattern**: kFrag/cFragのシリアライズ整合性
6. **Event Pattern**: Response内のEvents/Attributes検証

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test or N/A
- ❌ **Missing**: Test not implemented

## 参照ドキュメント

### AO Network通信（クライアント側）
- **AO Cookbook**: https://cookbook_ao.g8way.io/
  - aoconnect接続ガイド: https://cookbook_ao.g8way.io/guides/aoconnect/connecting.html
  - メッセージ送信: https://cookbook_ao.g8way.io/guides/aoconnect/sending-messages.html
  - 読み取りクエリ: https://cookbook_ao.g8way.io/guides/aoconnect/reading-results.html
- **@permaweb/aoconnect SDK**: https://www.npmjs.com/package/@permaweb/aoconnect
- **プロジェクト内ドキュメント**: `docs/PRD.md` セクション「AOネットワークの特性」

### AOプロセスAPI（呼び出し先）
- AOメッセージ型定義: `ao/contracts/src/msg.rs`
  - ExecuteMsg/QueryMsg: lines 1-87
  - ValidateMessage trait: lines 88-166
  - AOMessageTags: lines 216-250

### 注記
CosmWasmBookのパターン（Entry points, Execution messages等）はAOプロセス（コントラクト側）の設計指針であり、本ドキュメントではクライアントが呼び出すAPI仕様として参照している。クライアント側の実際の通信実装は`@permaweb/aoconnect` SDKを使用する。
