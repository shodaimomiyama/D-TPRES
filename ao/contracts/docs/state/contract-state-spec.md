# CosmWasm on AO｜プロセスのステート管理に関する主要機能ミニマム仕様書

> **目的**
> 単一 AO プロセス内で Owner / Holder / Requester の3ロールを論理的に扱い、
> `kFrag` と `Capsule` から **1 Capsule あたり 1 cFrag** を生成・保持・配布する最小実装。
> この仕様は、これまでの議論での**改善点・深掘り**（順序固定・index運用・監査・チャンク方針 等）を反映済み。
> 参照元はご指定の GitHub ページ（CosmWasm on AO）。本文の転載は行わない。

---

## 0. 前提・ポリシー

* **プロセス主キー**: `process.id` を状態キー空間のルートに採用（署名ウォレットは監査用メタに記録）。
* **ロール**: Owner / Holder / Requester（**同一プロセスに論理共存**）。
  * **Ownerロール**: ClientからkFrag/Capsuleを受信し、適切なHolderプロセスに委譲
  * **Holderロール**: kFrag/Capsuleを保管し、再暗号化を実行してcFragを生成
  * **Requesterロール**: cFragの取得と一覧表示
* **順序固定**: **必ず `handle_delegate_kFrag` → `handle_delegate_cFrag` → `handle_get_cFrag`** の順で到着する前提。

  * `kFrag` 未登録で `handle_delegate_cFrag` が来たら **`ERR_KFRAG_NOT_FOUND`**（何も書かない）。
* **同期制約**: 他プロセスの同期読取なし（必要データは当該プロセスに保持）。
* **ACL/DoS**: 当面なし（将来拡張前提）。
* **Holder選出**: 現在はハードコード（`DEFAULT_HOLDER_PROCESS_ID`）。将来的にRandAO統合予定。
* **監査・チャンク**: **将来実装予定**（AOのイベントソーシングで代替）。
* **一覧ページング**: 既定 `limit = 50`。

---

## 1. ドメインIDと冪等

* `kFragID`（≤128文字）, `capsuleID`（≤128文字）
* **cFrag** は `(kFragID, capsuleID)` に対して **常に1つ**。
* 冪等フラグ: `idem|process.id|kFragID|capsuleID = true` で**重複生成を完全No-Op**。

---

## 2. 状態スキーマ（KV＝エンティティ）とキー設計

> **命名規則**: `role|<process.id>|...` を先頭に置き、**プレフィックス走査**を最短化。
> **値**は原則バイト列＋軽メタ（`size_bytes`, `sha256`, `updated_ts`）。
> **チャンク分割**時のみ `...|meta` / `...|part/<i>` を追加。

### 2.1 本体（Owner / Holder）

#### キーテンプレート表記の読み方

KVストレージのキー表記 `owner|<process.id>|kfrag|<kFragID>` は以下のように解釈します：

* **パイプ記号（`|`）**: 階層的な名前空間を作成するセパレータ
* **固定文字列**: `owner`, `kfrag` など（リテラル文字列として使用）
* **変数部分**: `<process.id>`, `<kFragID>` など（実際の値に置換される）

**具体例**: process.id=`"123"`, kFragID=`"A"`, capsuleID=`"cap001"` の場合

```
OWNER_KFRAGS:
  テンプレート: owner|<process.id>|kfrag|<kFragID>
  実際のキー:   owner|123|kfrag|A

OWNER_CAPSULES:
  テンプレート: owner|<process.id>|capsule|<kFragID>|<capsuleID>
  実際のキー:   owner|123|capsule|A|cap001

KFRAG_HOLDERS:
  テンプレート: kfrag_holders|<process.id>|<kFragID>
  実際のキー:   kfrag_holders|123|A

HOLDER_CFRAGS:
  テンプレート: holder|<process.id>|cfrag|<kFragID>|<capsuleID>
  実際のキー:   holder|123|cfrag|A|cap001
```

#### CosmWasm/cw-storage-plus での実装

```rust
// Map定義（タプルキー）
pub const OWNER_KFRAGS: Map<(String, String), OwnerKFragData> =
    Map::new("owner_kfrags");

// 使用例
let key = (process_id.clone(), kfrag_id.clone()); // ("123", "A")
OWNER_KFRAGS.save(deps.storage, key, &kfrag_data)?;

// 内部的には namespace + encode(tuple) でキーが生成される
// 実際のストレージキー: "owner_kfrags" + binary_encoded(("123", "A"))
```

#### Prefix スキャンの仕組み

```rust
// kFragID="A" に関連する全Capsuleを取得
let prefix = (process_id.clone(), kfrag_id.clone()); // ("123", "A")
let capsules = INDEX_KFRAG_TO_CAPS
    .prefix(prefix)  // "holder|123|index|kfrag_to_caps|A|*" の範囲をスキャン
    .range(deps.storage, None, None, Order::Ascending)
    .collect::<StdResult<Vec<_>>>()?;
```

#### キー設計の利点

1. **階層的な整理**: ロール（owner/holder）→プロセス→リソースタイプ→ID
2. **効率的なスキャン**: Prefixによる範囲検索が高速
3. **名前空間の分離**: 異なるプロセス間でデータが混在しない
4. **可読性**: デバッグ時にキーを見て内容が推測可能

---

#### KVマッピング定義

