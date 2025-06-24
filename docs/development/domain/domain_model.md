---
title: "D-TPRES ドメインモデル定義"
version: "1.0.0"
last_updated: "2025-06-01"
author: "D-TPRES Development Team"
status: "draft"
---

# D-TPRES ドメインモデル定義

## 1. 概要

### ドメインモデルの目的

D-TPRES（Deterministic Threshold Proxy Re-Encryption System）のドメインモデルは、分散暗号システムにおける**Threshold Proxy Re-Encryption**を実現するための核心的なデータ構造とビジネスルールを定義します。

### 適用範囲

- **Arweave**: 不変ストレージをKVSとして使用した暗号データ管理
- **AO Network**: WebAssembly分散実行環境での独立プロセス管理
- **EVM Smart Contracts**: 決定論的アクセス制御検証
- **Browser**: クライアントサイド暗号化/復号化

### データストレージアーキテクチャ

D-TPRESは**Key-Value Store (KVS)中心設計**を採用します：

- **プライマリキー**: Arweave Transaction ID (tx_id)
- **データアクセス**: tx_idベースの直接アクセス
- **リレーション管理**: アプリケーションレベルでの参照整合性
- **スケーラビリティ**: 各AOプロセスが独立してArweaveにアクセス

### 主要な制約条件

- **暗号学的安全性**: X25519楕円曲線、AES-GCM 128bit、Umbral-PRE準拠
- **分散システム特性**: k-of-n閾値（k≥3, n≥5）、Byzantine fault tolerance
- **決定論的実行**: AO環境での再現可能な計算結果
- **永続性**: Arweaveによる不変ストレージ

### 設計方針


1. **型安全性**: Rustの所有権システムと型システムを最大活用
2. **ゼロコスト抽象化**: 実行時オーバーヘッドの最小化
3. **暗号学的正当性**: 形式的検証可能な暗号プロトコル準拠
4. **拡張性**: プラガブルアーキテクチャによる将来対応

---

## 2. ドメインオブジェクト定義

### 2.1 エンティティ (Entity)

エンティティは**一意の識別子**を持ち、**ライフサイクル**を通じて**同一性**が保たれるオブジェクトです。属性が変更されても同じエンティティとして認識されます。

#### EncryptedDataShare (Entity)

```yaml
description: "Shamir Secret Sharingによる暗号化データの分散シェア"
entity_rationale: "share_idによって一意に識別され、Arweave永続化やプロセス割り当てなどライフサイクルを持つ"
attributes:
  - name: "share_id"
    type: "ShareId (UUID)"
    required: true
    description: "シェアの一意識別子"
    constraints: "UUID v4, グローバル一意性"
  - name: "data_id"
    type: "DataId (UUID)"
    required: true
    description: "元データへの参照"
    constraints: "外部キー制約"
  - name: "threshold_index"
    type: "NonZeroU8"
    required: true
    description: "Shamirスキームにおけるインデックス (1-255)"
    constraints: "1 ≤ index ≤ n, 一意性制約"
  - name: "encrypted_fragment"
    type: "SecretVec<u8>"
    required: true
    description: "暗号化された秘密断片"
    constraints: "非空、zeroize対象"
  - name: "owner_public_key"
    type: "PublicKey (X25519)"
    required: true
    description: "データ所有者の公開鍵"
    constraints: "暗号学的妥当性"
  - name: "created_at"
    type: "SystemTime"
    required: true
    description: "作成タイムスタンプ"
    constraints: "単調増加"
  - name: "arweave_tx_id"
    type: "Option<TxId>"
    required: false
    description: "Arweave永続化トランザクションID"
    constraints: "一度設定後は不変"
relationships:
  - type: "belongs_to"
    target: "OriginalData"
    cardinality: "N:1"
    description: "複数シェアが1つの元データに属する"
  - type: "assigned_to"
    target: "ProcessActor"
    cardinality: "N:M"
    description: "シェアは複数のプロセスに割り当て可能"
```

