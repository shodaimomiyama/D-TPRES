---
title: "D-TPRES Domain層アーキテクチャ概要"
description: "Entity/Repository interface/Infrastructure層の責務分離と設計指針"
tags: ["domain-driven-design", "clean-architecture", "ao-native", "prd-compliant"]
status: "specification"
created: "2025-06-25"
author: "D-TPRES Development Team"
---

# D-TPRES Domain層アーキテクチャ概要

## 1. はじめに

本ドキュメントは、D-TPRES（Deterministic Threshold Proxy Re-Encryption System）のdomain層アーキテクチャ全体像を説明します。Entity/Repository interface/Infrastructure層の明確な責務分離により、保守性と拡張性を両立した設計を実現しています。

## 2. アーキテクチャ概要

### 2.1 層構造と責務分離

```mermaid
graph TB
    subgraph "Domain Layer"
        subgraph "Entity"
            E1[ProcessEntity]
            E2[ShareEntity]
            E3[CapsuleEntity]
            E4[AccessRequestEntity]
            E5[RekeyFragmentEntity]
            E6[ReencryptionEntity]
        end
        
        subgraph "Repository Interface"
            R1[ProcessEntityRepository]
            R2[ShareEntityRepository]
            R3[CapsuleEntityRepository]
            R4[AccessRequestEntityRepository]
            R5[RekeyFragmentEntityRepository]
            R6[ReencryptionEntityRepository]
        end
    end
    
    subgraph "Infrastructure Layer"
        subgraph "Repository Implementation"
            I1[ArweaveRepositoryImpl]
            I2[ProcessEntityRepositoryImpl]
            I3[ShareEntityRepositoryImpl]
            I4[CapsuleEntityRepositoryImpl]
            I5[AccessRequestEntityRepositoryImpl]
            I6[RekeyFragmentEntityRepositoryImpl]
            I7[ReencryptionEntityRepositoryImpl]
        end
    end
    
    E1 --> R1
    E2 --> R2
    E3 --> R3
    E4 --> R4
    E5 --> R5
    E6 --> R6
    
    R1 --> I2
    R2 --> I3
    R3 --> I4
    R4 --> I5
    R5 --> I6
    R6 --> I7
    
    I2 --> I1
    I3 --> I1
    I4 --> I1
    I5 --> I1
    I6 --> I1
    I7 --> I1
```

### 2.2 各層の責務

#### Entity層
- **責務**: ビジネスデータの純粋な保持
- **特徴**: 
  - メソッドを持たない純粋なデータ構造
  - ビジネスルールはServiceレイヤーで実装
  - シリアライズ/デシリアライズ可能
  - 不変性を重視した設計

#### Repository Interface層
- **責務**: データアクセス操作の抽象定義
- **特徴**:
  - CRUD操作に特化
  - ドメイン特化のクエリメソッド定義
  - 永続化詳細から独立
  - テスタビリティの確保

#### Infrastructure層（Repository Implementation）
- **責務**: 具体的な永続化実装
- **特徴**:
  - Arweaveストレージへの永続化
  - タグベースインデックス管理
  - 不変ストレージへの対応
  - エラーハンドリングとリトライ

<<<<<<< HEAD
### 2.3 AOステートレス実行モデルへの対応

AOプロセスは以下の特性を持つ実行環境で動作します：

#### 実行環境の特性
- **ステートレス実行**: 各メッセージは異なるCU（Compute Unit）で処理される可能性があり、メモリ状態は保持されない
- **メッセージドリブン**: すべての処理はメッセージハンドラーとして実装され、メッセージ受信がトリガーとなる
- **状態の非永続性**: メモリ上のEntityインスタンスは次回メッセージ処理時には存在しない

> 📘 **詳細な技術仕様**: AOの実行モデルの詳細については[AO Process Model and Stateless Execution](../ao/ao_process_model.md)を参照してください。

#### 設計上の対応

1. **ProcessEntityの管理**
   - 各メッセージ処理の開始時にProcessEntityインスタンスを生成し、Arweaveから状態データを読み込む
   - プロセスのグローバル状態として扱い、メッセージ処理中は参照可能
   - 変更は即座にArweaveに永続化

2. **他Entityのライフサイクル**
   - 必要に応じてEntityインスタンスを生成し、オンデマンドで状態データをロード（Lazy Loading）
   - メッセージコンテキストから必要なEntity IDを抽出
   - 処理完了後は明示的な永続化

