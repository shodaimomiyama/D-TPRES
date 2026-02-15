# Owner Process 動作詳細仕様書

## 概要
本ドキュメントは、D-TPRES (Deterministic Threshold Proxy Re-Encryption System) におけるOwner ProcessのAO Network上での動作を、実装コードの参照と共にステップバイステップで解説します。

## Owner Process 動作フロー図

### 全体フロー（テキスト形式）

```
[O-Browser]
    |
    | 1. Spawn Process (WASM TX ID指定)
    v
[AO Network]
    |
    | 2. Instantiate (ProcessRole::Owner)
    v
[Owner Process 初期化]
    |
    | - owner_id設定
    | - total_holders_n設定
    | - signer_pubkey設定
    | - ユニバーサル初期化（全ロールのメタデータ保存）
    v
[Owner Process 待機状態]
    |
    | 3. ExecuteMsg::ReceiveKFrags
    |    (from O-Browser)
    v
[kFrag受信処理]
    |
    | 4. バリデーション
    | - kFrag ID確認
    | - kFragデータ確認
    | - 署名検証 (signer_pubkey使用)
    v
[RandAO Holder選出]
    |
    | 5. n個のHolderをランダム選出
    | - selection_seed生成
    | - block_height記録
    v
[kFrag保存と配布準備]
    |
    | 6. 各kFragに対して:
    | - OWNER_KFRAGSに保存
    | - target_holder割り当て
    | - HOLDER_ASSIGNMENTSに記録
    v
[AO Networkメッセージ生成]
    |
    | 7. 各Holderに対して:
    | - ExecuteMsg::SendKFragToHolder作成
    | - KFragDistribution構造体生成
    v
[メッセージ送信]
    |
    | 8. CosmWasmメッセージとして送信
    | - 各Holder Processへ
    | - kFrag（公開情報）と署名を送信
    v
[配布結果記録]
    |
    | 9. Response生成
    | - distributed_count
    | - failed_distributions
    | - selected_holders
    v
[Owner Process 完了]
```

### 詳細な処理フロー説明

#### Phase 0: プロセスの初期化
1. **O-Browserによるプロセス起動**
   - O-Browser（オーナー側のブラウザアプリケーション）が、Arweaveに保存されているWASMバイナリのTransaction IDを指定して、AO Network上に新しいプロセスをspawnします
   - この時点で、プロセスはまだ役割が決定していない状態です

2. **Instantiateメッセージによる初期化**
   - ProcessRole::Ownerを指定してInstantiateMsg送信
   - owner_id（オーナー識別子）、total_holders_n（Holder総数）、signer_pubkey（署名検証用公開鍵）を設定
   - ユニバーサル初期化により、Owner/Holder/Requesterすべてのメタデータストレージが初期化されます（他のロールはデフォルト値）

#### Phase 1: kFragの受信と検証
3. **O-BrowserからkFrags受信**
   - O-Browserが生成したkFrag（再暗号化鍵の断片）と署名をExecuteMsg::ReceiveKFragsメッセージとして受信
   - 複数のkFragを一度に受信可能（Vec<KFragWithSignature>）

4. **入力データの検証**
   - 各kFragのIDが空でないことを確認
   - kFragデータ（バイナリ）が存在することを確認
   - 署名データが存在することを確認
   - 保存されているsigner_pubkeyを使用して、各kFragの署名を検証
   - 検証に失敗した場合はエラーを返し、処理を中断

#### Phase 2: Holder選出と配布
5. **RandAOによるHolder選出（プレースホルダー実装対応）**
   - **プレースホルダー実装**: `holder_process_ids`が設定されている場合、事前定義されたHolder Processを使用
   - **本番実装**: `holder_process_ids`がnullの場合、total_holders_n個のHolder ProcessをRandAO（ランダム選出アルゴリズム）で選出
   - 現在のblock_heightをシードとして使用し、決定論的にランダム選出
   - selection_resultに選出されたHolder IDリストを保存

   **プレースホルダー実装の目的**:
   - 開発・テスト環境で確実に特定のHolder Processにアクセス可能
   - RandAOの実装を待たずにEnd-to-Endテストが実行可能
   - 本番環境への移行が容易（`holder_process_ids: null`にするだけ）

6. **kFragの保存と割り当て**
   - 各kFragをOWNER_KFRAGSストレージに保存（kFragは公開鍵として機能するため暗号化は不要）
   - 選出されたHolderに順番に割り当て
   - HOLDER_ASSIGNMENTSストレージに割り当て情報を記録
   - 割り当て状態はPending→Confirmed/Failedと遷移

