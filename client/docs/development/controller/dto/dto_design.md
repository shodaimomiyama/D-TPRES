# DTO（Data Transfer Object）詳細設計

## 1. 概要

DTOは、Controller層とService層の間でデータを受け渡すための構造化オブジェクトです。D-TPRESシステムでは、AOメッセージから抽出された情報を、Service層が処理しやすい形式に整理して提供します。

### 1.1 DTOの役割

1. **データの構造化**
   - フラットなメッセージタグを階層的なオブジェクトに変換
   - 関連データのグループ化
   - 型安全性の提供

2. **層間の独立性**
   - AOメッセージ形式からService層を分離
   - 内部表現の変更を局所化
   - テスタビリティの向上

3. **ビジネスロジックの準備**
   - 必要なデータの事前検証
   - デフォルト値の設定
   - 計算値の事前準備

### 1.2 設計原則

- **不変性（Immutability）**: 一度生成されたDTOは変更不可
- **完全性（Completeness）**: 必要な情報をすべて含む
- **単純性（Simplicity）**: ロジックを含まない純粋なデータ構造
- **検証可能性（Validatable）**: 自己検証機能を持つ

## 2. DTO階層構造

### 2.1 基底インターフェース

```rust
/// すべてのDTOが実装すべき基本インターフェース
pub trait DTO: Send + Sync + Debug + Clone {
    /// DTO種別の識別子
    fn dto_type(&self) -> &'static str;
    
    /// 内部整合性の検証
    fn validate(&self) -> Result<(), ValidationError>;
    
    /// シリアライズ可能な形式への変換
    fn to_value(&self) -> serde_json::Value;
}

/// リクエストDTOの基底
pub trait RequestDTO: DTO {
    /// リクエストID（冪等性のため）
    fn request_id(&self) -> &str;
    
    /// タイムアウト設定
    fn timeout(&self) -> Option<Duration>;
}

/// レスポンスDTOの基底
pub trait ResponseDTO: DTO {
    /// 処理成功フラグ
    fn is_success(&self) -> bool;
    
    /// エラー情報（失敗時）
    fn error(&self) -> Option<&ServiceError>;
}
```

### 2.2 共通データ構造

```rust
/// プロセス識別情報
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessIdentity {
    /// プロセスID
    pub process_id: ProcessId,
    
    /// プロセスロール
    pub role: ProcessRole,
    
    /// 公開鍵（オプション）
    pub public_key: Option<PublicKey>,
}

/// タイミング情報
#[derive(Debug, Clone)]
pub struct TimingInfo {
    /// 作成時刻
    pub created_at: SystemTime,
    
    /// 有効期限
    pub expires_at: Option<SystemTime>,
    
    /// 処理期限
    pub deadline: Option<SystemTime>,
}

/// 暗号化パラメータ
#[derive(Debug, Clone)]
pub struct CryptoParams {
    /// 暗号化アルゴリズム
    pub algorithm: CryptoAlgorithm,
    
    /// 鍵長
    pub key_size: usize,
    
    /// 追加パラメータ
    pub additional_params: HashMap<String, String>,
}
```

## 3. Phase別DTO定義

### 3.1 Phase 1: 秘密分割

#### リクエストDTO

```rust
/// 秘密分割リクエスト
#[derive(Debug, Clone)]
pub struct SplitSecretRequest {
    /// リクエストID
    pub request_id: String,
    
    /// 秘密の識別子
    pub secret_id: String,
    
    /// 秘密データ
    pub secret_data: SecretData,
    
    /// 分割パラメータ
    pub split_params: SplitParameters,
    
    /// アクセス制御
    pub access_control: AccessControl,
    
    /// メタデータ
    pub metadata: SecretMetadata,
}

#[derive(Debug, Clone)]
pub struct SecretData {
    /// 暗号化されたデータ
    pub encrypted_data: Vec<u8>,
    
    /// データ形式
    pub content_type: String,
    
    /// 元のサイズ
    pub original_size: usize,
    
    /// チェックサム
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct SplitParameters {
    /// 復元に必要な最小シェア数
    pub threshold: u8,
    
    /// 生成する総シェア数
    pub total_shares: u8,
    
    /// Shamir Secret Sharingのパラメータ
    pub shamir_params: ShamirParams,
}

#[derive(Debug, Clone)]
pub struct AccessControl {
    /// アクセス条件のタイプ
    pub condition_type: AccessConditionType,
    
    /// 条件の詳細
    pub conditions: Vec<AccessCondition>,
    
    /// 条件の組み合わせロジック
    pub logic: ConditionLogic,
}
```