3. **状態管理戦略**
   ```rust
   // メッセージハンドラーでの典型的なパターン
   async fn handle_message(msg: Message) -> Result<Response> {
       // 1. ProcessEntityインスタンスを生成し、状態データを読み込む
       let process = repository.find_process_by_id(ao.id).await?;
       
       // 2. メッセージから必要なEntityのインスタンスを生成し、データをロード
       let entity = repository.find_by_id(msg.entity_id).await?;
       
       // 3. ビジネスロジック実行
       let updated_entity = business_logic(process, entity)?;
       
       // 4. 変更を即座にArweaveに永続化
       repository.update(updated_entity).await?;
       
       // 5. レスポンス返却
       Ok(response)
   }
   ```

### 2.4 複数秘密管理とEntityロード戦略

AOのステートレス環境で複数の秘密を効率的に管理するための設計戦略：

#### AOの実行モデルの正確な理解

1. **WASMモジュール**: 
   - Rustプログラム全体が単一のWASMファイルにコンパイルされる
   - ArweaveにデプロイされたWASMのtx_idをプロセスが参照
   
2. **実行時の動作**:
   - メッセージごとにCUが割り当てられる
   - CUがWASMモジュールをロードして実行
   - Entity/Repositoryのインスタンスはコードから生成される
   - 必要な状態データはRepository経由でArweaveから読み込む
   
3. **メモリライフサイクル**:
   - インスタンス: メッセージ処理ごとに生成・破棄
   - 状態データ: Arweaveに永続化
   - コード: WASMとしてArweaveに保存

#### メッセージコンテキストによるEntity特定

1. **メッセージタグによる秘密の識別**
   ```rust
   // 全てのメッセージに秘密IDを含める
   pub struct MessageContext {
       pub action: String,          // "Split-Secret", "Access-Request" 等
       pub secret_id: Option<String>,  // 対象となる秘密のID
       pub entity_ids: Vec<String>,    // 関連するEntity ID群
   }
   ```

2. **軽量な秘密インデックス**
   ```rust
   // ProcessEntityに軽量なインデックスを保持
   pub struct SecretIndex {
       pub secret_id: String,
       pub status: SecretStatus,
       pub entity_references: EntityReferences,
       pub last_updated: u64,
   }
   
   pub struct EntityReferences {
       pub share_ids: Vec<String>,      // ShareEntity IDs
       pub capsule_ids: Vec<String>,    // CapsuleEntity IDs
       pub active_requests: Vec<String>, // AccessRequestEntity IDs
   }
   ```

3. **遅延ロード（Lazy Loading）パターン**
   ```rust
   async fn load_entities_for_message(
       ctx: &HandlerContext,
       msg: &Message,
   ) -> Result<EntityBundle> {
       let secret_id = msg.tags.get("Secret-Id")?;
       let action = msg.tags.get("Action")?;
       
       match action.as_str() {
           "Access-Request" => {
               // ShareとCapsuleの概要情報のみ必要
               let index = ctx.process.secret_indices.get(secret_id)?;
               Ok(EntityBundle::minimal(index))
           },
           "Re-Encrypt" => {
               // 実際のShareとCapsuleデータが必要
               let shares = load_shares(secret_id).await?;
               let capsules = load_capsules(secret_id).await?;
               Ok(EntityBundle::full(shares, capsules))
           },
           _ => Ok(EntityBundle::empty())
       }
   }
   ```

#### Entity管理の最適化フロー

```mermaid
flowchart LR
    M[Message] --> MC[Message Context Extraction]
    MC --> SI[Secret Index Lookup]
    SI --> EL[Entity Loader]
    EL --> |Minimal Load| ML[Metadata Only]
    EL --> |Full Load| FL[Complete Entities]
    ML --> BL[Business Logic]
    FL --> BL
    BL --> P[Persist Changes]
```

=======
>>>>>>> origin/development
## 3. PRD準拠のワークフロー

### 3.1 Phase 0-5 概要

