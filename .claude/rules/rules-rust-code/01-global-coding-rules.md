# グローバルコーディングルール

基本ルールはプロジェクト全体に適用されるべきです。

## 0. WASM/AOターゲット制約

**理由:** FORMIXはAOネットワーク上でWebAssemblyとして動作し、ネイティブRustとは異なる特定の制約があります。

**ルール:**

### 0.1 WASMでの標準ライブラリなし

- **可能な限りno_stdを使用**: バイナリサイズを削減し、WASM互換性を確保
- **明示的なアロケーター**: より小さいフットプリントのために `wee_alloc` または類似のものを使用
- **フィーチャーフラグ**: stdとno_stdのコードパスを分離

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String};

#[cfg(feature = "std")]
use std::{vec::Vec, string::String};
```

### 0.2 シングルスレッド実行 (ao/contracts/ のみ)

**重要: このセクションの制約は `ao/contracts/src/` に適用されます。`client/` は async/await を使用できます（AOClient trait が既に async）。**

- **スレッドなし**: AO WASMはシングルスレッド
- **async/awaitなし**: すべての操作は同期的でなければならない
- **スレッドローカルストレージなし**: 明示的な状態渡しを使用

```rust
// === ao/contracts/ ===
// 悪い例: AOでは非同期は許可されない
async fn process_message(msg: Message) -> Result<Response> {
    // AOでは動作しません！
}

// 良い例: 同期処理
fn process_message(msg: Message) -> Result<Response> {
    // 同期操作のみ
}

// === client/ ===
// 悪い例: sync メソッド内で runtime を生成 → "runtime inside runtime" panic
fn send_data(&self) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread().build()?;
    rt.block_on(self.ao_client.execute(msg))?; // パニック！
    Ok(())
}

// 良い例: async を伝播
async fn send_data(&self) -> Result<()> {
    self.ao_client.execute(msg).await?;
    Ok(())
}
```

### 0.3 メモリ制約

- **ヒープ割り当ての最小化**: 可能な限りスタック割り当て配列を使用
- **境界付きコレクション**: 予測可能なメモリのために `heapless` または `arrayvec` を使用
- **サイズ制限**: Arweaveストレージのためにバイナリを2MB未満に保つ

```rust
use heapless::Vec as HeaplessVec;
use heapless::consts::*;

// 良い例: 境界付きコレクション
type BoundedShares = HeaplessVec<Share, U20>; // 最大20シェア

// 悪い例: 境界なしコレクション
type UnboundedShares = Vec<Share>; // 無限に成長する可能性
```

## 1. Rustバージョンとベストプラクティス

**理由:** Rustは進化しています。時代遅れのプラクティスや非推奨の機能を使用すると、互換性の問題、パフォーマンスの低下、またはセキュリティリスクにつながる可能性があります。

**ルール:**

- 常に `rust-toolchain.toml` でRustバージョンを確認する。
- 現在のRustバージョンのベストプラクティスに従ってコーディングする。
- 非推奨のライブラリや関数の使用を避ける。公式ドキュメントやリリースノートを確認する。

## 2. インポートパスの解決

**理由:** 一貫したインポートパスは可読性とモジュール依存関係管理を向上させます。

**ルール:**

- Rustでは `mod` キーワードを使用してモジュールをインポート
- 相対インポート（`../`、`./`）は一般的に禁止。
- 類似したドメイン/パス名を再確認する。
- 標準/サードパーティモジュールと内部定義モジュールの間に空行を確保する。

### 例（インポートパスの解決）

```rust:src/usecase/handlers/owner_handlers.rs
// 良い例 
use anyhow::Result;
use umbral_pre::{SecretKey, PublicKey, KeyFrag};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::controller::{MessageHandler, MessageRouter};
use crate::domain::{
    entities::{ProcessEntity, ShareEntity, CapsuleEntity},
    repositories::{ProcessEntityRepository, ShareEntityRepository},
    value_objects::{ProcessId, ProcessRole, CryptoPhase},
};
use crate::service::{
    core::{CryptoService, ProcessManagementService},
    workflow::DistributionWorkflow,
};
```

## 3. コメント規約

**理由:** コードは誰にでも読めるべきです。しかし、過度または不適切に書かれたコメントはノイズになる可能性があります。

**ルール:**

- インポート文に `// Import xx` や `// 〇〇を追加` のようなコメントを残さない。
- コメントは英語で書く
- 「このコードが何をするか」を説明するコメントは書かず、「なぜこうするのか」を示すコメントのみを残す
- コードベースで冗長なコメントを避ける。タスクを完了する前に、'なぜ'を説明するコメント以外はすべて削除する。コメントは簡潔に保つ。

