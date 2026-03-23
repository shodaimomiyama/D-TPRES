# HyperBEAM コントラクト開発ガイド

`ao/contracts/src/` — HyperBEAM 初心者向けのコード解説。

---

## 全体像: 5ファイルの役割

```
ao/contracts/src/
├── lib.rs              ← エントリポイント（HyperBEAM が呼ぶ関数）
├── message.rs          ← 入出力の型定義（JSON ↔ Rust）
├── state.rs            ← プロセスの状態（メモリ上に永続化）
├── handlers.rs         ← ビジネスロジック（9つのアクション）
└── getrandom_impl.rs   ← 乱数（WASM 用 placeholder）
```

---

## 1. lib.rs — HyperBEAM との接点

HyperBEAM は「WASM バイナリの中の特定の関数を呼ぶ」ことでコントラクトを実行する。Web サーバーでいう「HTTP リクエストを受けて JSON を返す」のと似ている。

```
HyperBEAM                          WASM コントラクト
─────────                          ──────────────────
1. alloc(size) で領域確保      →    メモリを確保して ptr を返す
2. ptr にメッセージ JSON を書く
3. handle(ptr, len) を呼ぶ      →    JSON をパース → 処理 → 結果を内部バッファに書く
4. response_len() で長さ取得    →    バッファの長さを返す
5. 返されたポインタから読む     ←    レスポンス JSON
6. dealloc(ptr, size) で解放    →    メモリを解放
```

```rust
// lib.rs のコア部分（簡略化）

// ─── グローバル状態（HyperBEAM がメモリごとスナップショットする） ───
static PROCESS_STATE: SyncCell<Option<ProcessState>> = ...;
static RESPONSE_BUF: SyncCell<Vec<u8>> = ...;

// ─── HyperBEAM が呼ぶ4つの関数 ───
pub extern "C" fn alloc(size: i32) -> i32          // メモリ確保
pub extern "C" fn handle(ptr: i32, len: i32) -> i32 // メイン処理
pub extern "C" fn response_len() -> i32             // レスポンス長
pub extern "C" fn dealloc(ptr: i32, size: i32)      // メモリ解放
```

**ポイント**: `handle` の中で JSON をパースし、`handlers::dispatch` に渡すだけ。ルーティングのフロントコントローラー的な役割。

---

## 2. message.rs — メッセージの型

Web API でいう「リクエスト/レスポンスの型定義」にあたる。

```rust
// ─── 入力（HyperBEAM → コントラクト）───
pub struct AOMessage {
    pub action: String,              // "DelegateKFrag", "GetCFrag" 等
    pub from: Option<String>,        // 送信者のアドレス
    pub id: Option<String>,          // メッセージID
    pub data: Option<serde_json::Value>,  // アクション固有のペイロード
}

// ─── 出力（コントラクト → HyperBEAM）───
pub struct AOResponse {
    pub ok: bool,                    // 成功/失敗
    pub data: Option<Value>,         // 成功時のレスポンスデータ
    pub error: Option<String>,       // 失敗時のエラーメッセージ
    pub messages: Vec<OutgoingMessage>,  // 他プロセスへの転送メッセージ
}

// ─── 他プロセスへの転送 ───
pub struct OutgoingMessage {
    pub target: String,   // 送信先プロセスID
    pub action: String,   // 転送するアクション名
    pub data: Value,      // 転送するデータ
}
```

**Web API との対比**:

| Web API | AO コントラクト |
|---------|----------------|
| HTTP Method (GET/POST) | `action` フィールド |
| Request Body | `data` フィールド |
| Response JSON | `AOResponse` |
| Webhook (別サーバーに通知) | `OutgoingMessage` |

---

## 3. state.rs — メモリ上の永続状態

Web サーバーでいう「データベース」にあたるが、全部インメモリの HashMap。

```rust
pub struct ProcessState {
    // ─── 初期化情報 ───
    pub role: Option<ProcessRole>,   // Owner / Holder / Requester / Combined
    pub owner_id: Option<String>,    // 初期化した人のアドレス（認可に使用）

    // ─── Owner 側のデータ ───
    pub owner_kfrags: HashMap<String, Vec<u8>>,
    //   "secret-1_0" → [bincode bytes...]
    //   鍵フラグメントのバイト列

    pub kfrag_holders: HashMap<String, String>,
    //   "secret-1_0" → "holder-process-xyz"
    //   どの kFrag をどの Holder に委任したか

    // ─── Capsule 管理 ───
    pub owner_capsules: HashMap<String, OwnerCapsuleData>,
    //   "9:secret-1_0/cap-001" → { capsule_bytes, status, ... }
    //   暗号化カプセルとその処理状態

    pub kfrag_to_caps: HashMap<String, Vec<String>>,
    //   "secret-1_0" → ["cap-001", "cap-002"]
    //   kFrag に紐づく capsule ID の逆引きインデックス

    // ─── Holder 側のデータ ───
    pub holder_cfrags: HashMap<String, Vec<u8>>,
    //   "9:secret-1_0/cap-001" → [cfrag bytes...]
    //   再暗号化で生成された cFrag
}
```