#### EncryptedDataShare テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| share_id | ShareId (UUID) | ✓ | シェアの一意識別子 | UUID v4, グローバル一意性 |
| data_id | DataId (UUID) | ✓ | 元データへの参照 | 外部キー制約 |
| threshold_index | NonZeroU8 | ✓ | Shamirスキームにおけるインデックス (1-255) | 1 ≤ index ≤ n, 一意性制約 |
| encrypted_fragment | SecretVec<u8> | ✓ | 暗号化された秘密断片 | 非空、zeroize対象 |
| owner_public_key | PublicKey (X25519) | ✓ | データ所有者の公開鍵 | 暗号学的妥当性 |
| created_at | SystemTime | ✓ | 作成タイムスタンプ | 単調増加 |
| arweave_tx_id | Option<TxId> | - | Arweave永続化トランザクションID | 一度設定後は不変 |

**関係性:**
- **belongs_to** OriginalData (N:1) - 複数シェアが1つの元データに属する
- **assigned_to** ProcessActor (N:M) - シェアは複数のプロセスに割り当て可能


#### ReEncryptionCapsule (Entity)

```yaml
description: "Umbral Proxy Re-Encryptionの暗号化カプセル"
entity_rationale: "capsule_idによって一意に識別され、Original→ReEncryptedへの状態遷移ライフサイクルを持つ"
attributes:
  - name: "capsule_id"
    type: "CapsuleId (UUID)"
    required: true
    description: "カプセルの一意識別子"
    constraints: "UUID v4"
  - name: "data_id"
    type: "DataId"
    required: true
    description: "関連するデータID"
    constraints: "外部キー制約"
  - name: "capsule_bytes"
    type: "Vec<u8>"
    required: true
    description: "Umbral Capsuleのバイナリ表現"
    constraints: "Umbral仕様準拠"
  - name: "public_key"
    type: "PublicKey"
    required: true
    description: "暗号化対象の公開鍵"
    constraints: "X25519楕円曲線"
  - name: "validation_hash"
    type: "Blake3Hash"
    required: true
    description: "改竄検証用ハッシュ"
    constraints: "Blake3アルゴリズム"
  - name: "phase"
    type: "CapsulePhase"
    required: true
    description: "カプセルの処理段階"
    constraints: "列挙型制約"
relationships:
  - type: "derives_from"
    target: "EncryptedDataShare"
    cardinality: "1:N"
    description: "1つのカプセルから複数のcFragmentが派生"
```

#### ReEncryptionCapsule テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| capsule_id | CapsuleId (UUID) | ✓ | カプセルの一意識別子 | UUID v4 |
| data_id | DataId | ✓ | 関連するデータID | 外部キー制約 |
| capsule_bytes | Vec<u8> | ✓ | Umbral Capsuleのバイナリ表現 | Umbral仕様準拠 |
| public_key | PublicKey | ✓ | 暗号化対象の公開鍵 | X25519楕円曲線 |
| validation_hash | Blake3Hash | ✓ | 改竄検証用ハッシュ | Blake3アルゴリズム |
| phase | CapsulePhase | ✓ | カプセルの処理段階 | 列挙型制約 |

**関係性:**
- **derives_from** EncryptedDataShare (1:N) - 1つのカプセルから複数のcFragmentが派生

#### ProcessActor (Entity)

```yaml
description: "AO分散実行環境における実行プロセス"
entity_rationale: "process_idによって一意に識別され、オンライン状態変更や信頼度スコア更新などライフサイクルを持つ"
attributes:
  - name: "process_id"
    type: "ProcessId (String)"
    required: true
    description: "AOプロセスの一意識別子"
    constraints: "AO仕様準拠のID形式"
  - name: "role"
    type: "ActorRole"
    required: true
    description: "プロセスの役割"
    constraints: "Owner/Holder/Requesterのいずれか"
  - name: "wasm_module_tx"
    type: "TxId"
    required: true
    description: "デプロイ済みWasmモジュールのトランザクションID"
    constraints: "Arweave TxID形式"
  - name: "online_status"
    type: "OnlineStatus"
    required: true
    description: "プロセスのオンライン状態"
    constraints: "列挙型制約"
  - name: "stake_amount"
    type: "Option<u64>"
    required: false
    description: "ステーキング量（Phase2実装予定）"
    constraints: "非負整数"
  - name: "reputation_score"
    type: "f32"
    required: true
    description: "信頼度スコア"
    constraints: "0.0 ≤ score ≤ 1.0"
  - name: "last_heartbeat"
    type: "SystemTime"
    required: true
    description: "最終生存確認時刻"
    constraints: "単調増加"
relationships:
  - type: "manages"
    target: "EncryptedDataShare"
    cardinality: "M:N"
    description: "Holderプロセスは複数のシェアを管理"
  - type: "participates_in"
    target: "ReEncryptionSession"
    cardinality: "M:N"
    description: "プロセスは複数の再暗号化セッションに参加"
```

