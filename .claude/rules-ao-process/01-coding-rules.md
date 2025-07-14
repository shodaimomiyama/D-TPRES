# AO Process Coding Rules

## AOプロセス実装のコーディング規約

このドキュメントは、D-TPRES暗号システムのAOネットワークプロセス実装におけるコーディング規約を定義します。

## 1. 基本原則

### 1.1 メッセージ駆動設計
```rust
// すべての処理はメッセージをトリガーとする
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AOMessage {
    pub id: String,
    pub from: String,
    pub action: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

// メッセージハンドラの基本構造
pub fn handle_message(msg: AOMessage) -> Result<AOResponse, ProcessError> {
    match msg.action.as_str() {
        "init" => handle_init(msg),
        "split" => handle_split(msg),
        "generate_kfrag" => handle_kfrag_generation(msg),
        "reencrypt" => handle_reencryption(msg),
        _ => Err(ProcessError::UnknownAction(msg.action)),
    }
}
```

### 1.2 同期処理の原則
```rust
// ❌ 悪い例: 非同期処理（AOでは不可）
async fn process_data(data: &[u8]) -> Result<Vec<u8>, Error> {
    // async/awaitは使用不可
}

// ✅ 良い例: 同期処理
fn process_data(data: &[u8]) -> Result<Vec<u8>, Error> {
    // すべて同期的に処理
    validate_data(data)?;
    transform_data(data)
}
```

## 2. メッセージ型定義

### 2.1 アクション型の定義
```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageAction {
    // Owner-Process actions
    Init,
    Split,
    GenerateKFrag,
    
    // Holder-Process actions
    StoreKFrag,
    Reencrypt,
    
    // Requester-Process actions
    RequestAccess,
    CollectCFrag,
    
    // Common actions
    Query,
    Health,
}

impl MessageAction {
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "init" => Ok(MessageAction::Init),
            "split" => Ok(MessageAction::Split),
            "generate_kfrag" => Ok(MessageAction::GenerateKFrag),
            "store_kfrag" => Ok(MessageAction::StoreKFrag),
            "reencrypt" => Ok(MessageAction::Reencrypt),
            "request_access" => Ok(MessageAction::RequestAccess),
            "collect_cfrag" => Ok(MessageAction::CollectCFrag),
            "query" => Ok(MessageAction::Query),
            "health" => Ok(MessageAction::Health),
            _ => Err(ParseError::UnknownAction(s.to_string())),
        }
    }
}
```

### 2.2 メッセージデータ型
```rust
// 型安全なメッセージデータ
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MessageData {
    Init {
        process_role: ProcessRole,
        threshold: usize,
        total_shares: usize,
    },
    Split {
        secret_id: String,
        metadata: SplitMetadata,
    },
    KFragGeneration {
        requester_pk: PublicKey,
        threshold: usize,
    },
    Reencrypt {
        capsule: Capsule,
        cfrag_request: CFragRequest,
    },
    Query {
        query_type: QueryType,
        parameters: serde_json::Value,
    },
}

// 使用例
fn handle_typed_message(action: MessageAction, data: MessageData) -> Result<AOResponse, ProcessError> {
    match (action, data) {
        (MessageAction::Init, MessageData::Init { process_role, threshold, total_shares }) => {
            handle_init_process(process_role, threshold, total_shares)
        },
        (MessageAction::Split, MessageData::Split { secret_id, metadata }) => {
            handle_secret_split(&secret_id, metadata)
        },
        _ => Err(ProcessError::MessageActionDataMismatch),
    }
}
```

## 3. プロセス状態管理