#### Phase 3: AO Network経由での配布
7. **プロセス間メッセージの生成**
   - 各Holder Processに対してExecuteMsg::SendKFragToHolderメッセージを生成
   - KFragDistribution構造体にkFrag情報、署名、送信先情報をパッケージ

8. **CosmWasmメッセージ送信**
   - AO NetworkのCosmWasm実行環境を通じて、各Holder Processにメッセージを送信
   - kFragは公開鍵として機能するため、平文で送信される（セキュリティ上問題なし）
   - 非同期的に処理され、Holder側で受信・保存される

#### Phase 4: 配布結果の管理
9. **統計情報の記録とレスポンス**
   - distributed_count: 成功した配布数
   - failed_distributions: 失敗した配布のエラー詳細
   - selected_holders: 選出されたHolderリスト
   - レスポンス属性としてこれらの情報を返却

## 1. プロセスの初期化 (Phase 0)

### 1.1 プロセスのSpawn
O-BrowserがAO Network上にOwner Processをspawnします。

### 1.2 Instantiate関数による初期化

**該当コード: `src/contract.rs:103-195`**

```rust
// src/contract.rs:103-195
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    // メッセージバリデーション
    msg.validate()
        .map_err(|e| ContractError::ValidationError { msg: e })?;

    // 全てのロールのメタデータをデフォルト値で初期化
    let mut owner_metadata = OwnerMetadata::default();
    let mut holder_metadata = HolderMetadata::default();
    let mut requester_metadata = RequesterMetadata::default();

    // Owner Processとして初期化
    match msg.process_role {
        ProcessRole::Owner => {
            if let ProcessMetadata::Owner {
                owner_id,
                total_holders_n,
                signer_pubkey,
            } = msg.metadata
            {
                owner_metadata = OwnerMetadata {
                    owner_id: owner_id.clone(),
                    total_holders_n,
                    creation_time: env.block.time.seconds(),
                    signer_pubkey: signer_pubkey.clone(),
                };

                let owner_config = OwnerConfig {
                    process_role: ProcessRole::Owner,
                };
                OWNER_CONFIG.save(deps.storage, &owner_config)?;
            }
        }
        // ... 他のロール処理
    }

    // 全てのメタデータを保存（ユニバーサル初期化）
    OWNER_METADATA.save(deps.storage, &owner_metadata)?;
    HOLDER_METADATA.save(deps.storage, &holder_metadata)?;
    REQUESTER_METADATA.save(deps.storage, &requester_metadata)?;
}
```

### 1.3 初期化時に設定されるメタデータ

**データ構造定義: `src/state.rs:64-81`**

```rust
// src/state.rs:64-81
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OwnerMetadata {
    pub owner_id: String,
    pub total_holders_n: u32,        // RandAOでHolder選出に必要
    pub creation_time: u64,
    pub signer_pubkey: String,       // O-Browserの署名検証用公開鍵
}

impl Default for OwnerMetadata {
    fn default() -> Self {
        Self {
            owner_id: String::new(),
            total_holders_n: 0,
            creation_time: 0,
            signer_pubkey: String::new(),
        }
    }
}
```

## 2. kFragの受信と処理 (Phase 1-2)

### 2.1 ExecuteMsgによるkFrag受信

**メッセージルーティング: `src/contract.rs:199-204`**

```rust
// src/contract.rs:199-204
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    match msg {
        // Owner-Process メッセージ (PHASE 2のみ)
        ExecuteMsg::ReceiveKFrags { kfrags } => {
            handle_receive_kfrags(deps, env, info, kfrags)
        }
        // ... 他のメッセージ処理
    }
}
```

### 2.2 kFrag受信ハンドラの処理フロー

**該当コード: `src/owner/handlers.rs:55-240`**

```rust
// src/owner/handlers.rs:55-100
pub fn handle_receive_kfrags(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    kfrags: Vec<KFragWithSignature>,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_owner_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Owner".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 現在の状態をロード
    let _current_state = load_owner_state(deps.as_ref(), env.block.time)
        .map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 3. metadataロード
    let metadata = OWNER_METADATA.load(deps.storage)?;

    // 4. kFragsと署名のバリデーション
    for kfrag in &kfrags {
        if kfrag.kfrag_id.is_empty() {
            return Err(ContractError::ValidationError {
                msg: "kFrag ID cannot be empty".to_string()
            });
        }
        if kfrag.kfrag_data.is_empty() {
            return Err(ContractError::ValidationError {
                msg: "kFrag data cannot be empty".to_string()
            });
        }
        if kfrag.signature.is_empty() {
            return Err(ContractError::ValidationError {
                msg: "Signature cannot be empty".to_string()
            });
        }
        // 署名検証の実装
        let is_valid = verify_kfrag_signature(&kfrag, &metadata.signer_pubkey)?;
        if !is_valid {
            return Err(ContractError::ValidationError {
                msg: format!("Invalid signature for kFrag: {}", kfrag.kfrag_id)
            });
        }
    }
```