```mermaid
sequenceDiagram
    participant Browser as O/A-Browser
    participant PO as Owner-Process
    participant H as Holder-Process
    participant R as Requester-Process
    participant AR as Arweave
    participant EVM as EVM Chain
    
    Note over Browser,EVM: Phase 0: プロセス生成
    Browser->>PO: spawn process
    PO->>AR: ProcessEntity保存
    
    Note over Browser,EVM: Phase 1: 秘密分割
    Browser->>PO: encrypt secret
    PO->>AR: ShareEntity & CapsuleEntity保存
    
    Note over Browser,EVM: Phase 2: アクセス要求
    Browser->>R: request access
    R->>EVM: verify access
    R->>AR: AccessRequestEntity保存
    
    Note over Browser,EVM: Phase 3: kFrag配布
    PO->>H: distribute kFrag
    H->>AR: RekeyFragmentEntity保存
    
    Note over Browser,EVM: Phase 4: 再暗号化
    R->>H: request re-encryption
    H->>R: send cFrag
    R->>AR: ReencryptionEntity保存
    
    Note over Browser,EVM: Phase 5: 復号
    Browser->>Browser: decrypt with cFrags
```

### 3.2 Phase別Entity利用

| Phase | 主要Entity | 目的 |
|-------|-----------|------|
| Phase 0 | ProcessEntity | プロセス生成とロール設定 |
| Phase 1 | ShareEntity, CapsuleEntity | 秘密分割とカプセル生成 |
| Phase 2 | AccessRequestEntity | アクセス要求とEVM検証 |
| Phase 3 | RekeyFragmentEntity | 再暗号化キー分割と配布 |
| Phase 4 | ReencryptionEntity | プロキシ再暗号化実行 |
| Phase 5 | 全Entity | 復号と秘密復元 |

## 4. AO Native設計の特徴

### 4.1 AO環境との統合

```rust
// AO環境のネイティブプロパティ
pub struct AOContext {
    pub process_id: String,      // ao.id
    pub environment: String,     // ao.env
    pub module_id: String,       // ao._module
    pub spawn_data: String,      // ao._spawn
}
```

### 4.2 メッセージベース通信

AOプロセス間の通信はメッセージパッシングで実現：
- 非同期メッセージング
- イベントドリブンアーキテクチャ
- 状態の一貫性保証

<<<<<<< HEAD
### 4.3 メッセージ処理ライフサイクル

AOのステートレス環境でのメッセージ処理とEntity/Repository操作の統合：

```mermaid
sequenceDiagram
    participant M as Message
    participant H as Handler
    participant R as Repository
    participant A as Arweave
    participant E as Entity
    
    Note over M,E: メッセージ受信フェーズ
    M->>H: receive message
    H->>H: extract context (entity_id, action)
    
    Note over M,E: Entityインスタンス生成と状態復元フェーズ
    H->>R: create ProcessEntityRepository instance
    H->>R: load ProcessEntity(ao.id)
    R->>A: fetch state data from storage
    A-->>R: serialized state data
    R->>E: create instance & deserialize data
    R-->>H: ProcessEntity instance with state
    
    Note over M,E: ビジネスロジック実行フェーズ
    H->>R: load related entities
    R->>A: fetch by IDs
    A-->>R: entity data
    R-->>H: Entity instances
    H->>H: execute business logic
    
    Note over M,E: 永続化フェーズ
    H->>R: save all changes
    R->>E: serialize entities
    R->>A: persist to storage
    A-->>R: tx confirmation
    
    Note over M,E: レスポンスフェーズ
    H-->>M: send response
    Note over H: Memory cleared (next message may run on different CU)
```

#### 重要な考慮事項

1. **トランザクション境界**
   - 1メッセージ処理 = 1トランザクション
   - 部分的な永続化は避け、全ての変更を一括で保存

2. **エラーハンドリング**
   - Entity復元失敗時の適切なエラーレスポンス
   - 永続化失敗時のロールバック戦略

3. **パフォーマンス最適化**
   - 必要最小限のEntityのみロード
   - バッチ読み込み・書き込みの活用

=======
>>>>>>> origin/development
## 5. 普遍的プロセス設計

### 5.1 マルチロール対応

各プロセスは以下の3つのロールを同時に保持可能：

```rust
pub struct ProcessEntity {
    pub active_roles: Vec<String>, // ["owner", "holder", "requester"]
    pub owner_data: Option<OwnerData>,
    pub holder_data: Option<HolderData>,
    pub requester_data: Option<RequesterData>,
}
```

### 5.2 ロール別責務

#### Owner Role
- 秘密鍵（skO）の管理
- Shamir Secret Sharingによる分割
- 再暗号化キー（ReKey）の生成
- アクセス制御条件の設定

