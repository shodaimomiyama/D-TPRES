# FORMIX KVストレージ仕様書

## 概要

本仕様書は、FORMIX（Deterministic Threshold Proxy Re-Encryption System）のCosmWasm AOプロセスにおけるKVストレージの設計・実装について定義します。

## アーキテクチャ設計

### 基本原則

1. **ステートレス実行**: メッセージ間でメモリ状態は保持されない
2. **イベントソーシング**: メッセージ履歴から状態を復元
3. **役割分離**: Owner/Holder/Requesterの3つの役割を明確に分離
4. **暗号学的安全性**: 秘密鍵材料の適切な管理と即座のメモリクリア

### プロセス間の関係

```
Owner-Process (Pᴼ)
├── kFrags生成・配布
├── Holder選出・管理
└── メタデータ管理

Holder-Process (Hⱼ) [j=1...n]
├── kFrag受信・保存
├── cFrag生成・保存
└── Capsule参照管理

Requester-Process (R-Proc)
├── cFrag収集
├── 閾値チェック
└── 復元状態管理
```

## ストレージ設計

### Owner-Process ストレージ

#### ストレージマップ定義

```rust
// Owner-Process専用ストレージ
pub const OWNER_KFRAGS: Map<String, OwnerKFragData> = Map::new("owner_kfrags");
pub const HOLDER_ASSIGNMENTS: Map<String, HolderAssignment> = Map::new("holder_assignments");
pub const OWNER_METADATA: Item<OwnerMetadata> = Item::new("owner_metadata");
pub const OWNER_CONFIG: Item<OwnerConfig> = Item::new("owner_config");
```