### 2.3 RandAO選出とHolder割り当て（プレースホルダー実装対応）

**該当コード: `src/owner/handlers.rs:101-150`**

```rust
// src/owner/handlers.rs:101-150
    // 5. RandAO選出（プレースホルダー実装対応）
    let selection_result = perform_randao_selection(
        metadata.total_holders_n,
        env.block.height,
        metadata.holder_process_ids.clone(), // プレースホルダー実装: 事前定義されたHolder ProcessのIDを使用
    )?;

    // プレースホルダー実装のperform_randao_selection関数の詳細
    // `src/owner/handlers.rs:359-390`に実装されている:
    fn perform_randao_selection(
        total_holders_n: u32,
        block_height: u64,
        predefined_holders: Option<Vec<String>>, // プレースホルダー実装の新しいパラメータ
    ) -> Result<RandAOSelection, ContractError> {
        // プレースホルダー実装: 事前定義されたHolderがある場合はそれを使用
        if let Some(holders) = predefined_holders {
            return Ok(RandAOSelection {
                selected_holders: holders,
                selection_seed: "placeholder_seed".to_string(),
                block_height,
            });
        }

        // 従来のRandAO実装（省略）
        // ...
    }

    // 6. kFrags保存と配布準備
    let mut distribution_result = KFragDistributionResult {
        distributed_count: 0,
        failed_distributions: Vec::new(),
        selected_holders: selection_result.selected_holders.clone(),
    };

    for (index, kfrag) in kfrags.iter().enumerate() {
        let target_holder = if index < selection_result.selected_holders.len() {
            selection_result.selected_holders[index].clone()
        } else {
            // フォールバック: ランダム選出が足りない場合
            format!("holder_process_{}", index)
        };

        match store_kfrag(
            deps.branch(),
            kfrag.kfrag_id.clone(),
            kfrag.kfrag_data.clone(),
            target_holder.clone(),
            env.block.time,
        ) {
            Ok(_) => {
                // Holder割り当て記録
                if let Err(e) = assign_holder(
                    deps.branch(),
                    target_holder.clone(),
                    vec![kfrag.kfrag_id.clone()],
                    env.block.time,
                ) {
                    distribution_result.failed_distributions.push(
                        format!("Failed to assign holder {}: {}", target_holder, e)
                    );
                } else {
                    distribution_result.distributed_count += 1;
                    response_attributes.push((
                        format!("kfrag_{}_assigned_to", kfrag.kfrag_id),
                        target_holder,
                    ));
                }
            }
            // エラー処理...
        }
    }
```

### 2.4 AO Networkメッセージの生成と送信

**該当コード: `src/owner/handlers.rs:151-200`**

```rust
// src/owner/handlers.rs:151-200
    // 6. AO Networkメッセージ生成
    let mut cosmos_messages = Vec::new();
    for (index, kfrag) in kfrags.iter().enumerate() {
        let target_holder = if index < selection_result.selected_holders.len() {
            selection_result.selected_holders[index].clone()
        } else {
            format!("holder_process_{}", index)
        };

        // Holder-Processへの送信メッセージ
        let kfrag_distribution = KFragDistribution {
            id: kfrag.kfrag_id.clone(),
            encrypted_data: kfrag.kfrag_data.clone(), // 注：名前は"encrypted_data"だが、kFragは公開鍵として機能するため実際には暗号化不要
            holder_id: target_holder.clone(),
            holder_process_id: target_holder.clone(),
            signature: kfrag.signature.clone(),
        };

        // ExecuteMsg::SendKFragToHolderメッセージを作成
        let holder_msg = ExecuteMsg::SendKFragToHolder {
            target_process: target_holder.clone(),
            kfrag: kfrag_distribution,
            owner_process: info.sender.to_string(),
        };

        // CosmWasmメッセージとして送信
        let cosmos_msg = OwnerAOIntegration::send_kfrag_to_holder(
            target_holder.clone(),
            holder_msg,
        )?;
        cosmos_messages.push(cosmos_msg);
    }
```

## 3. データ構造とストレージ

### 3.1 kFragデータ構造

**メッセージ定義: `src/msg.rs:132-146`**