#### レスポンスDTO

```rust
/// 秘密分割レスポンス
#[derive(Debug, Clone)]
pub struct SplitSecretResponse {
    /// 処理結果
    pub success: bool,
    
    /// エラー情報
    pub error: Option<ServiceError>,
    
    /// 結果データ
    pub result: Option<SplitSecretResult>,
}

#[derive(Debug, Clone)]
pub struct SplitSecretResult {
    /// 秘密ID
    pub secret_id: String,
    
    /// 生成されたシェア情報
    pub shares: Vec<ShareInfo>,
    
    /// カプセル情報
    pub capsule: CapsuleInfo,
    
    /// Arweaveトランザクション
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Debug, Clone)]
pub struct ShareInfo {
    /// シェアID
    pub share_id: String,
    
    /// シェアインデックス
    pub index: u8,
    
    /// 保存先
    pub storage_location: StorageLocation,
    
    /// ハッシュ
    pub hash: String,
}
```

### 3.2 Phase 2: アクセス要求

#### リクエストDTO

```rust
/// アクセス要求リクエスト
#[derive(Debug, Clone)]
pub struct AccessRequestDTO {
    /// リクエストID
    pub request_id: String,
    
    /// ターゲット秘密
    pub target_secret: TargetSecret,
    
    /// リクエスター情報
    pub requester: RequesterInfo,
    
    /// アクセス証明
    pub access_proof: AccessProofDTO,
    
    /// リクエストオプション
    pub options: AccessRequestOptions,
}

#[derive(Debug, Clone)]
pub struct TargetSecret {
    /// 秘密ID
    pub secret_id: String,
    
    /// 期待されるデータ型
    pub expected_type: Option<String>,
    
    /// 必要なシェア数
    pub required_shares: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct RequesterInfo {
    /// プロセスID
    pub process_id: ProcessId,
    
    /// Ethereumアドレス
    pub eth_address: EthereumAddress,
    
    /// 公開鍵
    pub public_key: PublicKey,
    
    /// 認証情報
    pub credentials: Option<Credentials>,
}

#[derive(Debug, Clone)]
pub struct AccessProofDTO {
    /// 証明タイプ
    pub proof_type: ProofType,
    
    /// 証明データ
    pub proof_data: ProofData,
    
    /// 検証結果
    pub verification: Option<VerificationResult>,
}
```

### 3.3 Phase 3: 再暗号化キー配布

#### リクエストDTO

```rust
/// kFrag配布リクエスト
#[derive(Debug, Clone)]
pub struct DistributeKFragRequest {
    /// リクエストID
    pub request_id: String,
    
    /// アクセス要求ID
    pub access_request_id: String,
    
    /// 再暗号化キー
    pub reencryption_key: ReencryptionKeyDTO,
    
    /// 配布戦略
    pub distribution: DistributionStrategy,
    
    /// タイミング制御
    pub timing: TimingControl,
}

#[derive(Debug, Clone)]
pub struct ReencryptionKeyDTO {
    /// キーID
    pub key_id: String,
    
    /// 暗号化された再暗号化キー
    pub encrypted_key: Vec<u8>,
    
    /// キーメタデータ
    pub metadata: KeyMetadata,
}

#[derive(Debug, Clone)]
pub struct DistributionStrategy {
    /// 配布方式
    pub method: DistributionMethod,
    
    /// ターゲットホルダー
    pub target_holders: Vec<HolderSelection>,
    
    /// 冗長性設定
    pub redundancy: RedundancyConfig,
    
    /// 優先順位
    pub priority: Priority,
}
```

### 3.4 Phase 4: プロキシ再暗号化

#### リクエストDTO