#### データ構造

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct OwnerKFragData {
    pub kfrag_id: String,
    pub encrypted_kfrag: Vec<u8>,  // AES-GCM暗号化済み
    pub target_holder: String,
    pub created_at: u64,
    pub distributed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderAssignment {
    pub holder_id: String,
    pub assigned_kfrags: Vec<String>,
    pub assignment_time: u64,
    pub status: AssignmentStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OwnerMetadata {
    pub threshold_k: u32,
    pub total_holders_n: u32,
    pub capsule_txid: String,
    pub requester_pubkey: String,
    pub creation_time: u64,
}
```

### Holder-Process ストレージ

#### ストレージマップ定義

```rust
// Holder-Process専用ストレージ
pub const HOLDER_KFRAGS: Map<String, HolderKFragData> = Map::new("holder_kfrags");
pub const HOLDER_CFRAGS: Map<String, HolderCFragData> = Map::new("holder_cfrags");
pub const CAPSULE_CACHE: Map<String, CapsuleData> = Map::new("capsule_cache");
pub const HOLDER_METADATA: Item<HolderMetadata> = Item::new("holder_metadata");
```

#### データ構造

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct HolderKFragData {
    pub kfrag_id: String,
    #[zeroize(skip)]
    pub source_owner: String,
    pub encrypted_kfrag: Vec<u8>,  // 暗号化されたkFragment
    pub signature: Vec<u8>,
    pub received_at: u64,
    pub processed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderCFragData {
    pub cfrag_id: String,
    pub source_kfrag: String,
    pub cfrag_data: Vec<u8>,
    pub arweave_txid: Option<String>,
    pub generated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapsuleData {
    pub capsule_id: String,
    pub capsule_bytes: Vec<u8>,
    pub arweave_txid: String,
    pub cached_at: u64,
}
```

### Requester-Process ストレージ

#### ストレージマップ定義

```rust
// Requester-Process専用ストレージ
pub const CFRAG_COLLECTION: Map<String, CFragCollection> = Map::new("cfrag_collection");
pub const RECOVERY_SESSIONS: Map<String, RecoverySession> = Map::new("recovery_sessions");
pub const THRESHOLD_TRACKER: Item<ThresholdInfo> = Item::new("threshold_tracker");
pub const REQUESTER_METADATA: Item<RequesterMetadata> = Item::new("requester_metadata");
```

#### データ構造

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragCollection {
    pub session_id: String,
    pub collected_cfrags: Vec<CollectedCFrag>,
    pub target_threshold: u32,
    pub collection_started: u64,
    pub status: CollectionStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectedCFrag {
    pub cfrag_id: String,
    pub holder_id: String,
    pub cfrag_data: Vec<u8>,
    pub collected_at: u64,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoverySession {
    pub session_id: String,
    pub capsule_data: Vec<u8>,
    pub collected_cfrags: Vec<String>,
    pub threshold_met: bool,
    pub recovery_completed: bool,
}
```

## メッセージハンドラ仕様

### 基本パターン

全てのメッセージハンドラは以下の3段階パターンに従います：

```rust
pub fn handle_message(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // 1. 状態ロード
    let current_state = load_process_state(deps.storage, &info.sender)?;

    // 2. ビジネスロジック実行
    let (new_state, response_data) = process_message(current_state, msg)?;

    // 3. 状態保存
    save_process_state(deps.storage, &info.sender, &new_state)?;

    Ok(Response::new()
        .add_attributes(response_data.attributes)
        .add_events(response_data.events))
}
```

### Owner-Process メッセージハンドラ

#### 1. kFrag配布ハンドラ

```rust
pub fn handle_distribute_kfrags(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    kfrags: Vec<KFragDistribution>,
) -> Result<Response, ContractError> {
    // 1. 現在のkFrag状態をロード
    let mut distributions = Vec::new();

    for kfrag_dist in kfrags {
        // kFrag検証
        validate_kfrag(&kfrag_dist)?;

        // ストレージに保存
        let kfrag_data = OwnerKFragData {
            kfrag_id: kfrag_dist.id.clone(),
            encrypted_kfrag: kfrag_dist.encrypted_data,
            target_holder: kfrag_dist.holder_id.clone(),
            created_at: env.block.time.seconds(),
            distributed: false,
        };

        OWNER_KFRAGS.save(deps.storage, kfrag_dist.id.clone(), &kfrag_data)?;

        // Holder割り当て記録
        let assignment = HolderAssignment {
            holder_id: kfrag_dist.holder_id,
            assigned_kfrags: vec![kfrag_dist.id],
            assignment_time: env.block.time.seconds(),
            status: AssignmentStatus::Pending,
        };

        HOLDER_ASSIGNMENTS.save(deps.storage, assignment.holder_id.clone(), &assignment)?;
        distributions.push(kfrag_dist.id);
    }

    Ok(Response::new()
        .add_attribute("action", "distribute_kfrags")
        .add_attribute("distributed_count", distributions.len().to_string()))
}
```

### Holder-Process メッセージハンドラ

#### 1. kFrag受信ハンドラ

```rust
pub fn handle_receive_kfrag(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    kfrag_data: KFragReceiptData,
) -> Result<Response, ContractError> {
    // 1. kFrag検証
    validate_kfrag_signature(&kfrag_data)?;

    // 2. ストレージに保存
    let holder_kfrag = HolderKFragData {
        kfrag_id: kfrag_data.id.clone(),
        source_owner: info.sender.to_string(),
        encrypted_kfrag: kfrag_data.encrypted_kfrag,
        signature: kfrag_data.signature,
        received_at: env.block.time.seconds(),
        processed: false,
    };

    HOLDER_KFRAGS.save(deps.storage, kfrag_data.id.clone(), &holder_kfrag)?;

    // 3. 自動的にcFrag生成をトリガー
    let cfrag_result = generate_cfrag(deps.storage, &kfrag_data.id)?;

    Ok(Response::new()
        .add_attribute("action", "receive_kfrag")
        .add_attribute("kfrag_id", kfrag_data.id)
        .add_attribute("cfrag_generated", cfrag_result.cfrag_id))
}
```

#### 2. cFrag生成ハンドラ

```rust
pub fn handle_generate_cfrag(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    kfrag_id: String,
) -> Result<Response, ContractError> {
    // 1. kFragをロード
    let kfrag_data = HOLDER_KFRAGS.load(deps.storage, kfrag_id.clone())?;

    // 2. Capsuleデータを取得（キャッシュまたはArweaveから）
    let capsule = get_or_fetch_capsule(deps.storage, &kfrag_data.source_owner)?;

    // 3. cFrag生成（暗号化処理）
    let cfrag_result = perform_reencryption(&kfrag_data.encrypted_kfrag, &capsule.capsule_bytes)?;

    // 4. cFragを保存
    let cfrag_data = HolderCFragData {
        cfrag_id: cfrag_result.id.clone(),
        source_kfrag: kfrag_id.clone(),
        cfrag_data: cfrag_result.data,
        arweave_txid: None, // 後でArweaveに保存
        generated_at: env.block.time.seconds(),
    };

    HOLDER_CFRAGS.save(deps.storage, cfrag_result.id.clone(), &cfrag_data)?;

    // 5. kFragの処理済みフラグを更新
    let mut updated_kfrag = kfrag_data;
    updated_kfrag.processed = true;
    HOLDER_KFRAGS.save(deps.storage, kfrag_id, &updated_kfrag)?;

    Ok(Response::new()
        .add_attribute("action", "generate_cfrag")
        .add_attribute("cfrag_id", cfrag_result.id))
}
```

### Requester-Process メッセージハンドラ

#### 1. cFrag収集ハンドラ

```rust
pub fn handle_collect_cfrags(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    session_id: String,
    threshold: u32,
) -> Result<Response, ContractError> {
    // 1. 既存セッション確認
    let mut collection = CFRAG_COLLECTION.may_load(deps.storage, session_id.clone())?
        .unwrap_or(CFragCollection {
            session_id: session_id.clone(),
            collected_cfrags: Vec::new(),
            target_threshold: threshold,
            collection_started: env.block.time.seconds(),
            status: CollectionStatus::InProgress,
        });

    // 2. 利用可能なcFrags検索
    let available_cfrags = query_available_cfrags()?;

    // 3. 閾値まで収集
    for cfrag in available_cfrags {
        if collection.collected_cfrags.len() >= threshold as usize {
            break;
        }

        let collected_cfrag = CollectedCFrag {
            cfrag_id: cfrag.id,
            holder_id: cfrag.holder_id,
            cfrag_data: cfrag.data,
            collected_at: env.block.time.seconds(),
            verified: true, // 暗号学的検証済み
        };

        collection.collected_cfrags.push(collected_cfrag);
    }

    // 4. 閾値チェック
    if collection.collected_cfrags.len() >= threshold as usize {
        collection.status = CollectionStatus::ThresholdMet;
    }

    // 5. セッション更新
    CFRAG_COLLECTION.save(deps.storage, session_id.clone(), &collection)?;

    Ok(Response::new()
        .add_attribute("action", "collect_cfrags")
        .add_attribute("session_id", session_id)
        .add_attribute("collected_count", collection.collected_cfrags.len().to_string())
        .add_attribute("threshold_met", collection.status.to_string()))
}
```

## セキュリティ考慮事項

### 1. メモリ安全性

#### Zeroize実装
```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecretData {
    #[zeroize(skip)]
    pub public_metadata: String,
    pub secret_material: Vec<u8>,  // 自動的にゼロクリア
}
```

#### 定数時間操作
```rust
use subtle::ConstantTimeEq;

fn verify_signature_constant_time(sig1: &[u8], sig2: &[u8]) -> bool {
    sig1.ct_eq(sig2).into()
}
```

### 2. 暗号化保存

#### AES-GCM暗号化
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};

pub fn encrypt_kfrag(kfrag: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(&generate_nonce());

    let ciphertext = cipher.encrypt(nonce, kfrag)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok(ciphertext)
}
```

### 3. アクセス制御

#### プロセス役割検証
```rust
pub fn verify_process_role(deps: &DepsMut, expected_role: ProcessRole) -> Result<(), ContractError> {
    let process_tags = get_process_tags(deps)?;

    if process_tags.role != expected_role {
        return Err(ContractError::UnauthorizedRole);
    }

    Ok(())
}
```

## エラーハンドリング

### カスタムエラー定義

```rust
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized role: expected {expected}, got {actual}")]
    UnauthorizedRole { expected: String, actual: String },

    #[error("KFrag not found: {id}")]
    KFragNotFound { id: String },

    #[error("Insufficient cFrags: need {required}, have {available}")]
    InsufficientCFrags { required: u32, available: u32 },

    #[error("Threshold not met: {current}/{required}")]
    ThresholdNotMet { current: u32, required: u32 },

    #[error("Cryptographic operation failed")]
    CryptographicError,

    #[error("Invalid signature")]
    InvalidSignature,
}
```

## テスト仕様

### 単体テスト

1. **ストレージ操作テスト**
   - KVの読み書き操作
   - データシリアライゼーション
   - エラーケース

2. **メッセージハンドラテスト**
   - 各ハンドラの正常系
   - エラーハンドリング
   - 状態遷移

### 統合テスト

1. **状態復元テスト**
   - メッセージ履歴からの完全復元
   - 部分的メッセージ再生
   - 破損データからの回復

2. **TPRE フローテスト**
   - Owner → Holder → Requester の完全フロー
   - k-of-n 閾値動作
   - 暗号学的整合性

### セキュリティテスト

1. **メモリ安全性テスト**
   - Zeroize動作確認
   - メモリリーク検出
   - 秘密データの適切なクリア

2. **暗号化テスト**
   - 暗号化・復号化の整合性
   - 鍵管理の適切性
   - 定数時間操作の検証

## 実装ガイドライン

### コード規約

1. **命名規則**
   - 構造体: PascalCase
   - 関数: snake_case
   - 定数: SCREAMING_SNAKE_CASE

2. **エラーハンドリング**
   - `?` 演算子を積極活用
   - カスタムエラー型でコンテキスト提供
   - パニックは絶対に発生させない

3. **テスト**
   - 全ての公開関数に対するテスト
   - エッジケースの網羅
   - セキュリティ関連のテスト強化

### パフォーマンス考慮

1. **メモリ使用量**
   - 大容量データはArweave参照で管理
   - 不要になったデータの即座解放
   - メモリプールの効率的利用

2. **処理時間**
   - 暗号化処理の最適化
   - 不要な計算の排除
   - キャッシュ機構の活用

---

この仕様書に基づいて、FORMIXのKVストレージ実装を行います。各モジュールは独立してテスト可能で、CosmWasm AOのステートレス実行モデルに完全に対応した設計となっています。