### 例（コメント規約）

```rust
// 悪い例:
// Define CryptoService struct
pub struct CryptoService {
    pub umbral: Arc<UmbralService>,
}

// 良い例:
// Separate Umbral operations from Shamir to prevent mixing threshold schemes
pub fn new_crypto_service(config: CryptoConfig) -> Result<CryptoService> {
    let umbral_service = Arc::new(UmbralService::new(config.umbral)?);
    let shamir_service = Arc::new(ShamirService::new(config.shamir)?);

    Ok(CryptoService {
        umbral: umbral_service,
        shamir: shamir_service,
    })
}
```

## 4. ロギング

**理由:** ログは監視とトラブルシューティングに不可欠です。構造化ログ（例：JSON）は分析が容易です。

**ルール:**

- フォーマットされた文字列に変数を直接埋め込む代わりに、キーと値のペアを使用してイベントをログに記録する。これにより、簡単に解析できるコンテキストが提供される。
- tracingマクロが提供するフィールド構文を使用する（例：event!(Level::INFO, key = value, message = "...") または info!(key = value, "message") のような短縮マクロ）
- tracing::span! または #[tracing::instrument] 属性マクロを使用して、論理的な作業単位やコンテキストを定義する。

### 例（悪い例を避け、良い例を適用）

```rust
use tracing::{info, warn, Level};
use uuid::Uuid;

fn process_order(order_id: Uuid, user_id: &str, item_count: u32) {
    // 悪い例: 非構造化ログ - 確実な解析とフィルタリングが困難
    println!("Processing order {} for user {} with {} items", order_id, user_id, item_count);

    // 良い例: tracingを使用した構造化ログ
    info!(
        order_id = %order_id,
        user_id,
        item_count,
        "Processing customer order"
    );

    if item_count == 0 {
        // 別の構造化イベント
        warn!(order_id = %order_id, "Order received with zero items");
    }

    info!(order_id = %order_id, "Order processed successfully");
}
```

## 5. Cargoバージョンとベストプラクティス

**理由:** Rustエコシステムは進化しています。時代遅れのプラクティスや非推奨の機能を使用すると、互換性の問題、パフォーマンスの低下、またはセキュリティリスクにつながる可能性があります。

**ルール:**

- 常に `Cargo.toml` と `rust-toolchain.toml` でCargoと依存関係のバージョンを確認する。
- 現在のCargoバージョンのベストプラクティスに従ってコーディングする。
- 非推奨のライブラリや関数の使用を避ける。公式ドキュメントやリリースノートを確認する。

### 例

```toml
[toolchain]
channel = "1.86.0"  // このバージョンを確認
components = ["rustc", "cargo"]
profile = "minimal"
targets = []
```

## 6. `src/di.rs` を編集する時

**理由:** 依存性注入（DI）のセットアップは主に `src/di.rs` にありますが、各レイヤー（Usecase、Infra）にもDIコンテナがあります。通常、新機能の追加はレイヤー固有のコンテナの変更のみが必要です。`src/di.rs` への不必要な編集は競合を引き起こしたり、全体的なDI構造を不明瞭にする可能性があります。

**ルール:**

- `src/di.rs` は、**新しい依存コンポーネント**の追加（例：新しいDB接続、新しい外部サービスクライアント）など、全体的なDI構造に影響する変更のみ編集する。

- 新しいUsecases、Repositories、Controllersの追加は通常、それぞれのレイヤーのDIセットアップ（例：`src/usecase/usecases.rs`、`src/infrastructure/repositories.rs`）の更新のみが必要で、`di.rs` ではない。

## 7. 暗号実装ルール

**理由:** FORMIXは機密性の高い暗号操作を扱います。不適切な実装はセキュリティの脆弱性、タイミング攻撃、または秘密情報のメモリリークにつながる可能性があります。

**ルール:**

### 7.1 秘密情報のメモリ管理

- **常にZeroizeを使用**: 秘密情報を含むすべての構造体は `Zeroize` と `ZeroizeOnDrop` を導出する必要がある
- **秘密情報にCloneなし**: 秘密情報を含む型は、偶発的なコピーを防ぐために `Clone` を実装すべきでない
- **明示的な秘密情報型**: 明確さのために `secrecy::Secret<T>` または類似のラッパーを使用

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

// 良い例: 適切な秘密情報管理
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterSecret {
    #[zeroize(skip)] // 非秘密フィールドのみ
    pub id: String,
    secret_key: Vec<u8>, // ドロップ時にゼロ化される
}

