# 技術スタック

## プロジェクトタイプ

分散型暗号鍵管理クライアントライブラリ。Threshold Proxy Re-Encryption（TPRE）とシャミア秘密分散を組み合わせ、ローカルとオンチェーン（Arweave・AO）のみで動作する分散型鍵管理OSSライブラリ。

## システム構成

### コンポーネント構成

| コンポーネント | ディレクトリ | 実行環境 | 状態 |
|--------------|------------|---------|------|
| **クライアントライブラリ** | `client/` | ローカル（Rust WASM） | Clean Architecture（5層構成）で実装中 |
| **AOコントラクト** | `ao/` | AO Network | 実装完了 |
| **統合SDK** | `dtpres-sdk/` | JavaScript/TypeScript | 設計中 |

### 処理フロー（PRDベース）

```
Phase 1: 秘密の分割と初期配布
O-Browser → 秘密分散 → 暗号化 → Capsule作成 → kFrag分割 → Arweave保存

Phase 2: キーフラグメントの分散管理
Owner-Process → Holder選出 → kFrag配布 → Holder-Process → 再暗号化 → cFrag保存

Phase 3: 秘密の復元
Requester-Process → cFrag収集 → R-Browser → Capsule結合 → 復号 → シャミア補間
```

## コア技術

### 主要言語

- **言語**: Rust 1.86.0（Edition 2024）
- **ターゲット**: wasm32-unknown-unknown
- **言語固有ツール**:
  - Cargo（パッケージマネージャ）
  - rustfmt（フォーマッタ）
  - clippy（リンター）
  - wasm-pack（WASMビルド）

### 主要な依存ライブラリ

| ライブラリ | バージョン | 用途 |
|-----------|-----------|------|
| umbral-pre | 0.11 | 閾値プロキシ再暗号化（Umbral） |
| shamirsecretsharing | 0.1 | シャミア秘密分散（k-of-n） |
| aes-gcm | - | シェア暗号化用対称暗号 |
| zeroize | 1.8 | 秘密情報のメモリクリア |
| subtle | 2.6 | 定数時間暗号操作 |
| serde | 1.0.103 | シリアライゼーション |
| bincode | 1.3 | バイナリエンコーディング |
| thiserror | 1 | 構造化エラー定義 |

## クライアントライブラリ アーキテクチャ（`client/`）

### Clean Architecture（6層構成）

このライブラリを利用する開発者向けに、Clean Architectureに基づく6層構造を採用。依存性逆転原則（DIP）により、Domain層とRepository層が中心となり、Adapter層がRepository Interfaceを実装する。

```
┌─────────────────────────────────┐
│ Actions層（Facade）              │  開発者向けエンドポイント関数
├─────────────────────────────────┤
│ Controller層                     │  バリデーション、UseCase層への適切なデータ変換
├─────────────────────────────────┤
│ UseCase層                        │
│   ├── UseCaseService            │  ユースケース毎のオーケストレーション
│   └── CoreService               │  複数UseCaseから呼び出される共通業務ロジック
├─────────────────────────────────┤
│ Domain層                         │  DDDベースのEntity定義
├─────────────────────────────────┤
│ Repository層                     │  Repository Interface（永続化抽象）
├─────────────────────────────────┤
│ Adapter層                        │  Repository実装（Repository Interfaceを実装）
└─────────────────────────────────┘

依存関係フロー:
  Actions → Controller → UseCase → Domain
                              ↓
                         Repository ← Adapter (implements)
```

### 各層の役割

#### Actions層（`actions/`）
- **目的**: 開発者向けエンドポイント関数の提供（Facadeパターン）
- **責務**: ライブラリ利用者が呼び出すAPI関数を定義し、Controller層経由でUseCaseを呼び出す
- **設計原則**: 外部エンドポイントとして、内部実装の複雑さを隠蔽

#### Controller層（`controller/`）
- **目的**: 入力の検証とUseCase層への適切なデータ変換
- **コンポーネント**:
  - Validator: 入力の妥当性検証
  - ContextExtractor: UseCase層向けDTO変換

#### UseCase層（`usecase/`）
- **UseCaseService**: ユースケース毎のService（オーケストレーション）
  - SecretSharingService: Phase 1処理（CoreServiceを組み合わせて実行）
  - SecretRecoveryService: Phase 3処理
  - 責務: 各CoreServiceを適切な順序で呼び出し、ユースケースを実現
- **CoreService（`usecase/core/`）**: 複数UseCaseから呼び出される共通業務ロジック
  - CryptoService: TPRE・Shamir操作（純粋な暗号ロジック）
  - StorageService: Arweave操作
  - 設計原則: 責務の境界で分離し、再利用可能な単位として構成

#### Phase 2: AOコントラクト処理（`ao/contracts/`）

Phase 2（kFrag配布・再暗号化）はAOコントラクト側で処理される。

**Owner-Process (Pᴼ)**:
- `DelegateKFrag`: kFragをHolder-Processに委譲
- `DelegateCapsule`: CapsuleをHolder-Processに送信し、再暗号化を開始