* **OWNER_KFRAGS**

  * key: `owner|<process.id>|kfrag|<kFragID>`
  * value: `{ kFrag_bytes, meta }`
* **OWNER_CAPSULES**

  * key: `owner|<process.id>|capsule|<kFragID>|<capsuleID>`
  * value: `{ capsule_bytes, status, meta }`
  * `status ∈ { RECEIVED, REENC_IN_PROGRESS, CFRAG_READY, ERROR }`
* **KFRAG_HOLDERS**（Ownerロール用：kFragとHolderプロセスの対応）

  * key: `owner|<process.id>|kfrag_holder|<kFragID>`
  * value: `{ holder_process_id }`
  * 現在はハードコード（`DEFAULT_HOLDER_PROCESS_ID`）、将来的にRandAOで選出
* **HOLDER_CFRAGS**

  * key: `holder|<process.id>|cfrag|<kFragID>|<capsuleID>`
  * value: `{ cFrag_bytes, meta }`

### 2.2 インデックス（列挙／存在確認）

* **INDEX_KFRAG_TO_CAPS**（kFrag → Capsule 列挙）

  * key: `holder|<process.id>|index|kfrag_to_caps|<kFragID>|<capsuleID>`
  * value: `{ status, updated_ts }`（軽量）
* **INDEX_CAPS_TO_CFRAG**（Capsule → cFrag 存在）

  * key: `holder|<process.id>|index|caps_to_cfrag|<kFragID>|<capsuleID>`
  * value: `{ exists: true, updated_ts }`

### 2.3 冪等・将来実装予定機能

* **IDEM_FLAGS**

  * key: `idem|<process.id>|<kFragID>|<capsuleID>`
  * value: `{ generated: true, updated_ts }`

### 2.4 将来実装予定（現在未実装）

* **AUDIT_LOGS**（Append-only）- **AOのイベントソーシングで代替**

  * key: `audit|<ext_ts>|<EVENT>|...`
  * value: `{ event, actor_wallet, kFragID, capsuleID, reason_or_meta }`
  * 理由：AOはメッセージ履歴を完全に記録するため、独自の監査ログは冗長
* **CHUNK_META / CHUNK_PART**（4KB超のデータ分割）

  * meta key: `....|meta` → `{ size_bytes, parts, sha256, updated_ts }`
  * part key: `....|part/<i>` → `part_bytes`
  * 現在：4KB以下のデータのみサポート

---

## 3. メッセージ仕様（AO “ワイヤ”＋Input JSON）

### 3.1 ワイヤ共通（tags）

* `target`: `process.id`
* `data`: 空（通常未使用）
* `tags`（必須）

  * `App-Name`: 例 `cwao`
  * `Action`: 下記アクション名
  * `Read-Only`: `"True"` / `"False"`
  * `Input`: **JSON文字列**（各アクション入力）
  * `Process-Id`: `process.id`
  * `Actor`: 送信ウォレットアドレス
  * `Ts`: 外部時刻（RFC3339 または epoch-ms）→ 監査の `ext_ts` に採用

### 3.2 Execute

#### Ownerロール（Clientからの受信と委譲）

* **handle_delegate_kFrag**（順序：常に先行）

  ```json
  {
    "kFragID": "string<=128",
    "kFrag": "base64"
  }
  ```

  * ClientからkFragを受信し、ハードコードされたHolderプロセスに委譲
  * KFRAG_HOLDERSマップにHolder情報を記録
  * **SubMsg機能**: `WasmMsg::Execute`でHolderプロセスに`handle_submit_kFrag`を送信

* **handle_delegate_cFrag**（kFrag受信後）

  ```json
  {
    "kFragID": "string<=128",
    "capsuleID": "string<=128",
    "capsule": "base64"
  }
  ```

  * ClientからCapsuleを受信し、対応するHolderプロセスに委譲
  * kFrag未委譲なら `ERR_KFRAG_NOT_FOUND`
  * **SubMsg機能**: `WasmMsg::Execute`でHolderプロセスに`handle_submit_Capsule`を送信

#### Holderロール（プロセス間での処理）

* **handle_submit_kFrag**（Ownerプロセスから受信）

  ```json
  {
    "kFragID": "string<=128",
    "kFrag": "base64"
  }
  ```

  * 既存なら No-Op。

* **handle_submit_Capsule**（Ownerプロセスから受信）

  ```json
  {
    "kFragID": "string<=128",
    "capsuleID": "string<=128",
    "capsule": "base64"
  }
  ```

  * `kFrag` 未登録なら `ERR_KFRAG_NOT_FOUND`（本体/index含め**何も書かない**）。
  * 冪等ONなら完全No-Op。

* **Reencrypt**（運用リトライ）

  ```json
  {
    "kFragID": "string<=128",
    "capsuleID": "string<=128"
  }
  ```

### 3.3 Query

* **handle_get_cFrag**

  ```json
  {
    "kFragID": "string<=128",
    "capsuleID": "string<=128"
  }
  ```

  * 返却: `{ cFrag: "base64", meta: { size_bytes, sha256, updated_ts } }`
  * 監査: `GET_CFRAG` を必ず記録。

* **handle_list_Capsule_by_kFrag**

  ```json
  {
    "kFragID": "string<=128",
    "start_after": "string<=128?",
    "limit": 50
  }
  ```

  * 返却: `capsules[]` + `next_start_after`（ページング）