```rust
/// 再暗号化実行リクエスト
#[derive(Debug, Clone)]
pub struct ReencryptionRequest {
    /// リクエストID
    pub request_id: String,
    
    /// ターゲット
    pub target: ReencryptionTarget,
    
    /// 実行パラメータ
    pub params: ReencryptionParams,
    
    /// 収集戦略
    pub collection: CollectionStrategy,
}

#[derive(Debug, Clone)]
pub struct ReencryptionTarget {
    /// カプセルID
    pub capsule_id: String,
    
    /// 必要なcFrag数
    pub required_cfrags: u8,
    
    /// タイムアウト
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ReencryptionParams {
    /// 使用するkFragのID
    pub kfrag_ids: Vec<String>,
    
    /// 検証設定
    pub verification: VerificationConfig,
    
    /// パフォーマンス設定
    pub performance: PerformanceConfig,
}
```

### 3.5 Phase 5: 秘密復元

#### リクエストDTO

```rust
/// 秘密復元リクエスト
#[derive(Debug, Clone)]
pub struct RecoverSecretRequest {
    /// リクエストID
    pub request_id: String,
    
    /// 復元対象
    pub recovery_target: RecoveryTarget,
    
    /// 収集されたcFrag
    pub collected_cfrags: Vec<CFragDTO>,
    
    /// 復元オプション
    pub options: RecoveryOptions,
}

#[derive(Debug, Clone)]
pub struct RecoveryTarget {
    /// 秘密ID
    pub secret_id: String,
    
    /// カプセル
    pub capsule: CapsuleDTO,
    
    /// 期待されるハッシュ
    pub expected_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CFragDTO {
    /// cFrag ID
    pub cfrag_id: String,
    
    /// cFragデータ
    pub data: Vec<u8>,
    
    /// 提供元ホルダー
    pub holder_id: ProcessId,
    
    /// 検証情報
    pub verification: CFragVerification,
}
```

## 4. 検証ルール

### 4.1 基本検証

```rust
/// DTO検証トレイト
pub trait Validatable {
    fn validate(&self) -> ValidationResult;
}

/// 検証結果
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// 検証エラー
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
```

### 4.2 共通検証ルール

| 検証項目 | ルール | 適用対象 |
|---------|--------|---------|
| ID形式 | UUID v4形式 | すべてのID |
| タイムスタンプ | 未来時刻の禁止 | created_at |
| 有効期限 | created_at < expires_at | TimingInfo |
| 閾値 | 1 ≤ threshold ≤ total | SplitParameters |
| サイズ制限 | 各フィールドの最大サイズ | すべてのバイナリデータ |

### 4.3 カスタム検証

```rust
impl Validatable for SplitSecretRequest {
    fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::default();
        
        // 基本検証
        if self.secret_id.is_empty() {
            result.add_error(ValidationError {
                field: "secret_id".to_string(),
                code: "EMPTY_ID".to_string(),
                message: "Secret ID cannot be empty".to_string(),
                details: None,
            });
        }
        
        // 閾値検証
        if self.split_params.threshold > self.split_params.total_shares {
            result.add_error(ValidationError {
                field: "split_params".to_string(),
                code: "INVALID_THRESHOLD".to_string(),
                message: format!(
                    "Threshold {} exceeds total shares {}",
                    self.split_params.threshold,
                    self.split_params.total_shares
                ),
                details: None,
            });
        }
        
        // データサイズ検証
        if self.secret_data.encrypted_data.len() > MAX_SECRET_SIZE {
            result.add_error(ValidationError {
                field: "secret_data".to_string(),
                code: "DATA_TOO_LARGE".to_string(),
                message: format!(
                    "Secret data size {} exceeds maximum {}",
                    self.secret_data.encrypted_data.len(),
                    MAX_SECRET_SIZE
                ),
                details: None,
            });
        }
        
        result
    }
}
```

## 5. 変換パターン

### 5.1 Message → DTO変換

