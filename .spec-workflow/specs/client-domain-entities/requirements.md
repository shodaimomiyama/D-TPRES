# Requirements Document: Client Domain Entities

## Introduction

D-TPRESクライアントライブラリのDomain層におけるエンティティ群とValue Objectsを実装する。これらは、閾値プロキシ再暗号化（Umbral）とシャミア秘密分散を組み合わせた分散型鍵管理システムのコアドメインモデルを形成する。

DDDの原則に従い：
- **Entity**: 一意のIDを持ち、ライフサイクルを通じて同一性を維持。永続化対象。
- **Value Object**: IDを持たず、値で等価性を判断。不変。

**重要な設計方針：**
- Domain Entityは「永続化されるビジネス概念」のみを表現
- 処理の中間状態（平文シェア `f(i)` 等）はService層で扱い、Domain層には含めない
- 暗号化されArweaveに保存されるデータのみがEntityとして定義される
- 一時的な値（鍵、秘密データ）はValue Objectとして定義し、使用後Zeroize

## Alignment with Product Vision

このfeatureは以下のプロダクト目標をサポートする：

1. **分散性優先**: Secret、Share、KFrag、CFragエンティティがk-of-n閾値暗号の基盤を提供
2. **設計によるセキュリティ**: Zeroizeトレイトによるメモリ安全性を保証
3. **検証可能性**: Capsuleエンティティが暗号カプセルのArweave永続化を抽象化
4. **開発者フレンドリー**: 明確なドメインモデルがSDK統合の基盤を提供

## Requirements: Entities

### Requirement 1: Secret Entity（集約ルート）

**User Story:** As a 開発者, I want Secretエンティティで秘密のメタデータを管理したい, so that 秘密の分割・復元状態を追跡できる

#### Design Note

Secret Entityはメタデータのみを保持。秘密の実データ `f(0)=secret` はValue Object `SecretData` として別途定義され、Service層で一時的に処理される。

#### Acceptance Criteria

1. WHEN Secret::new()が呼ばれた THEN システム SHALL SecretIdを生成し、InitializedステートのSecretを返す
2. WHEN 無効なthreshold（k > n または k = 0）が指定された THEN システム SHALL DomainErrorを返す
3. WHEN Secret.split()が呼ばれた THEN システム SHALL ステートをSplitに遷移し、関連するShare IDsを記録する
4. WHEN Secret.capsule_id()が呼ばれた THEN システム SHALL 関連するCapsuleのID（Option）を返す

### Requirement 2: Share Entity（暗号化シェア）

**User Story:** As a 開発者, I want Shareエンティティで暗号化されたシャミアシェアを表現したい, so that Arweaveに永続化されるシェアを管理できる

#### Design Note

PRDのライフサイクルに基づく責務分離：
- **Share Entity (Domain層)**: 暗号化シェア `Cᵢ = AES_GCM(kₒ, f(i))` を表現。Arweaveに永続化。
- **PlaintextShare (Service層)**: 平文シェア `f(i)` を一時的に処理。Zeroize必須、永続化しない。

```
Phase 1: f(i) → [AES暗号化] → Cᵢ → [Arweave保存] → Share Entity
Phase 3: Share Entity → [Arweave取得] → Cᵢ → [AES復号] → f(i) → [シャミア補間]
```

#### Acceptance Criteria

1. WHEN Share::new()が呼ばれた THEN システム SHALL ShareId、SecretId参照、インデックス、暗号化データ（Cᵢ）を持つShareを返す
2. WHEN シェアインデックスが閾値パラメータの範囲外（0またはn超過）THEN システム SHALL DomainErrorを返す
3. WHEN Share.encrypted_data()が呼ばれた THEN システム SHALL 暗号化されたシェアデータ（Cᵢ）への参照を返す
4. WHEN Share.set_arweave_tx_id()が呼ばれた THEN システム SHALL ArweaveトランザクションIDを記録する

### Requirement 3: Capsule Entity（PREカプセル）

**User Story:** As a 開発者, I want CapsuleエンティティでUmbralカプセルを表現したい, so that プロキシ再暗号化に必要なカプセルを管理できる

#### Design Note

