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

- **Arweave**: 不変ストレージ上での暗号データ管理
- **AO Network**: WebAssembly分散実行環境でのプロセス管理
- **EVM Smart Contracts**: 決定論的アクセス制御検証
- **Browser**: クライアントサイド暗号化/復号化

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

## 2. エンティティ定義

### EncryptedDataShare

```yaml
description: "Shamir Secret Sharingによる暗号化データの分散シェア"
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

### ReEncryptionCapsule

```yaml
description: "Umbral Proxy Re-Encryptionの暗号化カプセル"
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

### ProcessActor

```yaml
description: "AO分散実行環境における実行プロセス"
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

### AccessProof

```yaml
description: "EVM検証結果とオンチェーン証明"
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

## 4. 列挙型定義

### ActorRole

```yaml
description: "プロセスアクターの役割定義"
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

### OnlineStatus

```yaml
description: "プロセスのオンライン状態"
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

### CapsulePhase

```yaml
description: "再暗号化カプセルの処理段階"
values:
  - value: "Original"
    description: "元の暗号化状態"
    constraints: "初期状態"
  - value: "ReEncrypted"
    description: "再暗号化済み状態"
    constraints: "kFragment適用後"
```

### TPREPhase

```yaml
description: "Threshold Proxy Re-Encryptionの実行フェーズ"
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

---

## 5. ライフサイクル管理

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

## 6. データ整合性ルール

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

## 7. 拡張性考慮事項

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

## 8. 用語集

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

## 9. 変更履歴

| バージョン | 日付 | 変更内容 | 担当者 |
|-----------|------|----------|--------|
| 1.0.0 | 2025-06-01 | 初版作成、全セクション定義 | D-TPRES Development Team |

---

## 10. 参考文献

- [Umbral Proxy Re-Encryption Specification](https://github.com/nucypher/umbral-pre)
- [Shamir's Secret Sharing](https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing)
- [AO Network Documentation](https://ao.arweave.dev/)
- [Arweave Developer Documentation](https://docs.arweave.org/)
- [WebAssembly System Interface (WASI)](https://wasi.dev/)