#### Holder Role  
- kFragの保持と管理
- プロキシ再暗号化の実行
- cFragの生成と提供
- 信頼性スコアの維持

#### Requester Role
- アクセス要求の発行
- EVM検証の調整
- cFragの収集
- 閾値達成の管理

## 6. データ永続化戦略

### 6.1 Arweave不変ストレージ

Arweaveの特性を活かした永続化：
- **不変性**: 一度保存したデータは変更不可
- **永続性**: データの永続保存保証
- **分散性**: 分散ネットワークによる可用性
- **検証可能性**: データの完全性検証

### 6.2 バージョニング戦略

```rust
// 更新は新しいトランザクションとして保存
pub struct VersionedEntity<T> {
    pub entity: T,
    pub version: u64,
    pub previous_tx_id: Option<String>,
    pub created_at: u64,
}
```

<<<<<<< HEAD
### 6.3 ステートレス環境での永続化パターン

#### ProcessEntityの特別な扱い

ProcessEntityはプロセスの「グローバル状態」として特別に管理：

```rust
// メッセージハンドラーの開始時に必ず実行
async fn initialize_handler_context() -> Result<HandlerContext> {
    // Repositoryインスタンスを生成
    let process_repo = ProcessEntityRepository::new();
    
    // ProcessEntityインスタンスを生成し、Arweaveから状態データを読み込む
    let process = process_repo.find_by_id(&ao.id)
        .await?
        .ok_or(Error::ProcessNotFound)?;
    
    Ok(HandlerContext {
        process,
        repositories: RepositoryContainer::new(),
    })
}
```

**重要な区別**:
- **コード（WASM）**: Arweaveに保存され、tx_idで参照される実行可能なバイナリ
- **インスタンス**: CU上でWASMコードから毎回生成されるオブジェクト
- **状態データ**: Arweaveに永続化され、Repository経由で読み書きされる実際のデータ

#### オンデマンドEntity管理

他のEntityは必要に応じてインスタンスを生成し、状態データをロード：

```rust
// 例: ShareEntityの遅延ロード
async fn handle_access_request(ctx: &HandlerContext, msg: Message) -> Result<()> {
    // メッセージから必要な情報を抽出
    let secret_id = msg.tags.get("Secret-Id")?;
    
    // ShareEntityインスタンスを生成し、必要な状態データのみロード
    let shares = ctx.repositories.share_repo
        .find_by_secret_id(secret_id)
        .await?;
    
    // ビジネスロジック実行
    // ...
    
    // 変更があれば即座に永続化
    for share in modified_shares {
        ctx.repositories.share_repo.update(&share).await?;
    }
    
    Ok(())
}
```

#### 効率的なEntity管理パターン

1. **Entity Bundle パターン**
   ```rust
   // 関連Entityをまとめて管理
   pub struct EntityBundle {
       pub shares: Option<Vec<ShareEntity>>,
       pub capsules: Option<Vec<CapsuleEntity>>,
       pub requests: Option<Vec<AccessRequestEntity>>,
       pub loaded_at: u64,
   }
   
   impl EntityBundle {
       pub fn minimal(index: &SecretIndex) -> Self {
           // メタデータのみを含む最小限のBundle
           Self {
               shares: None,
               capsules: None,
               requests: None,
               loaded_at: current_timestamp(),
           }
       }
       
       pub fn full(shares: Vec<ShareEntity>, capsules: Vec<CapsuleEntity>) -> Self {
           // 完全なEntityデータを含むBundle
           Self {
               shares: Some(shares),
               capsules: Some(capsules),
               requests: None,
               loaded_at: current_timestamp(),
           }
       }
   }
   ```

2. **メッセージスコープキャッシュ**
   ```rust
   pub struct MessageScopeCache {
       entities: HashMap<String, Box<dyn Any>>,
       
       pub fn get_or_load<T: Entity>(
           &mut self,
           id: &str,
           loader: impl Fn(&str) -> Result<T>
       ) -> Result<&T> {
           // メッセージ処理中のみ有効なキャッシュ
           if !self.entities.contains_key(id) {
               let entity = loader(id)?;
               self.entities.insert(id.to_string(), Box::new(entity));
           }
           Ok(self.entities.get(id).unwrap().downcast_ref().unwrap())
       }
   }
   ```