---

## 4. 動作フロー（厳密・順序固定）

### 4.0 Owner：handle_delegate_kFrag（ClientからkFrag受信と委譲）

1. 入力検証 → バリデーション完了
2. `DEFAULT_HOLDER_PROCESS_ID`を使用してHolderプロセスを決定
3. `put KFRAG_HOLDERS(pid, kFragID) = holder_process_id`
4. **SubMsg作成と送信**:
   ```rust
   let wasm_msg = WasmMsg::Execute {
       contract_addr: holder_process_id,
       msg: to_json_binary(&ExecuteMsg::handle_submit_kFrag { kfrag_id, kfrag }),
       funds: vec![],
   };
   let sub_msg = SubMsg::new(wasm_msg);
   ```
5. コミット（SubMsgは非同期で実行）

### 4.0.1 Owner：handle_delegate_cFrag（ClientからCapsule受信と委譲）

1. 入力検証
2. `get KFRAG_HOLDERS(pid, kFragID)` → **無**なら`ERR_KFRAG_NOT_FOUND`（**終了**）
3. **SubMsg作成と送信**:
   ```rust
   let wasm_msg = WasmMsg::Execute {
       contract_addr: holder_process_id,
       msg: to_json_binary(&ExecuteMsg::handle_submit_Capsule { kfrag_id, capsule_id, capsule }),
       funds: vec![],
   };
   let sub_msg = SubMsg::new(wasm_msg);
   ```
4. コミット（SubMsgは非同期で実行）

### 4.1 Holder：handle_submit_kFrag（Ownerプロセスから受信）

1. 入力検証 → 既存なら No-Op
2. `put owner|pid|kfrag|K = kFrag + meta`
3. コミット

### 4.2 Execute：handle_submit_Capsule（kFrag存在が前提）

1. 入力検証
2. `get owner|pid|kfrag|K` が **無** → `ERR_KFRAG_NOT_FOUND`（**終了**）
3. `get idem|pid|K|C` が **有** → **No-Op**（終了）
4. `put owner|pid|capsule|K|C = {capsule, status=RECEIVED, meta}`
5. `put index kfrag_to_caps|K|C = {status, ts}`
6. `update owner|pid|capsule|K|C.status = REENC_IN_PROGRESS`
7. 再暗号化 → cFrag 生成

   * 失敗 → `status=ERROR`, 監査 `ERROR`, 終了
8. `put holder|pid|cfrag|K|C = {cFrag, meta}`
9. `put index caps_to_cfrag|K|C = {exists:true, ts}`
10. `put idem|pid|K|C = true`
11. `update owner|pid|capsule|K|C.status = CFRAG_READY`
12. コミット

### 4.3 Query：handle_get_cFrag

1. 入力検証
2. `get index caps_to_cfrag|K|C` **無** → `ERR_CFRAG_NOT_READY`
3. `get holder|pid|cfrag|K|C`（必要ならチャンク結合）
4. 監査 `GET_CFRAG` 追記
5. 返却

### 4.4 Query：handle_list_Capsule_by_kFrag（limit=50 既定）

* prefix = `holder|pid|index|kfrag_to_caps|K|`
* `start_after` と `limit` でレンジ取得
* 軽メタ（status/updated_ts）をそのまま返却（必要に応じて本体補完）

---

## 5. ER 図（KV＝エンティティ、ロール別）

> **各エンティティ＝1つのKVマッピング**。**key → value** 対応を明示。
> RDBの外部キーは使わない（プレフィックス命名で疎結合）。

```mermaid
erDiagram
    KFRAG_HOLDERS {
      string key "kfrag_holders|<process.id>|<kFragID>"
      string holder_process_id
    }
    OWNER_KFRAGS {
      string key "owner|<process.id>|kfrag|<kFragID>"
      bytes  kFrag_bytes
      uint64 size_bytes
      string sha256_hex
      string updated_ts_rfc3339
    }
    OWNER_CAPSULES {
      string key "owner|<process.id>|capsule|<kFragID>|<capsuleID>"
      bytes  capsule_bytes
      string status "RECEIVED | REENC_IN_PROGRESS | CFRAG_READY | ERROR"
      uint64 size_bytes
      string sha256_hex
      string updated_ts_rfc3339
    }
    HOLDER_CFRAGS {
      string key "holder|<process.id>|cfrag|<kFragID>|<capsuleID>"
      bytes  cFrag_bytes
      uint64 size_bytes
      string sha256_hex
      string updated_ts_rfc3339
    }
    INDEX_KFRAG_TO_CAPS {
      string key "holder|<process.id>|index|kfrag_to_caps|<kFragID>|<capsuleID>"
      string status
      string updated_ts_rfc3339
    }
    INDEX_CAPS_TO_CFRAG {
      string key "holder|<process.id>|index|caps_to_cfrag|<kFragID>|<capsuleID>"
      bool   exists
      string updated_ts_rfc3339
    }
    IDEM_FLAGS {
      string key "idem|<process.id>|<kFragID>|<capsuleID>"
      bool   generated
      string updated_ts_rfc3339
    }

    KFRAG_HOLDERS ||..|| OWNER_KFRAGS : "kFrag委譲マッピング: <kFragID>"
    KFRAG_HOLDERS ||..|| OWNER_CAPSULES : "Holder選択: <kFragID>"
    OWNER_KFRAGS ||..|| OWNER_CAPSULES : "prefix関係: <kFragID>"
    OWNER_CAPSULES ||..|| HOLDER_CFRAGS : "論理従属: (kFragID,capsuleID)"
    OWNER_CAPSULES ||..|| INDEX_KFRAG_TO_CAPS : "列挙index"
    HOLDER_CFRAGS ||..|| INDEX_CAPS_TO_CFRAG : "存在index"
    OWNER_CAPSULES ||..|| IDEM_FLAGS : "冪等フラグ"
```