**Holder-Process (Hⱼ)**:
- `SubmitKFrag`: kFragを受信・保存
- `SubmitCapsule`: Capsuleを受信し、再暗号化を実行
- `perform_reencryption`: `umbral_pre::reencrypt()`でcFragを生成

**データフロー**:
```
Owner → DelegateKFrag → Holder (kFrag保存)
Owner → DelegateCapsule → Holder → perform_reencryption → cFrag保存
Requester ← GetCFrag ← Holder
```

**Holder選出**: 現在は固定値、将来的にRandAOで自動選出予定

#### Domain層（`domain/`）
- **目的**: DDDベースのEntity定義
- **Entities**: 純粋なデータ構造（コンストラクタで不変条件を検証）
  - Secret（集約ルート）, ShareCollection, Capsule, KFrag, CFrag
- **Value Objects**: SecretId, ShareCollectionId, CapsuleId, KFragId, CFragId
- **Errors**: DomainError, DomainResult

#### Repository層（`repositories/`）
- **目的**: 永続化操作を抽象化したtrait（依存性逆転原則）
- **配置**: `domain/`と同じレベル（`src/repositories/`）
- **Repository Interface**:
  - SecretRepository, ShareCollectionRepository, CapsuleRepository, KFragRepository, CFragRepository
- **依存方向**: UseCase → Repository ← Adapter（実装）

#### Adapter層（`adapter/`）
- **目的**: Repository層のRepository Interfaceを実装（DIP）
- **責務**: ArweaveへのGET/POST操作、永続化の詳細を隠蔽
- **構造**:
  - `repository_impl/`: Repository Interface実装
  - `external/`: 外部システムアダプター（ArweaveClient, AOClient）
- **依存方向**: Adapter → Repository（Repository層のInterfaceを実装）

### データストレージ

- **永続ストレージ**: Arweave（不変・公開）
- **保存対象**:
  - Capsule（暗号カプセル）
  - 暗号化シェア（Cᵢ）
  - kFrag/cFrag（キーフラグメント）
- **データフォーマット**: bincode（バイナリ）、JSON（メタデータ）

## AOコントラクト（`ao/`）

### 実装状態
AOネットワーク上で動作するコントラクトロジックは実装完了。

### プロセスロール
- **Owner-Process**: kFrag受信・RandAO選出・kFrag配布
- **Holder-Process**: kFrag保持・再暗号化・cFrag生成
- **Requester-Process**: cFrag収集・ブラウザへ送信

### 外部統合

| 統合先 | 用途 |
|-------|------|
| AO Network | 分散WebAssembly実行環境 |
| Arweave | 永続ストレージ（Capsule、暗号化シェア、WASMモジュール） |
| RandAO | Holder選出用乱数生成 |

## 統合SDK（`dtpres-sdk/`）

### 役割
FORMIXを利用するための**統一エンドポイント**。内部的に`client/`（ローカル暗号処理）から`ao/`（AOプロセス呼び出し）まで一気通貫で処理を行い、開発者は複雑な内部構造を意識せずに秘密の共有・復元を実現できる。

### 設計方針
- **シンプルなAPI**: 4つの関数のみ（`init`, `generateKeyPair`, `share`, `recover`）
- **内部処理の自動化**: Arweave保存・AO通信は全てSDK内部で処理
- **詳細なエラー型**: 問題特定が容易な型付きエラー

### 鍵管理の方針

| 項目 | 方針 |
|------|------|
| 鍵の再利用 | どちらも許容（複数秘密で再利用 or 秘密ごとに新規生成） |
| 鍵管理の責任 | 開発者が完全に管理（SDKは生成のみ） |
| PRE鍵のインポート | なし（FORMIXで常に生成） |
| 秘密のインポート | あり（外部からの秘密 f(0)=secret を共有可能） |
| Owner/Requester鍵 | 同じ種類のPRE鍵ペア（共通API） |

### API関数

| 関数 | 用途 |
|------|------|
| `DTPRES.init()` | SDK初期化、既存プロセスID指定（オプション） |
| `generateKeyPair()` | PRE鍵ペア生成（Owner/Requester共通、umbral-pre型に準拠） |
| `share()` | Phase 1: 秘密分割・Arweave保存・kFrag配布 |
| `recover()` | Phase 3: cFrag収集・復号・秘密復元 |