```rust
// src/msg.rs:132-146
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KFragWithSignature {
    pub kfrag_id: String,
    pub kfrag_data: Vec<u8>,      // kFragのバイナリデータ（公開鍵として機能）
    pub signature: Vec<u8>,       // O-Browserによる署名
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KFragDistribution {
    pub id: String,
    pub encrypted_data: Vec<u8>,  // 名前は"encrypted_data"だが、kFragは公開鍵のため暗号化不要
    pub holder_id: String,
    pub holder_process_id: String,  // AO Network用: Holder ProcessのID
    pub signature: Vec<u8>,
}
```

### 3.2 ストレージ定義

**ストレージ定義: `src/state.rs:5-9`**

```rust
// src/state.rs:5-9
// Owner-Process専用ストレージ
pub const OWNER_KFRAGS: Map<String, OwnerKFragData> = Map::new("owner_kfrags");
pub const HOLDER_ASSIGNMENTS: Map<String, HolderAssignment> = Map::new("holder_assignments");
pub const OWNER_METADATA: Item<OwnerMetadata> = Item::new("owner_metadata");
pub const OWNER_CONFIG: Item<OwnerConfig> = Item::new("owner_config");
```

### 3.3 kFragストレージデータ構造

**該当コード: `src/state.rs:37-47`**

```rust
// src/state.rs:37-47
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct OwnerKFragData {
    pub kfrag_id: String,
    pub encrypted_kfrag: Vec<u8>, // 名前は"encrypted_kfrag"だが、kFragは公開鍵として機能するため暗号化不要
    #[zeroize(skip)]
    pub target_holder: String,
    #[zeroize(skip)]
    pub created_at: u64,
    #[zeroize(skip)]
    pub distributed: bool,
}
```

## 4. プロセス間通信

### 4.1 Holder-Processへのメッセージ送信

**該当コード: `src/contract.rs:257-285`**

```rust
// src/contract.rs:257-285
/// Owner-Process → Holder-Process へのkFrag送信
fn execute_send_kfrag_to_holder(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    target_process: String,
    kfrag: crate::msg::KFragDistribution,
    owner_process: String,
) -> ContractResult {
    // Holder-Processでのみ処理
    verify_process_role(deps.as_ref(), ProcessRole::Holder)?;

    // Owner-Processからの送信であることを確認
    if info.sender.to_string() != owner_process {
        return Err(ContractError::Unauthorized {
            msg: format!("Expected message from owner {}, got {}", owner_process, info.sender),
        });
    }

    // kFragを受信処理
    let kfrag_receipt = crate::msg::KFragReceiptData {
        id: kfrag.id,
        encrypted_kfrag: kfrag.encrypted_data,
        signature: kfrag.signature,
        owner_id: owner_process,
    };

    handle_receive_kfrag(deps, env, info, kfrag_receipt)
}
```

### 4.2 AO統合ヘルパー関数

**該当コード: `src/ao_integration.rs:16-28`**

```rust
// src/ao_integration.rs:16-28
impl AOIntegration {
    /// プロセス間メッセージを送信
    pub fn send_ao_message(
        target_process: String,
        message: ExecuteMsg,
    ) -> Result<CosmosMsg, StdError> {
        let wasm_msg = WasmMsg::Execute {
            contract_addr: target_process,
            msg: to_json_binary(&message)?,
            funds: vec![],
        };

        Ok(CosmosMsg::Wasm(wasm_msg))
    }
}
```

## 5. クエリ機能

### 5.1 利用可能なクエリ

**クエリメッセージ定義: `src/msg.rs:91-96`**

```rust
// src/msg.rs:91-96
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Owner-Process クエリ
    GetOwnerMetadata {},
    GetKFrags {
        holder_id: Option<String>,
    },
    GetHolderAssignments {},
    // ... 他のクエリ
}
```

## 6. セキュリティ実装

### 6.1 Zeroizeによるメモリ管理

**該当コード: `src/state.rs:37,41`**

```rust
// src/state.rs:37,41
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct OwnerKFragData {
    // 機密データは自動的にメモリから消去される
}

#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct KFragDistributionResult {
    // 配布結果も安全に管理
}
```

### 6.2 署名検証

**該当コード: `src/owner/handlers.rs:92-98`**

```rust
// src/owner/handlers.rs:92-98
// 署名検証の実装
let is_valid = verify_kfrag_signature(&kfrag, &metadata.signer_pubkey)?;
if !is_valid {
    return Err(ContractError::ValidationError {
        msg: format!("Invalid signature for kFrag: {}", kfrag.kfrag_id)
    });
}
```

## 7. テスト実装

### 7.1 Owner Process初期化テスト

**該当コード: `src/tests.rs:11-29`**