---

## 6. シーケンス図（順序固定・メモリ展開の同期）

> 各メッセージ実行前に、担当 CU は**最新状態をメモリに展開**（スナップショット＋差分適用）。
> コミット後の状態は永続化され、**CUが再割当されても同じ状態**で即実行できる。

```mermaid
sequenceDiagram
  autonumber
  participant CL as Client
  participant OP as Owner Process
  participant HP as Holder Process
  participant SU as Scheduler/Messenger
  participant CU as CUノード（実行）
  participant ST as 永続層

  %% A) Owner: handle_delegate_kFrag（委譲フロー）
  CL->>SU: Msg(Action=handle_delegate_kFrag, Input={K,kFrag}, Ts, Actor)
  SU->>CU: deliver to Owner Process
  CU->>ST: 最新状態ロード（メモリ展開）
  CU->>OP: execute handle_delegate_kFrag
  OP->>ST: put kfrag_holders|pid|K = DEFAULT_HOLDER_PROCESS_ID
  OP->>HP: SubMsg(Action=handle_submit_kFrag, Input={K,kFrag})
  HP->>ST: put owner|holder_pid|kfrag|K = kFrag + meta
  HP-->>OP: success
  OP-->>CU: ok
  CU-->>SU: success（コミット）

  %% B) Owner: handle_delegate_cFrag（委譲フロー）
  CL->>SU: Msg(Action=handle_delegate_cFrag, Input={K,C,capsule}, Ts, Actor)
  SU->>CU: deliver to Owner Process
  CU->>ST: 最新状態ロード
  CU->>OP: execute handle_delegate_cFrag
  OP->>ST: get kfrag_holders|pid|K ?（無: ERR_KFRAG_NOT_FOUND, 終了）
  OP->>HP: SubMsg(Action=handle_submit_Capsule, Input={K,C,capsule})
  HP->>ST: get owner|holder_pid|kfrag|K ?（無: ERR_KFRAG_NOT_FOUND, 終了）
  HP->>ST: put owner|holder_pid|capsule|K|C = capsule, status=RECEIVED
  HP->>ST: put index kfrag_to_caps|K|C = {status,ts}
  HP->>ST: update owner|holder_pid|capsule|K|C.status = REENC_IN_PROGRESS
  HP->>HP: 再暗号化→cFrag生成
  alt 成功
    HP->>ST: put holder|holder_pid|cfrag|K|C = cFrag + meta
    HP->>ST: put index caps_to_cfrag|K|C = {exists:true,ts}
    HP->>ST: put idem|holder_pid|K|C = true
    HP->>ST: update owner|holder_pid|capsule|K|C.status = CFRAG_READY
    HP-->>OP: success
  else 失敗
    HP->>ST: update owner|holder_pid|capsule|K|C.status = ERROR
    HP-->>OP: error
  end
  OP-->>CU: result
  CU-->>SU: result（コミット）

  %% C) handle_get_cFrag（Holderプロセスに直接クエリ）
  CL->>SU: Msg(Action=handle_get_cFrag, Input={K,C}, Ts, Actor)
  SU->>CU: deliver to Holder Process
  CU->>ST: 最新状態ロード
  CU->>HP: query handle_get_cFrag
  HP->>ST: get index caps_to_cfrag|K|C ?（無: ERR_CFRAG_NOT_READY）
  HP->>ST: get holder|holder_pid|cfrag|K|C
  HP-->>CU: {cFrag, meta}
  CU-->>CL: レスポンス

  %% D) CU再割当（別CUでも同じ）
  Note over SU,CU: 負荷分散・再起動などでCUが切替わっても
  Note over CU,ST: スナップショット＋差分で最新KVを復元→即実行可能
```

---

## 7. バリデーション & エラー

* **入力**: `kFragID/capsuleID`（非空・ASCII安全・≤128）、`kFrag/capsule`（Base64・サイズ>0）、`Ts`（任意時刻）
* **代表エラー**

  * `ERR_KFRAG_NOT_FOUND`（順序違反の `handle_submit_Capsule`。**何も書かない**）
  * `ERR_CFRAG_NOT_READY`（未生成の `handle_get_cFrag`）
  * `ERR_REENC_FAILED`（再暗号化失敗）
  * `ERR_DUPLICATE_KFRAG` / `ERR_DUPLICATE_CAPSULE`（通知が必要なら使用）
  * `ERR_BAD_REQUEST` / `ERR_OBJECT_TOO_LARGE`

---

## 8. 既定値・運用ノート