```rust
/// メッセージからDTOへの変換トレイト
pub trait FromMessage: Sized {
    fn from_message(msg: &Message) -> Result<Self, ConversionError>;
}

/// 実装例
impl FromMessage for SplitSecretRequest {
    fn from_message(msg: &Message) -> Result<Self, ConversionError> {
        Ok(SplitSecretRequest {
            request_id: extract_required_tag(msg, "Request-Id")?,
            secret_id: extract_required_tag(msg, "Secret-Id")?,
            secret_data: SecretData {
                encrypted_data: msg.data.clone(),
                content_type: extract_optional_tag(msg, "Content-Type")
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                original_size: extract_required_tag::<usize>(msg, "Original-Size")?,
                checksum: extract_required_tag(msg, "Checksum")?,
            },
            split_params: SplitParameters {
                threshold: extract_required_tag::<u8>(msg, "Threshold-K")?,
                total_shares: extract_required_tag::<u8>(msg, "Threshold-N")?,
                shamir_params: extract_shamir_params(msg)?,
            },
            access_control: extract_access_control(msg)?,
            metadata: extract_metadata(msg),
        })
    }
}
```

### 5.2 DTO → Response変換

```rust
/// DTOからレスポンスへの変換トレイト
pub trait ToResponse {
    fn to_response(&self) -> Response;
}

/// 実装例
impl ToResponse for SplitSecretResponse {
    fn to_response(&self) -> Response {
        let status = if self.success { "Success" } else { "Error" };
        
        let mut tags = vec![
            ("Status".to_string(), status.to_string()),
            ("Action".to_string(), "Split-Secret-Response".to_string()),
        ];
        
        if let Some(result) = &self.result {
            tags.push(("Secret-Id".to_string(), result.secret_id.clone()));
            tags.push(("Share-Count".to_string(), result.shares.len().to_string()));
        }
        
        if let Some(error) = &self.error {
            tags.push(("Error-Code".to_string(), error.code().to_string()));
            tags.push(("Error-Message".to_string(), error.to_string()));
        }
        
        Response {
            tags,
            data: serde_json::to_vec(&self.to_value()).unwrap_or_default(),
        }
    }
}
```

## 6. シリアライゼーション

### 6.1 JSON表現

```rust
/// JSON用のシリアライゼーション設定
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonDTO<T> {
    #[serde(rename = "@type")]
    pub dto_type: String,
    
    #[serde(rename = "@version")]
    pub version: String,
    
    #[serde(flatten)]
    pub data: T,
}

/// カスタムシリアライザー
pub mod custom_serializers {
    pub mod base64_bytes {
        pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&base64::encode(bytes))
        }
        
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            base64::decode(&s).map_err(de::Error::custom)
        }
    }
}
```

### 6.2 バイナリ表現

```rust
/// 効率的なバイナリシリアライゼーション
pub trait BinarySerializable {
    fn to_bytes(&self) -> Result<Vec<u8>, SerializationError>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> where Self: Sized;
}

/// MessagePack実装
impl<T: Serialize + DeserializeOwned> BinarySerializable for T {
    fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        rmp_serde::to_vec(self)
            .map_err(|e| SerializationError::MessagePack(e.to_string()))
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        rmp_serde::from_slice(bytes)
            .map_err(|e| SerializationError::MessagePack(e.to_string()))
    }
}
```

## 7. バージョニング

### 7.1 DTOバージョン管理

```rust
/// バージョン付きDTO
pub trait Versioned {
    /// 現在のバージョン
    fn version(&self) -> Version;
    
    /// 互換性チェック
    fn is_compatible_with(&self, other_version: &Version) -> bool;
    
    /// マイグレーション
    fn migrate_from(&mut self, old_version: &Version) -> Result<(), MigrationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}
```

### 7.2 後方互換性

```rust
/// 古いバージョンのサポート
pub enum SplitSecretRequestV1 {
    V1_0(SplitSecretRequestV1_0),
    V1_1(SplitSecretRequestV1_1),
    V2_0(SplitSecretRequestV2_0),
}

impl SplitSecretRequestV1 {
    /// 最新バージョンへの変換
    pub fn to_latest(self) -> SplitSecretRequest {
        match self {
            Self::V1_0(v1) => v1.migrate_to_v2(),
            Self::V1_1(v1) => v1.migrate_to_v2(),
            Self::V2_0(v2) => v2,
        }
    }
}
```