#### ProcessActor テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| process_id | ProcessId (String) | ✓ | AOプロセスの一意識別子 | AO仕様準拠のID形式 |
| role | ActorRole | ✓ | プロセスの役割 | Owner/Holder/Requesterのいずれか |
| wasm_module_tx | TxId | ✓ | デプロイ済みWasmモジュールのトランザクションID | Arweave TxID形式 |
| online_status | OnlineStatus | ✓ | プロセスのオンライン状態 | 列挙型制約 |
| stake_amount | Option<u64> | - | ステーキング量（Phase2実装予定） | 非負整数 |
| reputation_score | f32 | ✓ | 信頼度スコア | 0.0 ≤ score ≤ 1.0 |
| last_heartbeat | SystemTime | ✓ | 最終生存確認時刻 | 単調増加 |

**関係性:**
- **manages** EncryptedDataShare (M:N) - Holderプロセスは複数のシェアを管理
- **participates_in** ReEncryptionSession (M:N) - プロセスは複数の再暗号化セッションに参加

#### AccessProof (Entity)

```yaml
description: "EVM検証結果とオンチェーン証明"
entity_rationale: "proof_idによって一意に識別され、有効期限管理やアクセス要求との関連付けライフサイクルを持つ"
attributes:
  - name: "proof_id"
    type: "ProofId (UUID)"
    required: true
    description: "証明の一意識別子"
    constraints: "UUID v4"
  - name: "requester_public_key"
    type: "PublicKey"
    required: true
    description: "アクセス要求者の公開鍵"
    constraints: "X25519楕円曲線"
  - name: "evm_block_height"
    type: "u64"
    required: true
    description: "EVM検証ブロック高"
    constraints: "非負整数"
  - name: "evm_transaction_hash"
    type: "H256"
    required: true
    description: "EVM検証トランザクションハッシュ"
    constraints: "32バイトハッシュ"
  - name: "verification_event_data"
    type: "Vec<u8>"
    required: true
    description: "検証イベントのRAWデータ"
    constraints: "非空"
  - name: "elciao_attestation"
    type: "Signature"
    required: true
    description: "Elciaoによるアテステーション署名"
    constraints: "Ed25519署名"
  - name: "validity_period"
    type: "TimePeriod"
    required: true
    description: "証明の有効期間"
    constraints: "開始 < 終了"
relationships:
  - type: "authorizes"
    target: "AccessRequest"
    cardinality: "1:1"
    description: "証明は1つのアクセス要求を認可"
```

#### AccessProof テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| proof_id | ProofId (UUID) | ✓ | 証明の一意識別子 | UUID v4 |
| requester_public_key | PublicKey | ✓ | アクセス要求者の公開鍵 | X25519楕円曲線 |
| evm_block_height | u64 | ✓ | EVM検証ブロック高 | 非負整数 |
| evm_transaction_hash | H256 | ✓ | EVM検証トランザクションハッシュ | 32バイトハッシュ |
| verification_event_data | Vec<u8> | ✓ | 検証イベントのRAWデータ | 非空 |
| elciao_attestation | Signature | ✓ | Elciaoによるアテステーション署名 | Ed25519署名 |
| validity_period | TimePeriod | ✓ | 証明の有効期間 | 開始 < 終了 |

**関係性:**
- **authorizes** AccessRequest (1:1) - 証明は1つのアクセス要求を認可

### 2.2 値オブジェクト (Value Object)

値オブジェクトは**識別子を持たず**、**値の等価性**によって比較されるオブジェクトです。不変 (immutable) であり、属性の組み合わせが同じであれば同じオブジェクトとして扱われます。

#### ShareId (Value Object)

```yaml
description: "暗号化データシェアの一意識別子"
value_object_rationale: "UUID値そのものに意味があり、同じUUID値なら同じShareIdとして扱われる"
attributes:
  - name: "uuid"
    type: "Uuid"
    required: true
    description: "UUID v4形式の識別子"
    constraints: "RFC 4122準拠、グローバル一意性"
invariants:
  - "UUID形式の妥当性検証"
  - "非null制約"
```

#### ShareId テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| uuid | Uuid | ✓ | UUID v4形式の識別子 | RFC 4122準拠、グローバル一意性 |

#### PublicKey (Value Object)