* `handle_list_Capsule_by_kFrag.limit` 既定 **50**
* 監査は **失敗** と **配布** を必ず記録（`ext_ts`=タグ`Ts`、`actor/requester`=タグ`Actor`）
* チャンクは **4KB超のみ**（最大32パーツ）。通常は非分割で高速I/O。
* index は「**一覧**（kfrag_to_caps）」と「**存在確認**（caps_to_cfrag）」に用途分離。
* 冪等フラグで**再送・重複**を完全No-Op。

---

## 9. 実装タスク

1. **instantiate**: `process.id` を state ルートに採用。
2. **KVユーティリティ**: `kv_put/kv_get/kv_scan_prefix`、キー生成（`role|process.id|...`）、冪等I/F。
3. **execute**: `handle_submit_kFrag` / `handle_submit_Capsule`（順序チェック・即時完結） / `Reencrypt`。
4. **query**: `handle_get_cFrag`（存在index→本体） / `handle_list_Capsule_by_kFrag(limit=50)`（prefixスキャン）。
5. **監査ユーティリティ**: 失敗/配布の記録（外部時刻とアクター転記）。
6. **SubMsg実装**（Owner→Holder委譲機能）:
   * `handlers.rs`にCosmosMsg, SubMsg, WasmMsg インポート追加
   * `handle_delegate_kfrag`と`handle_delegate_capsule`にSubMsg送信機能追加
   * `lib.rs`にreplyエントリーポイント追加（オプション）
   * Reply処理とエラーハンドリング実装
7. **テスト**:

   * 正常系: kFrag→Capsule→handle_get_cFrag、重複送信のNo-Op
   * 異常系: `handle_submit_Capsule`の順序違反、再暗号化失敗、未生成取得、巨大入力
   * ページング: `handle_list_Capsule_by_kFrag` 連続取得
   * 監査: 失敗＋配布が必ず残ること
   * **SubMsg**: SubMsg生成確認、プロセス間通信、エラー時のReply処理

---

# 10. CU再割当時の「状態復元」ステップ（Messaging / メモリ状態に着目）

> 目的：正常系後に**別のCU**へ割り当て直されても、**メッセージ履歴（スナップショット＋差分）**から**同一の最新状態**を再現し、以降の処理を継続できることを明確化。

### 10.1 事前状態（正常系完了後の永続KV）

* 例：`handle_submit_kFrag(K)`→`handle_submit_Capsule(K,C)`成功後

  * `OWNER_KFRAGS(pid,K)`：存在
  * `OWNER_CAPSULES(pid,K,C).status = CFRAG_READY`
  * `HOLDER_CFRAGS(pid,K,C)`：存在
  * `INDEX_KFRAG_TO_CAPS(pid,K,C)`：存在（status=CFRAG_READY）
  * `INDEX_CAPS_TO_CFRAG(pid,K,C)`：`exists=true`
  * `IDEM_FLAGS(pid,K,C)=true`
  * `AUDIT_LOGS`：必要に応じて `GET_CFRAG` 等が追加されている

### 10.2 CU再割当（割当直後の復元手順）

1. **SU（スケジューラ）→ 新CUに割当**
2. **新CU は永続層から最新スナップショット取得**
3. **スナップショット以降の差分メッセージを再生**

   * メッセージ順は **すでに順序固定**（`kFrag→Capsule→handle_get_cFrag`）のため、再生は直列で安全
4. **KVをメモリ上に展開**

   * `OWNER_* / HOLDER_* / INDEX_* / IDEM_* / AUDIT_*` が**直前と同一内容**に再構築
5. **実行準備完了**（以降の`execute/query`はこの最新状態で即時実行）

### 10.3 復元後のメッセージ到着（例：`handle_get_cFrag(K,C)`）

1. **Requester→SU→新CU** に配送
2. **新CU** はすでに最新KVをメモリに展開済み
3. **query処理**

   * `INDEX_CAPS_TO_CFRAG(pid,K,C)` を `get` → `exists=true` で即時肯定
   * `HOLDER_CFRAGS(pid,K,C)` を `get` → cFrag本体を返却
   * `AUDIT_LOGS` に `GET_CFRAG` を追記
4. **レスポンス返却**（再割当前と同一の結果）

> 結論：**スナップショット＋差分の適用 → KV再構築 → 直後の`execute/query`** という流れにより、**CUがどこでも一貫した最新状態**が保障される。

---

# 11. メッセージ仕様の「ワイヤ」完全例（tags含む）

> AOに送る**実体メッセージ**の例を示します（`data`は空を想定。CWAO流儀で`tags.Input`にJSONを詰める）。

### 11.1 handle_submit_kFrag（execute）

```json
{
  "target": "<process.id>",
  "data": "",
  "tags": [
    {"name":"App-Name","value":"cwao"},
    {"name":"Action","value":"handle_submit_kFrag"},
    {"name":"Read-Only","value":"False"},
    {"name":"Input","value":"{\"kFragID\":\"K\",\"kFrag\":\"<base64>\"}"},
    {"name":"Process-Id","value":"<process.id>"},
    {"name":"Actor","value":"<walletAddress_of_owner>"},
    {"name":"Ts","value":"2025-10-16T12:34:56Z"}
  ]
}
```

### 11.2 handle_submit_Capsule（execute）