### 3.1 状態構造の定義
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessState {
    pub process_id: String,
    pub role: ProcessRole,
    pub phase: CryptoPhase,
    pub created_at: u64,
    pub updated_at: u64,
    
    // ロール固有の状態
    pub owner_state: Option<OwnerState>,
    pub holder_state: Option<HolderState>,
    pub requester_state: Option<RequesterState>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OwnerState {
    pub master_key_id: Option<String>,
    pub shares_distributed: Vec<ShareInfo>,
    pub kfrags_generated: Vec<KFragInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HolderState {
    pub holder_id: u32,
    pub stored_kfrags: Vec<StoredKFrag>,
    pub reencryption_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequesterState {
    pub access_requests: Vec<AccessRequest>,
    pub collected_cfrags: Vec<CollectedCFrag>,
}
```

### 3.2 状態遷移管理
```rust
impl ProcessState {
    pub fn new(role: ProcessRole) -> Self {
        let now = current_timestamp();
        Self {
            process_id: generate_process_id(),
            role,
            phase: CryptoPhase::Initialized,
            created_at: now,
            updated_at: now,
            owner_state: if role == ProcessRole::Owner { Some(OwnerState::default()) } else { None },
            holder_state: if role == ProcessRole::Holder { Some(HolderState::default()) } else { None },
            requester_state: if role == ProcessRole::Requester { Some(RequesterState::default()) } else { None },
        }
    }
    
    pub fn transition_to(&mut self, new_phase: CryptoPhase) -> Result<(), StateError> {
        if !self.is_valid_transition(&new_phase) {
            return Err(StateError::InvalidTransition {
                from: self.phase.clone(),
                to: new_phase,
            });
        }
        
        self.phase = new_phase;
        self.updated_at = current_timestamp();
        Ok(())
    }
    
    fn is_valid_transition(&self, new_phase: &CryptoPhase) -> bool {
        use CryptoPhase::*;
        match (&self.role, &self.phase, new_phase) {
            // Owner-Process transitions
            (ProcessRole::Owner, Initialized, KeyGeneration) => true,
            (ProcessRole::Owner, KeyGeneration, SecretSplitting) => true,
            (ProcessRole::Owner, SecretSplitting, KFragGeneration) => true,
            
            // Holder-Process transitions
            (ProcessRole::Holder, Initialized, Ready) => true,
            (ProcessRole::Holder, Ready, Reencrypting) => true,
            (ProcessRole::Holder, Reencrypting, Ready) => true,
            
            // Requester-Process transitions
            (ProcessRole::Requester, Initialized, RequestingAccess) => true,
            (ProcessRole::Requester, RequestingAccess, CollectingCFrags) => true,
            (ProcessRole::Requester, CollectingCFrags, Completed) => true,
            
            _ => false,
        }
    }
}
```

## 4. エラーハンドリング

### 4.1 プロセス固有エラー型
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessError {
    // メッセージ関連エラー
    InvalidMessage(String),
    UnknownAction(String),
    MessageActionDataMismatch,
    DeserializationFailed(String),
    
    // 状態関連エラー
    InvalidState(String),
    StateTransitionFailed { from: CryptoPhase, to: CryptoPhase },
    StatePersistenceFailed(String),
    
    // 暗号関連エラー
    CryptoOperationFailed(String),
    InvalidThreshold,
    KeyGenerationFailed,
    
    // AO固有エラー
    AOMessageFailed(String),
    ProcessSpawnFailed(String),
    MemoryLimitExceeded,
}

impl ProcessError {
    pub fn to_response(&self) -> AOResponse {
        AOResponse::error(&self.to_string())
    }
    
    pub fn error_code(&self) -> u32 {
        match self {
            ProcessError::InvalidMessage(_) => 1001,
            ProcessError::UnknownAction(_) => 1002,
            ProcessError::MessageActionDataMismatch => 1003,
            ProcessError::InvalidState(_) => 2001,
            ProcessError::StateTransitionFailed { .. } => 2002,
            ProcessError::CryptoOperationFailed(_) => 3001,
            ProcessError::InvalidThreshold => 3002,
            ProcessError::AOMessageFailed(_) => 4001,
            ProcessError::MemoryLimitExceeded => 4002,
            _ => 9999,
        }
    }
}
```

### 4.2 レスポンス構造
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AOResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<ErrorInfo>,
    pub timestamp: u64,
    pub process_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorInfo {
    pub code: u32,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl AOResponse {
    pub fn success<T: Serialize>(data: T, process_id: String) -> Self {
        Self {
            success: true,
            data: Some(serde_json::to_value(data).unwrap()),
            error: None,
            timestamp: current_timestamp(),
            process_id,
        }
    }
    
    pub fn error(error: &ProcessError, process_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorInfo {
                code: error.error_code(),
                message: error.to_string(),
                details: None,
            }),
            timestamp: current_timestamp(),
            process_id,
        }
    }
}
```

## 5. メッセージハンドラ実装

### 5.1 Owner-Processハンドラ
```rust
pub struct OwnerHandler {
    state: ProcessState,
}

impl OwnerHandler {
    pub fn new() -> Self {
        Self {
            state: ProcessState::new(ProcessRole::Owner),
        }
    }
    
    pub fn handle_init(&mut self, threshold: usize, total_shares: usize) -> Result<InitResponse, ProcessError> {
        // 閾値パラメータの検証
        if threshold == 0 || threshold > total_shares {
            return Err(ProcessError::InvalidThreshold);
        }
        
        // 状態遷移
        self.state.transition_to(CryptoPhase::KeyGeneration)?;
        
        // マスターキーペア生成
        let keypair = generate_master_keypair()
            .map_err(|e| ProcessError::KeyGenerationFailed)?;
        
        // 状態の更新
        if let Some(ref mut owner_state) = self.state.owner_state {
            owner_state.master_key_id = Some(keypair.id.clone());
        }
        
        self.state.transition_to(CryptoPhase::Ready)?;
        
        Ok(InitResponse {
            public_key: keypair.public_key,
            process_id: self.state.process_id.clone(),
        })
    }
    
    pub fn handle_split(&mut self, secret_data: SplitData) -> Result<SplitResponse, ProcessError> {
        // 前提条件チェック
        if self.state.phase != CryptoPhase::Ready {
            return Err(ProcessError::InvalidState("Not ready for splitting".to_string()));
        }
        
        // 秘密分散実行
        let shares = perform_secret_splitting(&secret_data)
            .map_err(|e| ProcessError::CryptoOperationFailed(e.to_string()))?;
        
        // 状態更新
        self.state.transition_to(CryptoPhase::SecretSplitting)?;
        
        if let Some(ref mut owner_state) = self.state.owner_state {
            owner_state.shares_distributed = shares.iter().map(|s| ShareInfo::from(s)).collect();
        }
        
        Ok(SplitResponse {
            shares_count: shares.len(),
            distribution_complete: true,
        })
    }
}
```

### 5.2 Holder-Processハンドラ
```rust
pub struct HolderHandler {
    state: ProcessState,
}

impl HolderHandler {
    pub fn new(holder_id: u32) -> Self {
        let mut state = ProcessState::new(ProcessRole::Holder);
        if let Some(ref mut holder_state) = state.holder_state {
            holder_state.holder_id = holder_id;
        }
        
        Self { state }
    }
    
    pub fn handle_store_kfrag(&mut self, kfrag_data: KFragData) -> Result<StorageResponse, ProcessError> {
        // kFragの検証
        validate_kfrag(&kfrag_data.kfrag)
            .map_err(|e| ProcessError::CryptoOperationFailed(e.to_string()))?;
        
        // ストレージに保存
        let stored_kfrag = StoredKFrag {
            id: kfrag_data.id,
            kfrag: kfrag_data.kfrag,
            stored_at: current_timestamp(),
        };
        
        if let Some(ref mut holder_state) = self.state.holder_state {
            holder_state.stored_kfrags.push(stored_kfrag);
        }
        
        self.state.transition_to(CryptoPhase::Ready)?;
        
        Ok(StorageResponse {
            stored: true,
            kfrag_id: kfrag_data.id,
        })
    }
    
    pub fn handle_reencrypt(&mut self, reencrypt_data: ReencryptData) -> Result<CFragResponse, ProcessError> {
        // アクセス権限チェック
        if !self.has_permission(&reencrypt_data.requester) {
            return Err(ProcessError::AccessDenied);
        }
        
        // kFragの取得
        let kfrag = self.get_kfrag(&reencrypt_data.kfrag_id)
            .ok_or_else(|| ProcessError::KFragNotFound(reencrypt_data.kfrag_id.clone()))?;
        
        // 再暗号化実行
        self.state.transition_to(CryptoPhase::Reencrypting)?;
        
        let cfrag = perform_reencryption(&reencrypt_data.capsule, &kfrag)
            .map_err(|e| ProcessError::CryptoOperationFailed(e.to_string()))?;
        
        // カウンタ更新
        if let Some(ref mut holder_state) = self.state.holder_state {
            holder_state.reencryption_count += 1;
        }
        
        self.state.transition_to(CryptoPhase::Ready)?;
        
        Ok(CFragResponse {
            cfrag,
            holder_id: self.state.holder_state.as_ref().unwrap().holder_id,
        })
    }
}
```

## 6. 通信パターン

### 6.1 プロセス間メッセージング
```rust
// メッセージ送信の抽象化
pub trait AOMessaging {
    fn send_message(&self, target: &str, message: AOMessage) -> Result<(), MessageError>;
    fn broadcast_message(&self, message: AOMessage) -> Result<(), MessageError>;
}

// AOネットワーク固有の実装
pub struct AONetworkMessaging;

impl AOMessaging for AONetworkMessaging {
    fn send_message(&self, target: &str, message: AOMessage) -> Result<(), MessageError> {
        // AO固有のメッセージ送信ロジック
        ao_send(&target, &message)?;
        Ok(())
    }
    
    fn broadcast_message(&self, message: AOMessage) -> Result<(), MessageError> {
        // ブロードキャスト実装
        ao_broadcast(&message)?;
        Ok(())
    }
}
```

### 6.2 イベント処理
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProcessEvent {
    MessageReceived(AOMessage),
    StateChanged { from: CryptoPhase, to: CryptoPhase },
    ErrorOccurred(ProcessError),
    OperationCompleted(String),
}

pub trait EventHandler {
    fn handle_event(&mut self, event: ProcessEvent) -> Result<(), ProcessError>;
}

impl EventHandler for OwnerHandler {
    fn handle_event(&mut self, event: ProcessEvent) -> Result<(), ProcessError> {
        match event {
            ProcessEvent::MessageReceived(msg) => {
                self.handle_message(msg)
            },
            ProcessEvent::StateChanged { from, to } => {
                self.on_state_changed(from, to)
            },
            ProcessEvent::ErrorOccurred(error) => {
                self.on_error_occurred(error)
            },
            ProcessEvent::OperationCompleted(op) => {
                self.on_operation_completed(op)
            },
        }
    }
}
```

## 7. ロギングとモニタリング

### 7.1 構造化ログ
```rust
#[derive(Serialize, Debug)]
pub struct ProcessLog {
    pub timestamp: u64,
    pub process_id: String,
    pub level: LogLevel,
    pub message: String,
    pub context: serde_json::Value,
}

#[derive(Serialize, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

// ログマクロ
macro_rules! log_info {
    ($process_id:expr, $msg:expr, $($key:expr => $value:expr),*) => {
        {
            let mut context = serde_json::Map::new();
            $(
                context.insert($key.to_string(), serde_json::to_value($value).unwrap());
            )*
            
            let log = ProcessLog {
                timestamp: current_timestamp(),
                process_id: $process_id.to_string(),
                level: LogLevel::Info,
                message: $msg.to_string(),
                context: serde_json::Value::Object(context),
            };
            
            output_log(&log);
        }
    };
}

// 使用例
pub fn handle_message_with_logging(&mut self, msg: AOMessage) -> Result<AOResponse, ProcessError> {
    log_info!(
        self.state.process_id,
        "Processing message",
        "action" => &msg.action,
        "message_id" => &msg.id
    );
    
    let result = self.handle_message(msg);
    
    match &result {
        Ok(_) => log_info!(self.state.process_id, "Message processed successfully"),
        Err(e) => log_error!(self.state.process_id, "Message processing failed", "error" => e),
    }
    
    result
}
```

## 8. テスト

### 8.1 単体テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_owner_init() {
        let mut handler = OwnerHandler::new();
        let result = handler.handle_init(3, 5);
        
        assert!(result.is_ok());
        assert_eq!(handler.state.phase, CryptoPhase::Ready);
    }
    
    #[test]
    fn test_invalid_threshold() {
        let mut handler = OwnerHandler::new();
        let result = handler.handle_init(6, 5);  // threshold > total_shares
        
        assert!(matches!(result, Err(ProcessError::InvalidThreshold)));
    }
    
    #[test]
    fn test_state_transition() {
        let mut state = ProcessState::new(ProcessRole::Owner);
        
        assert!(state.transition_to(CryptoPhase::KeyGeneration).is_ok());
        assert!(state.transition_to(CryptoPhase::Completed).is_err()); // Invalid transition
    }
}
```

### 8.2 統合テスト
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_full_workflow() {
        // Owner-Process initialization
        let mut owner = OwnerHandler::new();
        let init_result = owner.handle_init(3, 5).unwrap();
        
        // Secret splitting
        let split_data = SplitData {
            secret: vec![1, 2, 3, 4],
            threshold: 3,
            total_shares: 5,
        };
        let split_result = owner.handle_split(split_data).unwrap();
        
        assert_eq!(split_result.shares_count, 5);
        assert!(split_result.distribution_complete);
    }
}
```

これらの規約に従うことで、AOネットワーク上で安定して動作するD-TPRESプロセスを実装できます。