PRDにおけるCapsule:
- `Capsuleₒ = PRE_Enc(pkₒ, kₒ)`: 共通鍵kₒをOwner公開鍵で暗号化したカプセル
- Phase 1-9でArweaveに保存され、Phase 2-4でHolder-Processが取得・再暗号化に使用

#### Acceptance Criteria

1. WHEN Capsule::new()が呼ばれた THEN システム SHALL CapsuleId、SecretId参照、シリアライズされたカプセルデータを持つCapsuleを返す
2. WHEN 無効なカプセルデータ（空または不正フォーマット）が渡された THEN システム SHALL DomainErrorを返す
3. WHEN Capsule.arweave_tx_id()が呼ばれた THEN システム SHALL Arweaveトランザクションへの参照（Option）を返す
4. WHEN Capsule.set_arweave_tx_id()が呼ばれた THEN システム SHALL ArweaveトランザクションIDを記録する

### Requirement 4: KFrag Entity（鍵フラグメント）

**User Story:** As a 開発者, I want KFragエンティティで再暗号化鍵フラグメントを表現したい, so that Owner-ProcessからHolder-Processへの鍵配布を管理できる

#### Design Note

PRDにおけるKFrag:
- Phase 1-6: `rekey = PRE_ReKey(skₒ → pkᴬ)` で再暗号化鍵を生成
- Phase 1-7: `kFragⱼ = Shamir_Split(rekey, k, n)` でkFragに分割
- Phase 2-2: 各Holder-Processに配布

#### Acceptance Criteria

1. WHEN KFrag::new()が呼ばれた THEN システム SHALL KFragId、SecretId参照、Holderインデックス、シリアライズされたkFragデータを持つKFragを返す
2. WHEN Holderインデックスが総Holder数を超えた THEN システム SHALL DomainErrorを返す
3. IF KFragがDropされた THEN システム SHALL ZeroizeトレイトによりkFragデータをメモリからクリアする
4. WHEN KFrag.holder_process_id()が呼ばれた THEN システム SHALL 割り当てられたHolder-ProcessのID（Option）を返す

### Requirement 5: CFrag Entity（再暗号化フラグメント）

**User Story:** As a 開発者, I want CFragエンティティで再暗号化されたフラグメントを表現したい, so that Holder-ProcessからRequesterへの再暗号化結果を管理できる

#### Design Note

PRDにおけるCFrag:
- Phase 2-5: `cFragⱼ = PRE_ReEnc(kFragⱼ, Capsuleₒ)` でHolder-Processが生成
- Phase 2-6: ArweaveまたはAOに保存
- Phase 3-2: Requester-Processがk個以上収集

#### Acceptance Criteria

1. WHEN CFrag::new()が呼ばれた THEN システム SHALL CFragId、SecretId参照、Holderインデックス、シリアライズされたcFragデータを持つCFragを返す
2. WHEN 対応するKFragIdが存在しない THEN システム SHALL DomainErrorを返す
3. IF CFragがDropされた THEN システム SHALL ZeroizeトレイトによりcFragデータをメモリからクリアする
4. WHEN CFrag.verify()が呼ばれた THEN システム SHALL カプセルとの整合性を検証しResult<bool>を返す

## Requirements: Value Objects

### Requirement 6: ID Value Objects

**User Story:** As a 開発者, I want 型安全なID Value Objectsを使いたい, so that 異なるエンティティのIDを混同しない

#### Design Note

各EntityのIDはnewtypeパターンでラップし、型安全性を確保する。

#### Acceptance Criteria

1. WHEN SecretId::new()が呼ばれた THEN システム SHALL 一意のIDを生成して返す
2. WHEN 同じ文字列値を持つSecretIdを比較した THEN システム SHALL 等価と判断する
3. WHEN SecretIdをShareIdが期待される場所で使用した THEN システム SHALL コンパイルエラーを発生させる
4. 以下のID Value Objectsを定義する:
   - `SecretId` - Secret Entity用
   - `ShareId` - Share Entity用
   - `CapsuleId` - Capsule Entity用
   - `KFragId` - KFrag Entity用
   - `CFragId` - CFrag Entity用

### Requirement 7: SecretData Value Object

**User Story:** As a 開発者, I want SecretData Value Objectで秘密の実データを扱いたい, so that 秘密を安全に一時処理できる