### データの流れ

```
DelegateKFrag で入る         DelegateCapsule で入る       再暗号化で生成
─────────────────          ────────────────────        ────────────────
owner_kfrags               owner_capsules              holder_cfrags
┌─────────┬──────┐        ┌──────────┬──────────┐    ┌──────────┬──────┐
│kfrag_id │bytes │        │cap_key   │status    │    │cap_key   │cfrag │
├─────────┼──────┤        ├──────────┼──────────┤    ├──────────┼──────┤
│secret_0 │[...]│   →    │s_0/cap01 │Received  │ → │s_0/cap01 │[...]│
│secret_1 │[...]│        │s_0/cap01 │CFragReady│    │          │      │
└─────────┴──────┘        └──────────┴──────────┘    └──────────┴──────┘

kfrag_holders              kfrag_to_caps
┌─────────┬──────────┐    ┌─────────┬────────────┐
│kfrag_id │holder_pid│    │kfrag_id │capsule_ids │
├─────────┼──────────┤    ├─────────┼────────────┤
│secret_0 │holder-A  │    │secret_0 │[cap01,cap02]│
└─────────┴──────────┘    └─────────┴────────────┘
```

### Capsule のステータス遷移

```
Received → ReencInProgress → CFragReady
                           → Error(msg)
```

### セキュリティ関連の型

```rust
// kFrag の中間デシリアライズ用。使用後にメモリがゼロ化される
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct StoredKeyFrag {
    pub id: String,
    pub key_data: Vec<u8>,           // Umbral KeyFrag のバイト列
    pub verification_data: Vec<u8>,  // 検証用公開鍵データ
    pub precursor: Vec<u8>,          // 将来拡張用
}

// cFrag の中間デシリアライズ用
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct StoredCFrag {
    pub fragment_data: Vec<u8>,
}
```

---

## 4. handlers.rs — ビジネスロジック

Web API でいう「コントローラー + サービス層」。`dispatch` 関数がルーター、各 `handle_*` がハンドラー。

### ルーティング

```rust
pub fn dispatch(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    // 未初期化なら Init のみ受付
    if msg.action == "Init" { return handle_init(state, msg); }
    if !state.is_initialized() { return error("send Init first"); }

    // ロールに応じてアクションを許可
    match (role, msg.action.as_str()) {
        // Owner/Combined のみ
        (Owner | Combined, "DelegateKFrag")   => handle_delegate_kfrag(state, msg),
        (Owner | Combined, "DelegateCapsule") => handle_delegate_capsule(state, msg),

        // Holder/Combined のみ
        (Holder | Combined, "SubmitKFrag")    => handle_submit_kfrag(state, msg),
        (Holder | Combined, "SubmitCapsule")  => handle_submit_capsule(state, msg),
        (Holder | Combined, "Reencrypt")      => handle_reencrypt(state, msg),

        // 全ロール（Query）
        (_, "GetCFrag")      => handle_get_cfrag(state, msg),
        (_, "ListCapsules")  => handle_list_capsules(state, msg),

        _ => error("Action not permitted for role"),
    }
}
```

### 各ハンドラーの処理概要

| ハンドラー | やること | 認可 | 冪等 |
|------------|----------|------|------|
| `handle_init` | ロール設定 + owner_id 登録 | from が必須 | No (二重初期化は拒否) |
| `handle_delegate_kfrag` | kFrag 保存 + Holder に SubmitKFrag 転送 | owner_id 一致 | No |
| `handle_delegate_capsule` | Capsule 保存 + Holder に SubmitCapsule 転送 | owner_id 一致 | No |
| `handle_submit_kfrag` | bincode 検証 → kFrag 保存 | owner_id 一致 | **Yes** (既存なら skip) |
| `handle_submit_capsule` | Capsule 保存 → **即座に再暗号化** | owner_id 一致 | **Yes** (cFrag 既存なら skip) |
| `handle_reencrypt` | 既存 Capsule の再暗号化リトライ | owner_id 一致 | No |
| `handle_get_cfrag` | cFrag を返す | 認可不要 | - |
| `handle_list_capsules` | kFrag に紐づく capsule ID 一覧 | 認可不要 | - |