// 悪い例: ゼロ化なしの秘密情報
pub struct InsecureSecret {
    secret_key: Vec<u8>, // メモリがクリアされない！
}
```

### 7.2 定数時間操作

- **subtleクレートを使用**: 定数時間でなければならない比較のために
- **秘密情報での分岐を避ける**: 秘密の値に基づくif文なし
- **暗号ライブラリを使用**: 暗号プリミティブを自分で実装しない

```rust
use subtle::ConstantTimeEq;

// 良い例: 定数時間比較
pub fn verify_secret(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

// 悪い例: タイミング脆弱性のある比較
pub fn verify_secret_bad(a: &[u8], b: &[u8]) -> bool {
    a == b // タイミングリーク！
}
```

### 7.3 Umbral-PRE固有のルール

- **鍵生成**: 常にumbral-preから `SecretKey::random()` を使用
- **シリアライゼーション**: 暗号オブジェクトにはumbralの組み込みシリアライゼーションを使用
- **エラーハンドリング**: エラーメッセージで暗号の詳細を決して公開しない

```rust
use umbral_pre::{SecretKey, PublicKey, encrypt, decrypt_original};

// 良い例: 適切なUmbralの使用
pub fn generate_keypair() -> Result<(SecretKey, PublicKey), CryptoError> {
    let sk = SecretKey::random();
    let pk = sk.public_key();
    Ok((sk, pk))
}

// 悪い例: エラーで暗号の詳細を公開しない
pub fn decrypt_bad(sk: &SecretKey, capsule: &Capsule, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    decrypt_original(sk, capsule, ciphertext)
        .map_err(|e| format!("Decryption failed with key {:?}: {}", sk, e)) // 秘密情報がリーク！
}
```

### 7.4 AOステートレス暗号の考慮事項

- **静的な秘密情報なし**: 秘密情報はメッセージ間で永続化できない
- **明示的な鍵のロード**: 鍵は各メッセージでRepositoryからロードする必要がある
- **アトミック操作**: 暗号操作は単一メッセージ内で完了する必要がある

## 8. エンティティの定義と責任

**理由:** エンティティはコアビジネスコンセプト（DDD）を表します。エンティティ内に状態と振る舞いをカプセル化することで、凝集性と保守性が向上します。

**ルール:**

- **エンティティの特定:** エンティティはドメインモデルで定義されたテーブルに対応する
- **エンティティごとにファイル:** 各エンティティに対して個別のファイルを作成
- **外部依存なし:** エンティティは純粋なドメインオブジェクトでなければならない。JSON（`json:"..."`）やDB永続化（`db:"..."`）などの外部関心事に関連する**アノテーションを追加しない**。これらはInterface/Infraレイヤーに属する。
- **カプセル化:** エンティティフィールドは**プライベートでなければならない**。アクセス/変更はメソッドを通じてのみ行われる。
- **必須コンストラクタ:** エンティティが有効な状態（不変条件を満たす）で作成されることを保証する責任を持つ**コンストラクタを常に提供する**（`NewUser`）。
- **コンストラクタの使用:** InfraレイヤーのRepositoriesで永続化から再構築する場合を**除いて**、エンティティインスタンスの作成には**常にコンストラクタを使用する**。これにより不変条件が保証される。
- **状態変更メソッド:** 状態を変更する操作をエンティティメソッドとして実装する。これらのメソッド内でドメインルールと不変条件をチェックする。

### 例（エンティティの定義）

```rust
use crate::domain::value_objects::{ProcessId, SecretId, ShareId};

// NG例: パブリックフィールドはカプセル化ルールに違反する。
// また、`serde`のようなアノテーションはエンティティではなくInfraレイヤーに属する。
// #[derive(serde::Serialize, serde::Deserialize)] 
pub struct ProcessEntityBad {
    pub id: ProcessId, // NG: パブリックフィールド
    pub role: ProcessRole, // NG: パブリックフィールド
    pub secret_id: Option<SecretId>, // NG: パブリックフィールド
    pub phase: CryptoPhase, // NG: パブリックフィールド
}

// OK例: プライベートフィールドがカプセル化を保証する。
// アクセスと変更はメソッドを通じて制御される。
#[derive(Debug, Clone, PartialEq, Eq)] // 基本的なderiveは許容される
pub struct ProcessEntity {
    id: ProcessId, // OK: プライベート（デフォルト）
    role: ProcessRole, // OK: プライベート
    secret_id: Option<SecretId>, // OK: プライベート
    phase: CryptoPhase, // OK: プライベート
    created_at: u64, // OK: プライベート
}

impl ProcessEntity {
    // バリデーション付きで新しいProcessEntityを作成するコンストラクタ
    pub fn new(id: ProcessId, role: ProcessRole) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            role,
            secret_id: None,
            phase: CryptoPhase::Initialized,
            created_at: current_timestamp(),
        })
    }

    // idフィールドのゲッター
    pub fn id(&self) -> &ProcessId {
        &self.id
    }

    // roleフィールドのゲッター
    pub fn role(&self) -> ProcessRole {
        self.role
    }

    // phaseフィールドのゲッター
    pub fn phase(&self) -> CryptoPhase {
        self.phase
    }

    // バリデーション付き状態遷移
    pub fn transition_to(&mut self, new_phase: CryptoPhase) -> Result<(), DomainError> {
        if !self.is_valid_transition(&new_phase) {
            return Err(DomainError::InvalidPhaseTransition {
                from: self.phase,
                to: new_phase,
            });
        }
        self.phase = new_phase;
        Ok(())
    }

    // 有効な遷移のためのドメインロジック
    fn is_valid_transition(&self, new_phase: &CryptoPhase) -> bool {
        use CryptoPhase::*;
        match (self.role, self.phase, new_phase) {
            (ProcessRole::Owner, Initialized, KeyGeneration) => true,
            (ProcessRole::Owner, KeyGeneration, SecretSplitting) => true,
            (ProcessRole::Holder, Initialized, Ready) => true,
            _ => false,
        }
    }
}
```

## 9. プロセスロール管理

**理由:** FORMIXには3つの異なるプロセスロール（Owner、Holder、Requester）があり、それぞれ異なる権限と責任を持ちます。ロールの混合はセキュリティの脆弱性につながります。

**ルール:**

### 9.1 ロールの分離

- **プロセスごとに単一ロール**: 各AOプロセスインスタンスは正確に1つのロールを持つ
- **ロール固有のハンドラー**: UseCaseハンドラーはロールごとに分離される
- **クロスロールアクセスなし**: ハンドラーは他のロールの機能にアクセスできない

```rust
// 良い例: ロール固有のハンドラー構造
pub mod handlers {
    pub mod owner_handlers {
        pub fn handle_init(msg: InitMessage) -> Result<Response> { /* ... */ }
        pub fn handle_split(msg: SplitMessage) -> Result<Response> { /* ... */ }
    }
    