```json
{
  "target": "<process.id>",
  "data": "",
  "tags": [
    {"name":"App-Name","value":"cwao"},
    {"name":"Action","value":"handle_submit_Capsule"},
    {"name":"Read-Only","value":"False"},
    {"name":"Input","value":"{\"kFragID\":\"K\",\"capsuleID\":\"C\",\"capsule\":\"<base64>\"}"},
    {"name":"Process-Id","value":"<process.id>"},
    {"name":"Actor","value":"<walletAddress_of_owner>"},
    {"name":"Ts","value":"2025-10-16T12:35:40Z"}
  ]
}
```

### 11.3 handle_get_cFrag（query）

```json
{
  "target": "<process.id>",
  "data": "",
  "tags": [
    {"name":"App-Name","value":"cwao"},
    {"name":"Action","value":"handle_get_cFrag"},
    {"name":"Read-Only","value":"True"},
    {"name":"Input","value":"{\"kFragID\":\"K\",\"capsuleID\":\"C\"}"},
    {"name":"Process-Id","value":"<process.id>"},
    {"name":"Actor","value":"<walletAddress_of_requester>"},
    {"name":"Ts","value":"2025-10-16T12:36:10Z"}
  ]
}
```

---

# 12. `state.rs` における KV マッピングとデータ構造


```rust
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};

pub const DEFAULT_LIST_LIMIT: u32 = 50;

// --------------------- 設定 ---------------------
#[cw_serde]
pub struct Config {
    pub process_id: String,
    pub default_list_limit: u32,
}
pub const CONFIG: Item<Config> = Item::new("config");

// Owner-Holder 委譲マッピング（ハードコード設定）
pub const KFRAG_HOLDERS: Map<(String, String), String> = Map::new("kfrag_holders");
pub const DEFAULT_HOLDER_PROCESS_ID: &str = "holder_process_placeholder";

// --------------------- メタ ---------------------
pub type Rfc3339String = String;

#[cw_serde]
pub struct BlobMeta {
    pub size_bytes: u64,
    pub sha256_hex: String,
    pub updated_ts: Rfc3339String,
}

// --------------------- ドメイン ---------------------
#[cw_serde]
pub enum CapsuleStatus { Received, ReencInProgress, CFragReady, Error }

#[cw_serde]
pub struct OwnerKFragData {
    pub kfrag: Binary,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct OwnerCapsuleData {
    pub capsule: Binary,
    pub status: CapsuleStatus,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct HolderCFragData {
    pub cfrag: Binary,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct IndexKFragToCapsValue {
    pub status: CapsuleStatus,
    pub updated_ts: Rfc3339String,
}

#[cw_serde]
pub struct IndexCapsToCFragValue {
    pub exists: bool,
    pub updated_ts: Rfc3339String,
}

#[cw_serde]
pub struct IdemFlag {
    pub generated: bool,
    pub updated_ts: Rfc3339String,
}

// --------------------- マッピング ---------------------
// すべて process_id を先頭キーに含むタプルキー

pub const OWNER_KFRAGS: Map<(String, String), OwnerKFragData> =
    Map::new("owner_kfrags");

pub const OWNER_CAPSULES: Map<(String, String, String), OwnerCapsuleData> =
    Map::new("owner_capsules");

pub const HOLDER_CFRAGS: Map<(String, String, String), HolderCFragData> =
    Map::new("holder_cfrags");

pub const INDEX_KFRAG_TO_CAPS: Map<(String, String, String), IndexKFragToCapsValue> =
    Map::new("index_kfrag_to_caps");

pub const INDEX_CAPS_TO_CFRAG: Map<(String, String, String), IndexCapsToCFragValue> =
    Map::new("index_caps_to_cfrag");

pub const IDEM_FLAGS: Map<(String, String, String), IdemFlag> =
    Map::new("idem_flags");
```

---

# 13. インデックスの機能詳細

> 目的：**列挙の高速化**（`kFrag→Capsule`）と、**単件存在判定のO(1)**（`Capsule→cFrag`）。
> 「順序固定」により、**一括更新**で一貫性を維持しやすい。

### 13.1 書き込み時（`handle_submit_Capsule(K,C)` 成功パス）

1. **本体保存（Capsule）**

   * `OWNER_CAPSULES(pid,K,C) = {capsule, status=RECEIVED, meta}`

2. **列挙用インデックス追加（kFrag→Caps）**

   * `INDEX_KFRAG_TO_CAPS(pid,K,C) = {status=RECEIVED, updated_ts}`
   * 効果：`prefix = (pid,K,*)` のレンジ走査で **CapsuleID が辞書順に列挙**可能

3. **状態遷移**

   * `OWNER_CAPSULES.status = REENC_IN_PROGRESS`

4. **cFrag生成 → 本体保存**

   * `HOLDER_CFRAGS(pid,K,C) = {cFrag, meta}`

5. **存在インデックス追加（Capsule→cFrag）**

   * `INDEX_CAPS_TO_CFRAG(pid,K,C) = {exists=true, updated_ts}`
   * 効果：**O(1)** の `get` で「cFrag存在」可否を即判定

6. **冪等ON**

   * `IDEM_FLAGS(pid,K,C) = {generated=true, updated_ts}`
   * 効果：同一 `(K,C)` の再送は**完全No-Op**

7. **最終状態更新**

   * `OWNER_CAPSULES.status = CFRAG_READY`
   * 望ましければ `INDEX_KFRAG_TO_CAPS(pid,K,C).status` も `CFRAG_READY` に更新
     （一覧で本体を見ずに status を返せる）

