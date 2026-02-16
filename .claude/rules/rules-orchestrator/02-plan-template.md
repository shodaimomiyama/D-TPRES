# [計画タイトル]

**タスク:** [ここには、この計画全体で達成しようとしている具体的な目標を簡潔に記述します]

**影響するライフサイクル:** [Process/Secret/Accessライフサイクルのうち、このタスクが影響するライフサイクルと段階を記載]

**対象プロセスロール:** [Owner/Holder/Requesterのうち、このタスクが対象とするロールを記載]

**完了条件:**

* [ここに、タスクが完了したと判断するための具体的な基準を箇条書きでリストします]
* [基準は明確で測定可能であることが望ましいです]
* [例: 暗号アルゴリズムの正確性テストが全て成功する]
* [例: WASM ビルドが正常に完了し、AOネットワークで実行可能]
* [例: 閾値暗号のk-of-n検証が成功する]
* [例: セキュリティ監査チェックリストが完了している]
* [例: PRが正しく作成されている]

**セキュリティ考慮事項:**

* [ ] 秘密鍵が適切に保護されている
* [ ] アクセス制御が正しく実装されている
* [ ] 暗号パラメータが適切に設定されている
* [ ] サイドチャネル攻撃への対策が実装されている
* [ ] AOステートレス環境での秘密情報管理が適切
* [ ] プロセスロール間の権限分離が実装されている

**AOステートレス制約:**

* [ ] メッセージ間でのメモリ非永続性に対応
* [ ] 状態の明示的な保存・復元が実装されている
* [ ] Compute Units分散への対応が完了
* [ ] 同期処理での実装が確認されている

**実行ステップ:**

1. [ステップ1の説明を記述します。具体的かつ実行可能な単位で記述します。]
    * [任意] **Subtask:** [モード名] - [このステップを実行する場合のサブタスクの説明を記述します。]
    * **Status:** - [ ]
2. [ステップ2の説明を記述します。]
    * [任意] **Subtask:** [モード名] - [このステップを実行する場合のサブタスクの説明を記述します。]
    * **Status:** - [ ]
3. [ステップ3の説明...]
    * **Status:** - [ ]
    * *注意: 各ステップには必ず初期状態として `**Status:** - [ ]` を含めてください。ステップ（または関連サブタスク）が完了したら `- [x]` に更新します。*

**サブタスク一覧:**

* [ ] サブタスク 1: [サブタスク1の具体的な説明を記述します。実行ステップで言及したものと対応させます。] (Mode: [モード名])
* [ ] サブタスク 2: [サブタスク2の具体的な説明を記述します。] (Mode: [モード名])
* [ ] サブタスク N: [...] (Mode: [...])
* *注意: ここには上記「実行ステップ」で特定された全てのサブタスクをリストし、個々のステータス（未完了 `- [ ]` or 完了 `- [x]`）を管理します。*

**依存関係:**

* **外部ライブラリ:** [例: umbral-pre, serde]
* **外部サービス:** [例: Arweave, AO Network]
* **内部モジュール:** 
  - UseCase: [例: Owner/Holder/Requester Handlers]
  - Controller: [例: MessageHandler, MessageRouter, MessageValidator]
  - Service: [例: Workflow Services, Core Services]
  - Domain: [例: ProcessEntity, ShareEntity, CapsuleEntity]
  - Infrastructure: [例: ArweaveRepositoryImpl, elcaio Bridge]

---

## FORMIX タスク例

### 例1: Umbral暗号実装タスク

**タスク:** Umbralプロキシ再暗号化の基本実装を完成させる

**影響する暗号フェーズ:** Phase 3, Phase 4

**完了条件:**

* Umbral鍵生成が正しく動作する
* kFrag生成とcFrag生成が成功する
* 再暗号化のテストケースが全て成功する
* メモリリークがないことが確認されている

**セキュリティ考慮事項:**