### 処理フロー例
```typescript
// === 初期化（一度だけ）===
const dtpres = await DTPRES.init({
  ownerProcessId?: string,
  requesterProcessId?: string,
  arweaveWallet?: JWKInterface
});

// === Owner側の準備 ===
// 1. Owner が PRE 鍵ペアを生成（同じAPIを使用）
const ownerKeyPair = await dtpres.generateKeyPair();
// → { secretKey: skₒ, publicKey: pkₒ }
// 開発者がこの鍵ペアを安全に保存・管理する

// === Requester側の準備 ===
// 2. Requester が PRE 鍵ペアを生成（同じAPIを使用）
const requesterKeyPair = await dtpres.generateKeyPair();
// → { secretKey: skᴬ, publicKey: pkᴬ }
// 開発者がこの鍵ペアを安全に保存・管理する

// 3. Requester が pkᴬ を Owner に共有（SDK外のプロセス）

// === Phase 1: 秘密を共有（Owner側）===
const result = await dtpres.share(secret, {
  ownerSecretKey: ownerKeyPair.secretKey,  // Owner の PRE 秘密鍵 skₒ（必須）
  k: 3,                                     // 閾値
  n: 5,                                     // 総Holder数
  requesterPublicKey: requesterKeyPair.publicKey,  // Requester の公開鍵 pkᴬ（必須）
  metadata?: { name, description }
});
// → { secretId, ownerProcessId, ownerPublicKey, capsuleId, holderProcessIds }

// === Phase 3: 秘密を復元（Requester側）===
const { secret } = await dtpres.recover(secretId, {
  requesterSecretKey: requesterKeyPair.secretKey  // Requester の PRE 秘密鍵 skᴬ（必須）
});
```

### 内部処理フロー
```
share() 内部処理:
  1. 引数から Owner の skₒ と Requester の pkᴬ を取得
  2. Shamirで秘密をn個のシェアに分割
  3. Umbralで pkₒ を使ってカプセル作成
  4. skₒ と pkᴬ から kFrag を生成
  5. Arweaveに暗号化データ・カプセル・メタデータを保存
  6. RandAOでHolder選出
  7. AOプロセス経由でkFragをHolderに配布

recover() 内部処理:
  1. 引数から Requester の skᴬ を取得
  2. Requester-Process経由でcFragを収集
  3. Arweaveから暗号化データを取得
  4. cFragを結合し、skᴬ でカプセル復号
  5. Shamirでシェアを補間し秘密を復元
```

## 開発環境

### ビルド・開発ツール

```bash
# クライアントライブラリ (client/)
make check    # コンパイルチェック
make fmt      # コードフォーマット
make clippy   # リント（厳密設定）
make lint     # fmt + clippy
make test     # テスト実行
make all      # check + lint + test

# WASMビルド
wasm-pack build --target web --out-dir wasm
```

### コード品質ツール

- **静的解析**: clippy（カスタムルール設定）
- **フォーマット**: rustfmt（行幅100文字、4スペースインデント）
- **テストフレームワーク**: cargo test
- **ドキュメント**: rustdoc

### バージョン管理

- **VCS**: Git
- **ブランチ戦略**: GitHub Flow
- **ツールチェーン固定**: rust-toolchain.toml（1.86.0）

## 技術要件・制約

### パフォーマンス要件

- **WebAssembly読み込み時間**: < 3秒
- **暗号化処理時間**: < 5秒（1MBファイル）
- **k-of-n再暗号化時間**: < 10秒

### セキュリティ要件

**暗号学的強度**:
- IND-CPA準拠（secp256k1、256-bit）
- k-of-n閾値で共謀耐性（k-1では復元不可）
- Umbral論文のProvable Security証明

**メモリ安全性**:
- Zeroizeによる秘密情報の即時クリア
- 秘密鍵はブラウザメモリ外に漏洩しない
- 使用後即座にzeroize

**定数時間操作**:
- subtleクレートによるタイミング攻撃防止
- 暗号ライブラリは実績のあるumbral-pre使用

### 互換性要件

- **WASMターゲット**: wasm32-unknown-unknown
- **ブラウザ**: WebAssembly対応ブラウザ
- **Rust**: 1.86.0（rust-toolchain.tomlで固定）

## 技術的決定・根拠

### 決定ログ

1. **Rust + WebAssembly選択**:
   - 根拠: メモリ安全性、ゼロコスト抽象化、WASM高性能
   - 用途: クライアントライブラリとAOコントラクト両方で利用

2. **Clean Architecture（6層構成）**:
   - 根拠: テスタビリティ、依存性逆転原則（DIP）、開発者向けAPI設計
   - 対象: クライアントライブラリ（`client/`）のみ
   - 依存方向: Actions → Controller → UseCase → Domain/Repository ← Adapter

3. **umbral-pre採用**:
   - 根拠: 実績のあるProxy Re-Encryption実装、Rustネイティブ
   - 代替案却下: 自前実装（セキュリティリスク）

4. **Arweave永続化**:
   - 根拠: 不変性、永続性、AOネットワークとの親和性
   - 用途: Capsule、暗号化シェア、WASMモジュールの保存

5. **AOコントラクト分離（`ao/`）**:
   - 根拠: クライアントロジックとコントラクトロジックの責務分離
   - 状態: 実装完了

## 既知の制限

- **EVM統合**: アクセス制御条件の検証は外部システム責任（将来的にモジュール統合予定）
- **WASMサイズ**: Arweaveストレージコスト最適化のため最小化を目標
- **乱数生成**: Rust標準乱数（将来的にOsRng、crypto.getRandomValues強化）
- **セキュリティ監査**: 未監査（将来的に形式検証ツール導入予定）