> **順序固定**により、**“Capsuleだけ存在”** や **“indexだけ先に立つ”** といった**中間不整合**を構造的に排除。

---

### 13.2 読み取り時：一覧（`handle_list_Capsule_by_kFrag(K, start_after, limit)`）

1. **prefix決定**

   * `P = (pid,K,*)` に相当（実際のキーは `(process_id, kfrag_id, capsule_id)`）

2. **開始位置**

   * `start_after` があれば、`(pid,K,start_after)` の **直後**から
   * 無ければ `(pid,K,"")` 相当から

3. **レンジ走査**

   * `INDEX_KFRAG_TO_CAPS` から **辞書順で最大 `limit` 件** の `(K,C)` を取得
   * 値の `status, updated_ts` をそのまま一覧に載せる
   * 詳細が必要なら、その `C` に対して `OWNER_CAPSULES(pid,K,C)` を `get` して補完

4. **ページング**

   * 最後に読んだ `C` を `next_start_after` に入れて返却

---

### 13.3 読み取り時：単件取得（`handle_get_cFrag(K,C)`）

1. **存在判定（O(1)）**

   * `INDEX_CAPS_TO_CFRAG(pid,K,C)` を `get`
   * 無ければ `ERR_CFRAG_NOT_READY`（または `ERR_NOT_FOUND`）

2. **本体取得**

   * `HOLDER_CFRAGS(pid,K,C)` を `get`（分割時は `CHUNK_META/CHUNK_PARTS` で結合）

3. **監査**

   * `AUDIT_LOGS` に `GET_CFRAG` を追記（`ext_ts`, `actor`, `size`, `hash` 等）

---

### 13.4 削除・再生成（運用時の一貫性）

* **削除**

  * `OWNER_CAPSULES(pid,K,C)` を削除
  * 対応する `INDEX_KFRAG_TO_CAPS(pid,K,C)` を削除
  * cFragも削除するなら `INDEX_CAPS_TO_CFRAG(pid,K,C)` と `HOLDER_CFRAGS(pid,K,C)` を削除
  * 冪等リセットしたい場合は `IDEM_FLAGS(pid,K,C)` を削除

* **再生成（Reencrypt）**

  * `IDEM_FLAGS(pid,K,C)` が **存在** → 完全No-Op
  * **不存在** → 13.1 の 4〜7 を再実行（cFrag生成→index→idem→status）

> 書き込みは **「本体→index→（cFrag本体）→index→idem→status」** の一貫した順序で更新し、
> 読み取りは **「index→（必要時のみ本体）」** の最短I/Oで処理する、が原則。

---

# 14. SubMsg実装仕様詳細（Owner→Holder プロセス間通信）

## 14.1 概要

D-TPRESにおけるOwner→Holder間のプロセス間通信は、CosmWasm標準の`SubMsg`を使用して実現します。これにより、単一プロセス内でのローカル処理ではなく、真のプロセス間分離が実現されます。

## 14.2 必要なインポート

### handlers.rs への追加
```rust
use cosmwasm_std::{
    // 既存のインポート...
    CosmosMsg, SubMsg, WasmMsg, Reply, ReplyOn
};
```

### Reply ID定数定義
```rust
// handlers.rs の上部に追加
const REPLY_DELEGATE_KFRAG: u64 = 1;
const REPLY_DELEGATE_CAPSULE: u64 = 2;
```

## 14.3 実装パターン

### handle_delegate_kfrag の実装パターン
```rust
pub fn handle_delegate_kfrag(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    kfrag: Binary,
) -> ContractResult {
    // 1. バリデーションと設定（既存コード維持）
    let config = CONFIG.load(deps.storage)?;
    let holder_process_id = DEFAULT_HOLDER_PROCESS_ID.to_string();

    // 2. KFRAG_HOLDERSマッピング保存（既存コード維持）
    let holder_key = (config.process_id.clone(), kfrag_id.clone());
    KFRAG_HOLDERS.save(deps.storage, holder_key, &holder_process_id)?;

    // 3. ローカル処理を削除し、SubMsgに置換
    // 削除: let created = persist_kfrag(...);

    // 4. SubMsg作成
    let wasm_msg = WasmMsg::Execute {
        contract_addr: holder_process_id.clone(),
        msg: to_json_binary(&ExecuteMsg::SubmitKFrag {
            kfrag_id: kfrag_id.clone(),
            kfrag: kfrag.clone(),
        })?,
        funds: vec![],
    };

    let sub_msg = SubMsg {
        id: REPLY_DELEGATE_KFRAG,
        msg: CosmosMsg::Wasm(wasm_msg),
        gas_limit: None,
        reply_on: ReplyOn::Error, // エラー時のみReply
    };

    // 5. SubMsgを含むレスポンス返却
    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "delegate_kfrag")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("holder_process_id", holder_process_id))
}
```