* [x] 秘密鍵が適切に保護されている
* [x] アクセス制御が正しく実装されている
* [ ] 暗号パラメータが適切に設定されている
* [ ] サイドチャネル攻撃への対策が実装されている

**実行ステップ:**

1. Umbral暗号ライブラリの基本実装を`src/crypto/umbral.rs`に作成
    * **Subtask:** crypto-impl - Umbral基本操作の実装
    * **Status:** - [x]
2. 鍵生成とkFrag生成機能を実装
    * **Subtask:** crypto-impl - 鍵管理機能の実装
    * **Status:** - [ ]
3. 再暗号化テストを作成し実行
    * **Subtask:** rust-test - Umbral暗号テストの実装
    * **Status:** - [ ]

**サブタスク一覧:**

* [x] サブタスク 1: Umbral基本操作の実装（鍵ペア生成、カプセル作成） (Mode: crypto-impl)
* [ ] サブタスク 2: 鍵管理機能の実装（kFrag生成、シリアライズ） (Mode: crypto-impl)
* [ ] サブタスク 3: Umbral暗号テストの実装と実行 (Mode: rust-test)

**依存関係:**

* **外部ライブラリ:** umbral-pre (v0.11.0)
* **外部サービス:** なし
* **内部モジュール:** 
  - Core Services: CryptoService
  - Domain Entities: ProcessEntity, ShareEntity

### 例2: Owner-Process ハンドラー実装

**タスク:** Owner-Process (P^O) のUseCase Handlerとワークフロー統合を実装

**影響するライフサイクル:** Process Lifecycle (初期化 → アクティブ), Secret Lifecycle (作成 → 分割 → 配布)

**完了条件:**

* UseCase OwnerHandlerでのAOメッセージ受信が正しく動作する
* Controller層経由でWorkflow Servicesとの連携が成功する
* Repository経由でのArweave永続化が確認される
* AOステートレス環境での状態復元が正常に動作する
* WASMビルドが成功し、AOで実行可能

**セキュリティ考慮事項:**

* [x] プロセスロール認証が実装されている
* [ ] メッセージ間での秘密情報クリアが実装されている
* [ ] AOステートレス環境での状態永続化が安全
* [ ] プロセス間の権限分離が適切

**実行ステップ:**

1. UseCase OwnerHandlerの実装
    * **Subtask:** ao-process - AOメッセージ受信とルーティング
    * **Status:** - [ ]
2. Controller Componentsとの統合
    * **Subtask:** ao-process - MessageHandler、MessageRouter連携
    * **Status:** - [ ]
3. Workflow Services統合
    * **Subtask:** crypto-impl - DistributionWorkflowとの連携
    * **Status:** - [ ]
4. Repository永続化実装
    * **Subtask:** ao-process - ArweaveRepositoryImplでの状態保存
    * **Status:** - [ ]
5. WASMビルドとAOデプロイテスト
    * **Subtask:** wasm-build - AOネットワークでの動作確認
    * **Status:** - [ ]

**サブタスク一覧:**

* [ ] サブタスク 1: UseCase OwnerHandlerの基本実装 (Mode: ao-process)
* [ ] サブタスク 2: Controller Components統合 (Mode: ao-process)
* [ ] サブタスク 3: DistributionWorkflow統合 (Mode: crypto-impl)
* [ ] サブタスク 4: Repository永続化実装 (Mode: ao-process)
* [ ] サブタスク 5: WASMビルドとAOネットワーク検証 (Mode: wasm-build)

**依存関係:**

* **外部ライブラリ:** serde, umbral-pre
* **外部サービス:** AO Network, Arweave
* **内部モジュール:** 
  - UseCase: Owner Handlers
  - Controller: MessageHandler, MessageRouter, MessageValidator
  - Service: DistributionWorkflow, CryptoService, ProcessService
  - Domain: ProcessEntity, ShareEntity
  - Infrastructure: ArweaveRepositoryImpl