```yaml
description: "X25519楕円曲線公開鍵"
value_object_rationale: "32バイトの鍵データが同じであれば同じ公開鍵として扱われる"
attributes:
  - name: "key_bytes"
    type: "[u8; 32]"
    required: true
    description: "X25519公開鍵のバイト表現"
    constraints: "32バイト固定長、楕円曲線上の有効な点"
invariants:
  - "楕円曲線上の点の妥当性検証"
  - "32バイト長制約"
```

#### PublicKey テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| key_bytes | [u8; 32] | ✓ | X25519公開鍵のバイト表現 | 32バイト固定長、楕円曲線上の有効な点 |

#### Blake3Hash (Value Object)

```yaml
description: "Blake3ハッシュアルゴリズムによるハッシュ値"
value_object_rationale: "32バイトのハッシュ値が同じであれば同じハッシュとして扱われる"
attributes:
  - name: "hash_bytes"
    type: "[u8; 32]"
    required: true
    description: "Blake3ハッシュのバイト表現"
    constraints: "32バイト固定長"
invariants:
  - "32バイト長制約"
  - "非null制約"
```

#### Blake3Hash テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| hash_bytes | [u8; 32] | ✓ | Blake3ハッシュのバイト表現 | 32バイト固定長 |

#### TimePeriod (Value Object)

```yaml
description: "時間範囲を表現する値オブジェクト"
value_object_rationale: "開始時刻と終了時刻の組み合わせが同じであれば同じ期間として扱われる"
attributes:
  - name: "start"
    type: "SystemTime"
    required: true
    description: "期間開始時刻"
    constraints: "UTC時刻"
  - name: "end"
    type: "SystemTime"
    required: true
    description: "期間終了時刻"
    constraints: "UTC時刻、start < end"
invariants:
  - "開始時刻 < 終了時刻"
  - "両方の時刻が過去または未来の有効範囲内"
```

#### TimePeriod テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| start | SystemTime | ✓ | 期間開始時刻 | UTC時刻 |
| end | SystemTime | ✓ | 期間終了時刻 | UTC時刻、start < end |

#### Signature (Value Object)

```yaml
description: "Ed25519デジタル署名"
value_object_rationale: "64バイトの署名データが同じであれば同じ署名として扱われる"
attributes:
  - name: "signature_bytes"
    type: "[u8; 64]"
    required: true
    description: "Ed25519署名のバイト表現"
    constraints: "64バイト固定長"
  - name: "public_key"
    type: "PublicKey"
    required: true
    description: "署名に使用された公開鍵"
    constraints: "Ed25519公開鍵"
invariants:
  - "署名の暗号学的妥当性検証"
  - "64バイト長制約"
```

#### Signature テーブル表現

| 属性名 | 型 | 必須 | 説明 | 制約 |
|--------|----|----|------|------|
| signature_bytes | [u8; 64] | ✓ | Ed25519署名のバイト表現 | 64バイト固定長 |
| public_key | PublicKey | ✓ | 署名に使用された公開鍵 | Ed25519公開鍵 |

---

## 3. ビジネスルール

### ThresholdParameterRule

```yaml
description: "k-of-n閾値パラメータの妥当性検証"
type: "不変条件"
implementation: |
  fn validate_threshold_parameters(k: u8, n: u8) -> Result<()> {
    ensure!(k >= 3, "Threshold k must be at least 3 for security");
    ensure!(n >= 5, "Total shares n must be at least 5 for resilience");
    ensure!(k <= n, "Threshold k cannot exceed total shares n");
    ensure!(n <= 255, "Total shares limited by u8 index range");
    Ok(())
  }
validation: "すべての閾値設定時に実行"
```

### ShareUniquenessRule

```yaml
description: "シェアの一意性保証"
type: "不変条件"
implementation: |
  fn validate_share_uniqueness(
    data_id: &DataId,
    threshold_index: u8,
    existing_shares: &[EncryptedDataShare],
  ) -> Result<()> {
    let duplicate = existing_shares.iter().any(|share| {
      share.data_id == *data_id && share.threshold_index == threshold_index
    });
    ensure!(!duplicate, "Duplicate share detected");
    Ok(())
  }
validation: "シェア作成時に実行"
```

### HolderAvailabilityRule