### handle_delegate_capsule の実装パターン
```rust
pub fn handle_delegate_capsule(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> ContractResult {
    // 1. Holderマッピング取得（既存コード維持）
    let config = CONFIG.load(deps.storage)?;
    let holder_process_id = KFRAG_HOLDERS
        .may_load(deps.storage, (config.process_id.clone(), kfrag_id.clone()))?
        .ok_or(ContractError::KFragNotFound { kfrag_id: kfrag_id.clone() })?;

    // 2. ローカル処理を削除し、SubMsgに置換
    // 削除: let status = process_capsule_submission(...);

    // 3. SubMsg作成
    let wasm_msg = WasmMsg::Execute {
        contract_addr: holder_process_id.clone(),
        msg: to_json_binary(&ExecuteMsg::SubmitCapsule {
            kfrag_id: kfrag_id.clone(),
            capsule_id: capsule_id.clone(),
            capsule: capsule.clone(),
        })?,
        funds: vec![],
    };

    let sub_msg = SubMsg {
        id: REPLY_DELEGATE_CAPSULE,
        msg: CosmosMsg::Wasm(wasm_msg),
        gas_limit: None,
        reply_on: ReplyOn::Error,
    };

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "delegate_capsule")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("holder_process_id", holder_process_id))
}
```

## 14.4 Reply処理（オプション）

### handlers.rs への Reply処理追加
```rust
pub fn handle_reply(deps: DepsMut, _env: Env, msg: Reply) -> ContractResult {
    match msg.id {
        REPLY_DELEGATE_KFRAG => handle_delegate_kfrag_reply(deps, msg),
        REPLY_DELEGATE_CAPSULE => handle_delegate_capsule_reply(deps, msg),
        _ => Err(ContractError::BadRequest {
            msg: format!("Unknown reply id: {}", msg.id),
        }),
    }
}

fn handle_delegate_kfrag_reply(_deps: DepsMut, msg: Reply) -> ContractResult {
    match msg.result {
        cosmwasm_std::SubMsgResult::Err(err) => {
            Ok(Response::new()
                .add_attribute("action", "delegate_kfrag_error")
                .add_attribute("error", err))
        }
        _ => Ok(Response::new()),
    }
}

fn handle_delegate_capsule_reply(_deps: DepsMut, msg: Reply) -> ContractResult {
    match msg.result {
        cosmwasm_std::SubMsgResult::Err(err) => {
            Ok(Response::new()
                .add_attribute("action", "delegate_capsule_error")
                .add_attribute("error", err))
        }
        _ => Ok(Response::new()),
    }
}
```

### lib.rs への reply エントリーポイント追加
```rust
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    handlers::handle_reply(deps, env, msg)
}
```

## 14.5 テスト実装

### SubMsg生成の確認テスト
```rust
#[test]
fn test_delegate_kfrag_creates_submsg() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");

    let env = mock_env();
    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kfrag1".to_string(),
        kfrag: Binary::from(b"kfrag_data"),
    };

    let response = contract_execute(deps.as_mut(), env, info, msg).unwrap();

    // SubMsg生成確認
    assert_eq!(response.messages.len(), 1);
    let sub_msg = &response.messages[0];
    assert_eq!(sub_msg.id, REPLY_DELEGATE_KFRAG);

    // WasmMsg内容確認
    match &sub_msg.msg {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, .. }) => {
            assert_eq!(contract_addr, DEFAULT_HOLDER_PROCESS_ID);
            let decoded: ExecuteMsg = from_json(msg).unwrap();
            assert!(matches!(decoded, ExecuteMsg::SubmitKFrag { .. }));
        }
        _ => panic!("Expected WasmMsg::Execute"),
    }

    // KFRAG_HOLDERSマッピング確認
    let holder = KFRAG_HOLDERS
        .load(&deps.storage, ("test_process".to_string(), "kfrag1".to_string()))
        .unwrap();
    assert_eq!(holder, DEFAULT_HOLDER_PROCESS_ID);
}
```

## 14.6 実装チェックリスト

```
□ handlers.rs にCosmosMsg, SubMsg, WasmMsg, Reply, ReplyOn インポート追加
□ Reply ID定数定義（REPLY_DELEGATE_KFRAG, REPLY_DELEGATE_CAPSULE）
□ handle_delegate_kfrag からpersist_kfrag呼び出し削除
□ handle_delegate_capsule からprocess_capsule_submission呼び出し削除
□ SubMsg作成・送信コード追加
□ handle_reply関数実装（オプション）
□ lib.rs にreplyエントリーポイント追加（オプション）
□ テストコード更新
□ cargo test実行して全テストパス確認
□ DEFAULT_HOLDER_PROCESS_IDを実際のAO Process ID形式に更新
```

## 14.7 AO Network固有の考慮事項

### Process ID形式
```rust
// state.rs での実際のAO Process ID設定例
pub const DEFAULT_HOLDER_PROCESS_ID: &str = "holder_ABC123XYZ456DEF789GHI012JKL345MNO678PQR";
```

### AOメッセージタグ（将来拡張）
```rust
// 必要に応じてAO固有のタグを属性として追加
.add_attribute("ao_target", holder_process_id)
.add_attribute("ao_action", "delegate")
.add_attribute("ao_timestamp", env.block.time.to_string())
```

## 14.8 段階的実装アプローチ

1. **Phase 1（最小実装）**: SubMsg送信のみ、Replyなし
2. **Phase 2（エラー処理）**: Reply実装追加
3. **Phase 3（完全統合）**: AO Network統合テスト

---

以上が、**SubMsg実装仕様の完全版**です。この仕様に従って実装することで、CosmWasm標準に準拠した安全で効率的なOwner→Holderプロセス間通信が実現されます。