3. **バッチロード最適化**
   ```rust
   // 関連Entityを一括ロード
   async fn batch_load_for_secret(
       repos: &RepositoryContainer,
       secret_id: &str,
   ) -> Result<SecretEntities> {
       let (shares, capsules) = tokio::join!(
           repos.share_repo.find_by_secret_id(secret_id),
           repos.capsule_repo.find_by_secret_id(secret_id),
       );
       
       Ok(SecretEntities {
           shares: shares?,
           capsules: capsules?,
       })
   }
   ```

#### 永続化の原則

1. **Write-Through**: キャッシュを介さず直接Arweaveに書き込み
2. **Immediate Persistence**: 状態変更は即座に永続化
3. **Atomic Operations**: 関連する変更は一括で永続化
4. **Selective Loading**: 必要なEntityのみをロード
5. **Batch Operations**: 可能な限りバッチ処理で効率化

=======
>>>>>>> origin/development
## 7. 設計原則

### 7.1 SOLID原則の適用

1. **単一責任の原則（SRP）**
   - Entity: データ保持のみ
   - Repository: CRUD操作のみ
   - Implementation: 永続化詳細のみ

2. **開放閉鎖の原則（OCP）**
   - 新しいEntityの追加が容易
   - 既存コードの変更不要

3. **リスコフの置換原則（LSP）**
   - Repository実装は交換可能
   - テスト実装への置換が容易

4. **インターフェース分離の原則（ISP）**
   - 各Entityごとに専用Repository
   - 必要最小限のメソッド定義

5. **依存性逆転の原則（DIP）**
   - 上位層は抽象に依存
   - 実装詳細は下位層に隔離

### 7.2 Clean Architecture準拠

```
┌─────────────────────────────────────┐
│          Application Layer          │
│         (Service, UseCase)          │
├─────────────────────────────────────┤
│           Domain Layer              │
│    (Entity, Repository Interface)   │
├─────────────────────────────────────┤
│        Infrastructure Layer         │
│    (Repository Implementation)      │
└─────────────────────────────────────┘
```

## 8. 実装ガイドライン

### 8.1 Entity実装規約

```rust
// ✅ 正しい実装
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SomeEntity {
    pub id: String,
    pub data: String,
    pub created_at: u64,
}

// ❌ 間違った実装（メソッドを含む）
impl SomeEntity {
    pub fn validate(&self) -> bool { // NG: Entityにメソッド
        // validation logic
    }
}
```

### 8.2 Repository実装規約

```rust
// Repository Interface
#[async_trait]
pub trait SomeEntityRepository: Repository<SomeEntity, String> {
    // CRUD操作とドメイン特化クエリのみ
    async fn find_by_condition(&self, condition: &str) 
        -> Result<Vec<SomeEntity>, Self::Error>;
}

// ❌ 間違った実装（ビジネスロジックを含む）
#[async_trait]
pub trait SomeEntityRepository {
    async fn validate_and_save(&self, entity: &SomeEntity) 
        -> Result<(), Self::Error>; // NG: ビジネスロジック
}
```

<<<<<<< HEAD
### 8.3 AOメッセージハンドラーでの利用パターン

#### 基本的なハンドラー構造

```rust
use ao_sdk::{Message, Response, ao};

// メッセージハンドラーの登録
pub fn register_handlers() {
    // Phase 1: 秘密分割ハンドラー
    Handlers::add("split-secret", 
        Handlers::utils::hasMatchingTag("Action", "Split-Secret"),
        handle_split_secret
    );
    
    // Phase 2: アクセス要求ハンドラー
    Handlers::add("access-request",
        Handlers::utils::hasMatchingTag("Action", "Access-Request"),
        handle_access_request
    );
}

// ハンドラー実装例
async fn handle_split_secret(msg: Message) -> Response {
    // 1. コンテキスト初期化（ProcessEntity復元）
    let ctx = match initialize_context().await {
        Ok(ctx) => ctx,
        Err(e) => return error_response(e),
    };
    
    // 2. 入力検証
    let secret_data = match extract_secret_data(&msg) {
        Ok(data) => data,
        Err(e) => return error_response(e),
    };
    
    // 3. ビジネスロジック実行
    let result = match split_secret_service(&ctx, secret_data).await {
        Ok(result) => result,
        Err(e) => return error_response(e),
    };
    
    // 4. 結果の永続化（Service内で実行済み）
    
    // 5. レスポンス生成
    success_response(result)
}
```

#### スケーラブルなハンドラー実装