```yaml
description: "Holderプロセスの可用性検証"
type: "前提条件"
implementation: |
  fn validate_holder_availability(holders: &[ProcessActor]) -> Result<()> {
    let online_count = holders.iter()
      .filter(|h| h.online_status == OnlineStatus::Online)
      .count();
    ensure!(online_count >= MIN_THRESHOLD as usize, 
      "Insufficient online holders");
    Ok(())
  }
validation: "再暗号化開始前に実行"
```

### ReEncryptionIdempotencyRule

```yaml
description: "再暗号化の冪等性保証"
type: "不変条件"
implementation: |
  fn ensure_re_encryption_idempotency(
    original_cfrag: &CipherFragment,
    computed_cfrag: &CipherFragment,
  ) -> Result<()> {
    ensure!(original_cfrag.fragment_bytes == computed_cfrag.fragment_bytes,
      "Re-encryption not idempotent");
    Ok(())
  }
validation: "再暗号化実行後に検証"
```

### TPREPhaseTransitionRule

```yaml
description: "TPRE固有の状態遷移制御"
type: "状態遷移"
implementation: |
  fn validate_phase_transition(
    current_phase: TPREPhase,
    target_phase: TPREPhase,
    context: &TransitionContext,
  ) -> Result<()> {
    match (current_phase, target_phase) {
      (KeyGeneration, SecretSharing) => validate_key_generation_complete(context),
      (SecretSharing, AccessVerification) => validate_shares_distributed(context),
      (AccessVerification, ReEncryption) => validate_access_approved(context),
      (ReEncryption, Reconstruction) => validate_threshold_met(context),
      _ => bail!("Invalid phase transition"),
    }
  }
validation: "状態遷移時に実行"
```

---

## 4. 列挙型定義 (Enum Value Objects)

列挙型は特殊な値オブジェクトとして扱われ、有限の値セットを表現します。同じ列挙値であれば同じオブジェクトとして扱われます。

#### ActorRole (Enum Value Object)

```yaml
description: "プロセスアクターの役割定義"
value_object_rationale: "役割名が同じであれば同じ役割として扱われ、役割による振る舞いが決定される"
values:
  - value: "Owner"
    description: "データ所有者プロセス、秘密鍵管理と再暗号化鍵生成"
    constraints: "1プロセス/データ"
  - value: "Holder"
    description: "kFragment保持者、再暗号化実行"
    constraints: "n個プロセス、k個以上がオンライン必須"
  - value: "Requester"
    description: "アクセス要求プロセス、cFragment収集"
    constraints: "アクセス権限要求者に対応"
```

#### ActorRole 値一覧

| 値 | 説明 | 制約 |
|---|-----|------|
| Owner | データ所有者プロセス、秘密鍵管理と再暗号化鍵生成 | 1プロセス/データ |
| Holder | kFragment保持者、再暗号化実行 | n個プロセス、k個以上がオンライン必須 |
| Requester | アクセス要求プロセス、cFragment収集 | アクセス権限要求者に対応 |

#### OnlineStatus (Enum Value Object)

```yaml
description: "プロセスのオンライン状態"
value_object_rationale: "同じ状態名であれば同じオンライン状態として扱われ、状態による可用性判定が決定される"
values:
  - value: "Online"
    description: "正常動作中、ハートビート正常"
    constraints: "最終ハートビート < 5分"
  - value: "Offline"
    description: "停止状態、応答なし"
    constraints: "最終ハートビート > 15分"
  - value: "Degraded"
    description: "動作中だが性能低下"
    constraints: "エラー率 > 5%"
```

#### OnlineStatus 値一覧

| 値 | 説明 | 制約 |
|---|-----|------|
| Online | 正常動作中、ハートビート正常 | 最終ハートビート < 5分 |
| Offline | 停止状態、応答なし | 最終ハートビート > 15分 |
| Degraded | 動作中だが性能低下 | エラー率 > 5% |

#### CapsulePhase (Enum Value Object)

```yaml
description: "再暗号化カプセルの処理段階"
value_object_rationale: "同じフェーズ名であれば同じ処理段階として扱われ、処理可能な操作が決定される"
values:
  - value: "Original"
    description: "元の暗号化状態"
    constraints: "初期状態"
  - value: "ReEncrypted"
    description: "再暗号化済み状態"
    constraints: "kFragment適用後"
```

#### CapsulePhase 値一覧

| 値 | 説明 | 制約 |
|---|-----|------|
| Original | 元の暗号化状態 | 初期状態 |
| ReEncrypted | 再暗号化済み状態 | kFragment適用後 |