## 8. パフォーマンス考慮事項

### 8.1 メモリ効率

| 最適化手法 | 適用箇所 | 効果 |
|-----------|---------|------|
| Copy-on-Write | 大きなバイナリデータ | メモリ使用量削減 |
| Lazy Loading | オプショナルフィールド | 初期化時間短縮 |
| Interning | 繰り返し使用される文字列 | メモリ使用量削減 |
| Zero-Copy | バイナリデータの参照 | コピーコスト削減 |

### 8.2 処理効率

```rust
/// 効率的なDTO生成
pub struct DTOBuilder<T> {
    inner: T,
    validation_deferred: bool,
}

impl<T: Validatable> DTOBuilder<T> {
    /// 検証を遅延させて効率化
    pub fn defer_validation(mut self) -> Self {
        self.validation_deferred = true;
        self
    }
    
    /// 一括ビルド
    pub fn build_many(builders: Vec<Self>) -> Result<Vec<T>, ValidationError> {
        // 並列検証
        builders.into_par_iter()
            .map(|b| b.build())
            .collect()
    }
}
```

## 9. セキュリティ考慮事項

### 9.1 機密情報の扱い

```rust
/// 機密情報を含むフィールドのマーキング
#[derive(Debug, Clone)]
pub struct SensitiveData<T> {
    inner: T,
    #[debug(skip)]
    _phantom: PhantomData<T>,
}

impl<T> SensitiveData<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: value,
            _phantom: PhantomData,
        }
    }
    
    /// 明示的なアクセス
    pub fn reveal(&self) -> &T {
        &self.inner
    }
}

/// Debugトレイトでの秘匿
impl<T> fmt::Debug for SensitiveData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<REDACTED>")
    }
}
```

### 9.2 入力サニタイゼーション

```rust
/// 自動サニタイゼーション
pub trait Sanitizable {
    fn sanitize(&mut self);
}

impl Sanitizable for String {
    fn sanitize(&mut self) {
        // 制御文字の除去
        self.retain(|c| !c.is_control());
        
        // トリミング
        *self = self.trim().to_string();
        
        // 長さ制限
        if self.len() > MAX_STRING_LENGTH {
            self.truncate(MAX_STRING_LENGTH);
        }
    }
}
```

## 10. テスト支援

### 10.1 テストビルダー

```rust
/// テスト用のDTOビルダー
pub mod test_builders {
    pub struct TestDTOBuilder<T> {
        inner: T,
    }
    
    impl TestDTOBuilder<SplitSecretRequest> {
        pub fn valid_request() -> Self {
            Self {
                inner: SplitSecretRequest {
                    request_id: "test-123".to_string(),
                    secret_id: "secret-456".to_string(),
                    // ... デフォルト値
                }
            }
        }
        
        pub fn with_threshold(mut self, k: u8, n: u8) -> Self {
            self.inner.split_params.threshold = k;
            self.inner.split_params.total_shares = n;
            self
        }
        
        pub fn build(self) -> SplitSecretRequest {
            self.inner
        }
    }
}
```

### 10.2 アサーション

```rust
/// DTOアサーション
pub mod assertions {
    pub fn assert_valid_dto<T: Validatable>(dto: &T) {
        let result = dto.validate();
        assert!(
            result.is_valid,
            "DTO validation failed: {:?}",
            result.errors
        );
    }
    
    pub fn assert_dto_equals<T: PartialEq + Debug>(actual: &T, expected: &T) {
        assert_eq!(
            actual, expected,
            "DTOs are not equal.\nActual: {:#?}\nExpected: {:#?}",
            actual, expected
        );
    }
}
```

## 11. まとめ

DTOは、D-TPRESシステムにおけるデータ転送の中核を担う重要な要素です：

1. **明確な構造**: 型安全で理解しやすいデータ構造
2. **層間の分離**: Controller層とService層の独立性を保証
3. **検証機能**: データの整合性を保証する自己検証
4. **拡張性**: 新しいフィールドやバージョンへの対応
5. **テスタビリティ**: テストしやすい設計とヘルパー

これらの特性により、安全で保守性の高いデータ転送を実現し、システム全体の品質向上に貢献します。