```rust
// src/tests.rs:11-29 (RandAO使用)
#[test]
fn test_owner_instantiate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &coins(1000, "earth"));

    let msg = InstantiateMsg {
        process_role: ProcessRole::Owner,
        metadata: ProcessMetadata::Owner {
            owner_id: "test_owner".to_string(),
            total_holders_n: 5,
            signer_pubkey: "test_pubkey".to_string(),
            holder_process_ids: None, // RandAO使用
        },
    };

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(res.attributes.len(), 5);
    assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "instantiate_owner"));
    assert!(res.attributes.iter().any(|attr| attr.key == "universal_init" && attr.value == "true"));
}

// プレースホルダー実装のテスト (src/tests.rs:176-210)
#[test]
fn test_owner_instantiate_with_predefined_holders() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &coins(1000, "earth"));

    // プレースホルダー実装：事前定義されたHolder ProcessのIDを使用
    let predefined_holders = vec![
        "holder_process_001".to_string(),
        "holder_process_002".to_string(),
        "holder_process_003".to_string(),
    ];

    let msg = InstantiateMsg {
        process_role: ProcessRole::Owner,
        metadata: ProcessMetadata::Owner {
            owner_id: "test_owner_with_predefined".to_string(),
            total_holders_n: 3,
            signer_pubkey: "test_pubkey".to_string(),
            holder_process_ids: Some(predefined_holders.clone()),
        },
    };

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // メタデータが正しく保存されていることを確認
    let owner_meta = OWNER_METADATA.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner_meta.holder_process_ids, Some(predefined_holders));
}
```

### 7.2 メタデータクエリテスト

**該当コード: `src/tests.rs:90-114`**

```rust
// src/tests.rs:90-114
#[test]
fn test_query_owner_metadata() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &coins(1000, "earth"));

    // まずOwnerプロセスを初期化
    let instantiate_msg = InstantiateMsg {
        process_role: ProcessRole::Owner,
        metadata: ProcessMetadata::Owner {
            owner_id: "test_owner".to_string(),
            total_holders_n: 5,
            signer_pubkey: "test_pubkey".to_string(),
        },
    };

    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // メタデータをクエリ
    let query_msg = QueryMsg::GetOwnerMetadata {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();

    let metadata_response: crate::msg::OwnerMetadataResponse = from_json(&res).unwrap();
    assert_eq!(metadata_response.metadata.owner_id, "test_owner");
    assert_eq!(metadata_response.metadata.total_holders_n, 5);
}
```

## 8. AO Network特有の制約と対応

### 8.1 ステートレス実行への対応

各メッセージ処理は以下のパターンに従います：

```rust
// パターン例
pub fn handle_message(msg: AOMessage, repo: &dyn Repository) -> Result<Response> {
    // 1. 状態をロード
    let mut state = repo.load_state(msg.process_id)?;

    // 2. メッセージを処理
    let result = process_with_state(&mut state, msg)?;

    // 3. 状態を永続化
    repo.save_state(msg.process_id, &state)?;

    Ok(result)
}
```

### 8.2 同期処理のみの制約

- async/awaitは使用不可
- すべての処理はブロッキング
- エラーハンドリングは同期的に実行

## まとめ

Owner Processは、D-TPRESシステムの中核として、以下の責務を持ちます：

1. **kFragの受信と検証**: O-Browserから受信したkFragと署名を検証
2. **Holder選出**: RandAOを使用してn個のHolderをランダムに選出（プレースホルダー実装対応）
3. **kFrag配布**: 選出されたHolderにkFragを配布
4. **状態管理**: 配布状態とHolder割り当てを管理
5. **セキュリティ**: Zeroizeによる機密データの安全な管理と署名検証

### プレースホルダー実装について

**開発・テスト段階での利便性を考慮し、以下の機能を実装しています：**

- **事前定義Holder支援**: `holder_process_ids`パラメータで特定のHolder ProcessのIDを事前指定可能
- **RandAOバイパス**: プレースホルダー実装により、RandAOの実装を待たずにシステムテストが可能
- **簡単な本番移行**: `holder_process_ids: null`に設定するだけで本番RandAO実装に切り替え可能

**使用方法:**
```rust
// プレースホルダー実装（開発・テスト用）
holder_process_ids: Some(vec![
    "holder_process_001".to_string(),
    "holder_process_002".to_string(),
    "holder_process_003".to_string()
])

// 本番実装（RandAO使用）
holder_process_ids: None
```

すべての処理はAO Networkのステートレス実行モデルに準拠し、メッセージドリブンアーキテクチャで実装されています。