#### TPREPhase (Enum Value Object)

```yaml
description: "Threshold Proxy Re-Encryptionの実行フェーズ"
value_object_rationale: "同じフェーズ名であれば同じ実行段階として扱われ、フェーズ遷移ルールが決定される"
values:
  - value: "KeyGeneration"
    description: "鍵生成フェーズ（Phase 0）"
    constraints: "システム初期化時"
  - value: "SecretSharing"
    description: "秘密分散フェーズ（Phase 1）"
    constraints: "Shamirスキーム適用"
  - value: "AccessVerification"
    description: "アクセス検証フェーズ（Phase 2）"
    constraints: "EVM検証必須"
  - value: "ReEncryption"
    description: "再暗号化フェーズ（Phase 3-4）"
    constraints: "k-of-n Holder参加必須"
  - value: "Reconstruction"
    description: "秘密復元フェーズ（Phase 5）"
    constraints: "k個以上のcFragment必要"
```

#### TPREPhase 値一覧

| 値 | 説明 | 制約 |
|---|-----|------|
| KeyGeneration | 鍵生成フェーズ（Phase 0） | システム初期化時 |
| SecretSharing | 秘密分散フェーズ（Phase 1） | Shamirスキーム適用 |
| AccessVerification | アクセス検証フェーズ（Phase 2） | EVM検証必須 |
| ReEncryption | 再暗号化フェーズ（Phase 3-4） | k-of-n Holder参加必須 |
| Reconstruction | 秘密復元フェーズ（Phase 5） | k個以上のcFragment必要 |

---

## 5. ドメインオブジェクト分類まとめ

### 5.1 設計判断基準

| 判断基準 | Entity | Value Object |
|----------|--------|-------------|
| **識別子** | 一意のIDを持つ | IDを持たない |
| **等価性** | IDによる同一性 | 値による等価性 |
| **可変性** | 可変（ライフサイクル） | 不変 |
| **永続化** | IDで追跡 | 値で再構築 |
| **ビジネス意味** | 独立したビジネス概念 | 概念の属性や制約 |

### 5.2 分類結果

#### エンティティ
- **EncryptedDataShare**: シェアのライフサイクル管理
- **ReEncryptionCapsule**: 暗号化状態の遷移管理
- **ProcessActor**: プロセスの状態変更管理
- **AccessProof**: 証明の有効期限管理

#### 値オブジェクト
- **ShareId, CapsuleId, ProcessId, ProofId**: 識別子値
- **PublicKey**: 暗号学的鍵値
- **Blake3Hash**: ハッシュ値
- **TimePeriod**: 時間範囲値
- **Signature**: 署名値
- **ActorRole, OnlineStatus, CapsulePhase, TPREPhase**: 列挙値

---

## 6. ライフサイクル管理

```yaml
creation:
  - rule: "暗号学的妥当性検証"
    implementation: |
      fn validate_cryptographic_properties(share: &EncryptedDataShare) -> Result<()> {
        ensure!(share.encrypted_fragment.len() >= MIN_FRAGMENT_SIZE);
        ensure!(share.encrypted_fragment.len() <= MAX_FRAGMENT_SIZE);
        let entropy = calculate_entropy(&share.encrypted_fragment);
        ensure!(entropy >= MIN_ENTROPY_THRESHOLD);
        Ok(())
      }
  - rule: "閾値インデックス一意性"
    implementation: |
      fn validate_index_uniqueness(
        data_id: &DataId, 
        index: u8, 
        existing: &[EncryptedDataShare]
      ) -> Result<()> {
        let duplicate = existing.iter().any(|s| 
          s.data_id == *data_id && s.threshold_index == index);
        ensure!(!duplicate, "Duplicate threshold index");
        Ok(())
      }

update:
  - rule: "暗号材料不変性"
    implementation: |
      fn validate_immutable_fields(
        original: &EncryptedDataShare,
        updated: &EncryptedDataShare,
      ) -> Result<()> {
        ensure!(original.share_id == updated.share_id);
        ensure!(original.data_id == updated.data_id);
        ensure!(original.threshold_index == updated.threshold_index);
        ensure!(original.encrypted_fragment == updated.encrypted_fragment);
        Ok(())
      }
  - rule: "メタデータ更新制限"
    implementation: |
      fn validate_metadata_update(
        original: &EncryptedDataShare,
        updated: &EncryptedDataShare,
      ) -> Result<()> {
        // Arweave TxIDの追加のみ許可
        if original.arweave_tx_id.is_none() && updated.arweave_tx_id.is_some() {
          validate_arweave_tx_id(updated.arweave_tx_id.as_ref().unwrap())?;
        } else if original.arweave_tx_id != updated.arweave_tx_id {
          bail!("Arweave TxID cannot be modified once set");
        }
        Ok(())
      }

deletion:
  - rule: "Arweave永続化データ制限"
    implementation: |
      fn validate_deletion_constraints(share: &EncryptedDataShare) -> Result<()> {
        if share.arweave_tx_id.is_some() {
          warn!("Cannot physically delete Arweave-persisted share");
          return Ok(()); // 論理削除として処理
        }
        ensure!(!has_active_operations(share), "Active operations exist");
        ensure!(!has_foreign_references(share), "Foreign references exist");
        Ok(())
      }
  - rule: "参照整合性検証"
    implementation: |
      fn validate_reference_integrity(share: &EncryptedDataShare) -> Result<()> {
        // 他エンティティからの参照確認
        let assignments = find_process_assignments(&share.share_id)?;
        ensure!(assignments.is_empty(), "Process assignments exist");
        
        let active_sessions = find_active_re_encryption_sessions(&share.data_id)?;
        ensure!(active_sessions.is_empty(), "Active re-encryption sessions exist");
        
        Ok(())
      }
```