    pub mod holder_handlers {
        pub fn handle_store_kfrag(msg: KFragMessage) -> Result<Response> { /* ... */ }
        pub fn handle_reencrypt(msg: ReencryptMessage) -> Result<Response> { /* ... */ }
    }
    
    pub mod requester_handlers {
        pub fn handle_request_access(msg: AccessMessage) -> Result<Response> { /* ... */ }
        pub fn handle_collect_cfrags(msg: CollectMessage) -> Result<Response> { /* ... */ }
    }
}
```

### 9.2 ロール認証

- **エントリーでロールをチェック**: 処理前にプロセスロールを確認
- **即座に失敗**: 間違ったロールのメッセージは即座に拒否
- **違反をログ**: 不正アクセス試行を記録

```rust
// 良い例: ロール検証パターン
pub fn route_message(msg: AOMessage, process_role: ProcessRole) -> Result<Response> {
    match (process_role, &msg.action) {
        (ProcessRole::Owner, Action::Split) => owner_handlers::handle_split(msg),
        (ProcessRole::Holder, Action::Reencrypt) => holder_handlers::handle_reencrypt(msg),
        (ProcessRole::Requester, Action::RequestAccess) => requester_handlers::handle_request_access(msg),
        (role, action) => {
            // 不正な試行をログ
            warn!("Role {} attempted unauthorized action {}", role, action);
            Err(AuthorizationError::UnauthorizedAction)
        }
    }
}
```

### 9.3 ロール固有の状態

- **個別の状態型**: 各ロールは独自の状態構造を持つ
- **共有秘密情報なし**: ロールは互いの暗号マテリアルにアクセスできない
- **最小権限**: 各ロールは必要なデータのみを持つ

```rust
// 良い例: ロール固有の状態構造
#[derive(Serialize, Deserialize)]
pub enum ProcessState {
    Owner(OwnerState),
    Holder(HolderState),
    Requester(RequesterState),
}

pub struct OwnerState {
    master_key_id: String,
    shares_generated: Vec<ShareInfo>,
}

pub struct HolderState {
    holder_id: u32,
    stored_kfrags: Vec<KFragInfo>,
}

pub struct RequesterState {
    access_requests: Vec<RequestInfo>,
    collected_cfrags: Vec<CFragInfo>,
}
```