```rust
// 秘密の数に依存しないO(1)のEntity管理
async fn handle_message_with_secret_context(msg: Message) -> Response {
    // 1. 最小限のコンテキスト初期化
    let process = load_process_entity_minimal().await?;
    
    // 2. メッセージから秘密IDを特定
    let secret_id = extract_secret_id(&msg)?;
    
    // 3. 秘密固有のインデックスをロード
    let secret_index = process.get_secret_index(secret_id)?;
    
    // 4. 必要なEntityのみロード
    let entities = load_required_entities(&secret_index, &msg.action).await?;
    
    // 5. ビジネスロジック実行
    let result = execute_business_logic(entities, msg)?;
    
    // 6. 変更の永続化
    persist_changes(result).await?;
    
    Ok(Response::success())
}

// Entity選択的ロード戦略
async fn load_required_entities(
    index: &SecretIndex,
    action: &str,
) -> Result<EntityBundle> {
    match action {
        "Split-Secret" => {
            // 新規作成のため既存Entityは不要
            Ok(EntityBundle::empty())
        },
        "Access-Request" => {
            // 秘密の存在確認のみ必要
            Ok(EntityBundle::minimal(index))
        },
        "Distribute-KFrag" => {
            // AccessRequestとShareの情報が必要
            let request = load_access_request(index.active_requests.last()).await?;
            Ok(EntityBundle::with_request(request))
        },
        "Re-Encrypt" => {
            // 完全なShareとCapsuleデータが必要
            let (shares, capsules) = batch_load_for_reencryption(index).await?;
            Ok(EntityBundle::full(shares, capsules))
        },
        _ => Ok(EntityBundle::empty())
    }
}
```

#### Repository利用のベストプラクティス

1. **Repository生成はハンドラー開始時に一度だけ**
   ```rust
   let repo_container = RepositoryContainer::new();
   ```

2. **必要なEntityのみロード**
   ```rust
   // ❌ 非効率
   let all_shares = share_repo.find_all().await?;
   
   // ✅ 効率的
   let shares = share_repo.find_by_secret_id(secret_id).await?;
   ```

3. **トランザクション的な更新**
   ```rust
   // 関連するEntityをまとめて更新
   let mut updates = Vec::new();
   updates.push(share_repo.update(&share));
   updates.push(capsule_repo.update(&capsule));
   
   // 全て成功するか、全て失敗する
   futures::try_join_all(updates).await?;
   ```

4. **メッセージコンテキストの活用**
   ```rust
   // メッセージタグから必要な情報を効率的に抽出
   let ctx = MessageContext::from_tags(&msg.tags)?;
   let entities = ctx.determine_required_entities();
   ```

=======
>>>>>>> origin/development
## 9. まとめ

D-TPRES domain層は以下の特徴を持つ設計となっています：

1. **明確な責務分離**: Entity/Repository Interface/Implementationの3層構造
2. **PRD準拠**: Phase 0-5のワークフローを正確に実装
3. **AO Native**: AOプロセスの特性を活かした設計
4. **普遍的設計**: 各プロセスがマルチロール対応
5. **Arweave最適化**: 不変ストレージの特性を活用
<<<<<<< HEAD
6. **ステートレス対応**: AOの実行モデルに適合した状態管理

この設計により、保守性、拡張性、テスタビリティを兼ね備えた堅牢なシステムを実現します。

### 重要な設計指針

- **Entity**: データ保持に特化し、ビジネスロジックを含まない
- **Repository**: CRUD操作とクエリに特化し、永続化の詳細を隠蔽
- **Handler**: AOメッセージを受信し、Repository経由でEntityを操作
- **Persistence**: すべての状態変更は即座にArweaveに永続化

### AOの実行モデルに関する重要な理解

- **インスタンスとデータの区別**: Entityインスタンスは毎回生成され、状態データはArweaveから読み込まれる
- **WASMモジュール**: 全プロセスが同一のWASMコードを実行し、tx_idで参照
- **ステートレス実行**: メッセージ処理間でメモリは保持されず、CUも異なる可能性がある

これらの指針に従うことで、AOのステートレス実行環境でも一貫性のある状態管理が可能となります。

---

**関連ドキュメント**: 
- [AOプロセスモデルとステートレス実行](../ao/ao_process_model.md) - AOの実行モデルの詳細な説明

---

**Document Status**: Domain Architecture Specification  
**Version**: 1.0  
**Next Steps**: Entity詳細設計の実装（entities.md参照）