---

## 7. データ整合性ルール

```yaml
constraints:
  - type: "一意性制約"
    description: "エンティティ識別子の重複防止"
    implementation: |
      CREATE UNIQUE INDEX idx_share_data_threshold 
      ON encrypted_data_shares(data_id, threshold_index);
      
      CREATE UNIQUE INDEX idx_process_share_assignment
      ON process_share_assignments(process_id, share_id);
    validation: "データベース制約による自動検証"

  - type: "参照整合性制約"
    description: "外部キー関係の保証"
    implementation: |
      ALTER TABLE encrypted_data_shares 
      ADD CONSTRAINT fk_data_id 
      FOREIGN KEY (data_id) REFERENCES original_data(data_id) 
      ON DELETE CASCADE ON UPDATE RESTRICT;
      
      ALTER TABLE process_share_assignments
      ADD CONSTRAINT fk_process_id
      FOREIGN KEY (process_id) REFERENCES process_actors(process_id)
      ON DELETE CASCADE;
    validation: "外部キー制約による自動検証"

  - type: "範囲制約"
    description: "数値属性の有効範囲検証"
    implementation: |
      fn validate_threshold_index_range(index: u8, n: u8) -> Result<()> {
        ensure!(index > 0, "Threshold index must be positive");
        ensure!(index <= n, "Threshold index cannot exceed total shares");
        Ok(())
      }
      
      fn validate_reputation_score_range(score: f32) -> Result<()> {
        ensure!(score >= 0.0 && score <= 1.0, "Reputation score out of range");
        Ok(())
      }
    validation: "アプリケーションレベルでの事前検証"

  - type: "暗号学的整合性制約"
    description: "暗号材料の妥当性検証"
    implementation: |
      fn validate_capsule_integrity(capsule: &ReEncryptionCapsule) -> Result<()> {
        let computed_hash = compute_capsule_hash(capsule);
        ensure!(computed_hash == capsule.validation_hash, 
          "Capsule integrity check failed");
        
        // Umbral仕様準拠性検証
        let umbral_capsule = umbral_pre::Capsule::try_from_bytes(&capsule.capsule_bytes)?;
        ensure!(umbral_capsule.verify_correctness(), 
          "Umbral capsule correctness check failed");
        
        Ok(())
      }
    validation: "暗号操作前後での整合性検証"

  - type: "時系列制約"
    description: "タイムスタンプの順序関係保証"
    implementation: |
      fn validate_timestamp_ordering(
        earlier: &SystemTime,
        later: &SystemTime,
      ) -> Result<()> {
        ensure!(later >= earlier, "Timestamp ordering violation");
        Ok(())
      }
      
      fn validate_proof_validity_period(proof: &AccessProof) -> Result<()> {
        let now = SystemTime::now();
        ensure!(now >= proof.validity_period.start, "Proof not yet valid");
        ensure!(now <= proof.validity_period.end, "Proof expired");
        Ok(())
      }
    validation: "時系列イベント処理時の順序検証"
```