### 再暗号化の中身 (`perform_reencryption`)

最も重要な関数。Umbral PRE の再暗号化をオンチェーンで実行する。

```
入力: kfrag_bytes (bincode), capsule_bytes

Step 1: bincode::deserialize → StoredKeyFrag
Step 2: key_data / verification_data の空チェック
Step 3: verification_data → VerificationData { verifying_pk, delegating_pk, receiving_pk }
Step 4: 各公開鍵を bincode デシリアライズ
Step 5: KeyFrag::from_bytes → key_frag.verify(verifying_pk, delegating_pk, receiving_pk)
        → 検証失敗なら拒否（改竄検知）
Step 6: Capsule::from_bytes
Step 7: umbral_pre::reencrypt(&capsule, verified_kfrag) → cFrag 生成
Step 8: cFrag を bincode シリアライズして返す
```

---

## 5. getrandom_impl.rs — WASM 用乱数

WASM 環境には OS の `/dev/urandom` がないため、`getrandom` crate のカスタム実装が必要。

```rust
getrandom::register_custom_getrandom!(ao_getrandom);

pub fn ao_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    // ⚠️ 決定論的な placeholder — 本番では使用不可
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
    }
    Ok(())
}
```

**現状**: `umbral-pre` のコンパイルを通すためのスタブ。本番ではメッセージ ID やブロックハッシュからエントロピーを注入する必要がある。

---

## 処理の全体フロー（メッセージ 1 件の一生）

```
HyperBEAM
  │
  ├─ 前回のメモリスナップショットをリストア
  │
  ├─ alloc(len) → ptr を取得
  ├─ ptr に JSON を書き込み
  ├─ handle(ptr, len) を呼ぶ
  │     │
  │     ├─ JSON パース → AOMessage
  │     ├─ dispatch(state, msg)
  │     │     ├─ ロール確認
  │     │     ├─ 認可チェック (from == owner_id)
  │     │     ├─ ハンドラー実行 (state を &mut で変更)
  │     │     └─ AOResponse を返す
  │     │
  │     ├─ AOResponse を JSON シリアライズ
  │     └─ RESPONSE_BUF に書いて ptr を返す
  │
  ├─ response_len() でサイズ取得
  ├─ レスポンス読み取り
  ├─ dealloc で解放
  │
  └─ 現在のメモリ全体をスナップショット保存
      → state の変更が永続化される
```

---

## ステート管理で開発者が意識すべきこと

### メモリ上のすべてが永続化される

KV ストアなら「保存しなければ消える」が、HyperBEAM では **メモリに存在する = 次のメッセージでも存在する**。不要なデータを `static` に蓄積するとスナップショットサイズが増え続ける。

**対策**: 不要になったエントリは明示的に `remove` する。

### メモリは増えるが縮まない

WASM のリニアメモリは `memory.grow` で拡張されるが、**縮小する手段がない**。大量データの insert/remove を繰り返すとメモリフットプリントが肥大化する。

**対策**: 大量データの一時保存を避ける設計にする。完了した capsule データのクリーンアップ戦略を検討する。

### 秘密鍵素材は明示的にゼロ化が必要

メモリが永続化されるため、スコープを抜けてもヒープ上にゴーストデータが残る可能性がある。

**対策**: 秘密素材を扱う型には必ず `Zeroize` + `ZeroizeOnDrop` を derive する（現在の実装は対応済み）。

### メッセージ処理は原子的だが、エラー時もステートは保存される

HyperBEAM はハンドラーが正常に return した時点の状態をスナップショットする。パニックした場合は前回のスナップショットに戻る。しかし **エラーを return しても状態変更は保存される**。

**対策**: エラー時にロールバックが必要なら、明示的に元に戻すコードを書く。

### コントラクトのアップグレードが困難

メモリスナップショットはバイナリレベルでの互換性が必要。構造体のレイアウトが変わると旧スナップショットから復元した状態が壊れる可能性がある。

**対策**: `HashMap<String, Vec<u8>>` のように動的なスキーマを使う（現在の設計はこれに近い）。

### まとめ表

| 観点 | KV ストア脳 | メモリスナップショット脳 |
|------|-------------|------------------------|
| 保存 | 明示的に write | **何もしなくても保存される** |
| 削除 | key を delete | `remove` してもメモリは縮まない |
| 一時データ | 保存しなければ消える | **static に置くと永続化される** |
| エラー時 | transaction rollback | **明示的にロールバックが必要** |
| 大量データ | 必要な分だけ load | 常に全量がメモリ上 |
| 秘密鍵 | key 削除で消える | **Zeroize 必須** |