#### Design Note

PRD Phase 1-1の `f(0)=secret` に対応。SDK利用者から受け取り、シャミア分散後に破棄される。

#### Acceptance Criteria

1. WHEN SecretData::new()が呼ばれた THEN システム SHALL バイト列をラップしたSecretDataを返す
2. WHEN 空のバイト列が渡された THEN システム SHALL DomainErrorを返す
3. IF SecretDataがDropされた THEN システム SHALL Zeroizeによりメモリをクリアする
4. SecretData SHALL Cloneトレイトを実装しない（偶発的コピー防止）

### Requirement 8: KeyPair Value Object

**User Story:** As a 開発者, I want KeyPair Value ObjectでPRE鍵ペアを扱いたい, so that Owner/Requesterの鍵を型安全に管理できる

#### Design Note

PRD Phase 1-1の `skₒ(PRE)`, `pkₒ(PRE)` に対応。SDK利用者が管理し、D-TPRESは生成・使用のみ。

#### Acceptance Criteria

1. WHEN KeyPair::generate()が呼ばれた THEN システム SHALL umbral-preで鍵ペアを生成しKeyPairを返す
2. WHEN KeyPair.public_key()が呼ばれた THEN システム SHALL 公開鍵への参照を返す
3. WHEN KeyPair.secret_key()が呼ばれた THEN システム SHALL 秘密鍵への参照を返す
4. IF KeyPairがDropされた THEN システム SHALL Zeroizeにより秘密鍵をメモリからクリアする
5. KeyPair SHALL Cloneトレイトを実装しない（秘密鍵の偶発的コピー防止）

### Requirement 9: SymmetricKey Value Object

**User Story:** As a 開発者, I want SymmetricKey Value ObjectでAES共通鍵を扱いたい, so that シェア暗号化用の鍵を安全に管理できる

#### Design Note

PRD Phase 1-1の `kₒ` に対応。Capsule生成時に使用され、暗号化後は破棄される。

#### Acceptance Criteria

1. WHEN SymmetricKey::generate()が呼ばれた THEN システム SHALL 256-bit AES鍵を生成しSymmetricKeyを返す
2. WHEN SymmetricKey.as_bytes()が呼ばれた THEN システム SHALL 鍵バイト列への参照を返す
3. IF SymmetricKeyがDropされた THEN システム SHALL Zeroizeによりメモリをクリアする
4. SymmetricKey SHALL Cloneトレイトを実装しない（偶発的コピー防止）

## Non-Functional Requirements

### Code Architecture and Modularity

- **Single Responsibility Principle**: 各エンティティ/Value Objectファイルは1つの型のみを定義
- **Modular Design**: Domain層内で独立し、外部依存を持たない
- **Dependency Management**: serde等のシリアライズ注釈はInfrastructure層で追加（Domain層では純粋）
- **Clear Interfaces**: new()/generate()コンストラクタとgetter/setterで明確なインターフェースを提供
- **Layer Separation**:
  - Domain層: Entity（Secret, Share, Capsule, KFrag, CFrag）+ Value Objects
  - Service層: 処理中間状態（PlaintextShare等）はZeroize付きの内部構造体として定義

### Performance

- **メモリ効率**: 不必要なClone実装を避け、参照ベースのAPIを提供
- **アロケーション最小化**: WASMターゲットを考慮し、ヒープ割り当てを最小限に

### Security

- **Zeroize実装**: 秘密データを含むValue Objects（SecretData, KeyPair, SymmetricKey）とEntities（KFrag, CFrag）はZeroizeを実装
- **Clone禁止**: 秘密データを含む型はCloneを実装しない
- **不変条件検証**: コンストラクタで閾値パラメータ等の妥当性を検証
- **平文の非永続化**: 平文シェア `f(i)` はDomain層に含めず、Service層で一時的に処理後即座にZeroize

### Reliability

- **型安全性**: 強い型付け（newtypeパターン）によりコンパイル時にエラーを検出
- **Result型**: 失敗可能な操作はすべてResult<T, DomainError>を返す

### Usability

- **ドキュメント**: 各公開APIにrustdocコメントを追加
- **Debug実装**: デバッグ用にDebugトレイトを実装（秘密データは隠蔽）