---

## 8. 拡張性考慮事項

```yaml
future_considerations:
  - aspect: "暗号アルゴリズム拡張"
    description: "Umbral-PREからFHE（Fully Homomorphic Encryption）への移行対応"
    implementation_guidelines: |
      trait CryptographicBackend {
        type PublicKey;
        type SecretKey;
        type Capsule;
        type Fragment;
        
        fn generate_keypair(&self) -> Result<(Self::PublicKey, Self::SecretKey)>;
        fn re_encrypt(&self, capsule: &Self::Capsule, key: &ReKey) -> Result<Self::Fragment>;
      }
      
      // Phase2でFHEBackend実装予定
      struct FHEBackend;
      impl CryptographicBackend for FHEBackend { /* 実装 */ }

  - aspect: "マルチチェーン対応"
    description: "EVM以外のブロックチェーン（Solana, BTC L2）への拡張"
    implementation_guidelines: |
      trait BlockchainProvider {
        type TransactionHash;
        type EventData;
        
        async fn verify_access_condition(&self, condition: &AccessCondition) -> Result<Self::EventData>;
      }
      
      struct SolanaProvider {
        rpc_client: RpcClient,
        program_id: Pubkey,
      }

  - aspect: "スケーラビリティ強化"
    description: "階層型分散アーキテクチャとシャーディング対応"
    implementation_guidelines: |
      struct ScalabilityManager {
        shard_count: u32,
        replication_factor: u8,
      }
      
      impl ScalabilityManager {
        fn determine_shard(data_id: &DataId) -> u32 {
          // 一貫性ハッシュによるシャード決定
        }
        
        fn select_optimal_holders(&self, candidates: &[ProcessActor]) -> Vec<ProcessId> {
          // 負荷分散とレイテンシ最適化
        }
      }

  - aspect: "バージョニング戦略"
    description: "プロトコル版数管理と後方互換性保証"
    implementation_guidelines: |
      #[derive(Debug, Clone, PartialEq)]
      struct ProtocolVersion {
        major: u16,
        minor: u16,
        patch: u16,
      }
      
      trait VersionedEntity {
        fn protocol_version(&self) -> ProtocolVersion;
        fn migrate_to_version(&mut self, target: ProtocolVersion) -> Result<()>;
      }
      
      // セマンティックバージョニング準拠
      impl ProtocolVersion {
        fn is_compatible(&self, other: &ProtocolVersion) -> bool {
          self.major == other.major && self.minor <= other.minor
        }
      }

  - aspect: "TEE統合"
    description: "Trusted Execution Environment対応による秘匿計算強化"
    implementation_guidelines: |
      #[cfg(feature = "tee")]
      mod tee_integration {
        use sgx_tstd::prelude::*;
        
        pub struct TEESecureContext {
          enclave_id: sgx_enclave_id_t,
        }
        
        impl TEESecureContext {
          pub fn secure_re_encrypt(&self, kfrag: &KeyFragment) -> Result<CipherFragment> {
            // SGX Enclave内での安全な再暗号化
          }
        }
      }
```

---

## 9. 用語集

| 用語 | 定義 |
|------|------|
| **TPRE** | Threshold Proxy Re-Encryption: k-of-n閾値プロキシ再暗号化 |
| **kFragment** | Key Fragment: 再暗号化鍵の閾値分散シェア |
| **cFragment** | Cipher Fragment: 再暗号化された暗号文断片 |
| **Capsule** | Umbral-PREにおける暗号化状態を表現するデータ構造 |
| **Shamir Secret Sharing** | k-of-n閾値秘密分散スキーム |
| **AO** | Arweave分散コンピューティングネットワーク |
| **Elciao** | EVMイベントをArweaveに転送するブリッジシステム |

---

## 10. 変更履歴

| バージョン | 日付 | 変更内容 | 担当者 |
|-----------|------|----------|--------|
| 1.0.0 | 2025-06-01 | 初版作成、全セクション定義 | Shodai Momiyama |

---

## 11. 参考文献

- [Umbral Proxy Re-Encryption Specification](https://github.com/nucypher/umbral-pre)
- [Shamir's Secret Sharing](https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing)
- [AO Network Documentation](https://ao.arweave.dev/)
- [Arweave Developer Documentation](https://docs.arweave.org/)
- [WebAssembly System Interface (WASI)](https://wasi.dev/)
