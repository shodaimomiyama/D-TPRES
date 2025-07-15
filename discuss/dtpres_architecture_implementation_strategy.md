# D-TPRES アーキテクチャ・実装配置戦略（議論用）

> **目的**: D-TPRESシステム全体の設計アーキテクチャとブラウザ側実装の最適な配置戦略について議論・決定するための文書

---

## 1. 現状分析

### 1.1 既存実装状況

#### Rustコードベース（`src/`）
```
src/
├── main.rs     # AO WebAssembly エントリーポイント
└── di.rs       # 依存性注入（プレースホルダー）
```

- **対象環境**: AO Network（WebAssembly実行環境）
- **役割**: Owner-Process、Holder-Process、Requester-Process の統合ロジック
- **暗号化**: umbral-pre ライブラリを使用した TPRE 実装
- **現状**: 初期段階、プレースホルダー実装

#### 文書化された要件
- **O-Browser**: データ所有者フロントエンド（WebCrypto API、Shamir分散）
- **A-Browser**: アクセス者フロントエンド（EVM連携、復号処理）
- **技術スタック**: TypeScript、WebCrypto、MetaMask、umbral-pre（Wasm）

### 1.2 クリーンアーキテクチャに基づく依存関係

```mermaid
graph TB
    subgraph "Entities (Enterprise Business Rules)"
        E[ProcessEntity<br/>ShareEntity<br/>CapsuleEntity<br/>AccessRequestEntity<br/>RekeyFragmentEntity]
    end
    
    subgraph "Use Cases (Application Business Rules)"
        UC[UseCase Handlers<br/>・Owner Handlers<br/>・Holder Handlers<br/>・Requester Handlers]
        WF[Workflow Services<br/>・SecretSharingWorkflow<br/>・AccessRequestWorkflow<br/>・ReencryptionWorkflow]
    end
    
    subgraph "Interface Adapters"
        CTRL[Controllers<br/>・MessageHandler<br/>・MessageRouter<br/>・MessageValidator]
        REPO[Repository Interfaces<br/>・ProcessEntityRepository<br/>・ShareEntityRepository<br/>・CapsuleEntityRepository]
        PRES[Presenters<br/>・Response Formatters<br/>・DTO Converters]
    end
    
    subgraph "Frameworks & Drivers"
        WEB[Web<br/>・Browser UI<br/>・O-Browser<br/>・A-Browser]
        DB[External Storage<br/>・Arweave<br/>・IndexedDB]
        DEV[Devices & External Services<br/>・AO Network<br/>・EVM<br/>・elciao Bridge]
        IMPL[Repository Implementations<br/>・ArweaveRepositoryImpl<br/>・ProcessEntityRepositoryImpl]
    end
    
    %% 依存方向（外側から内側へ）
    WEB --> CTRL
    DEV --> CTRL
    CTRL --> UC
    UC --> E
    UC --> WF
    WF --> E
    CTRL --> REPO
    REPO --> E
    IMPL --> REPO
    IMPL --> DB
    CTRL --> PRES
    
    %% スタイル
    style E fill:#ffd700,stroke:#333,stroke-width:2px
    style UC fill:#90ee90,stroke:#333,stroke-width:2px
    style WF fill:#90ee90,stroke:#333,stroke-width:2px
    style CTRL fill:#87ceeb,stroke:#333,stroke-width:2px
    style REPO fill:#87ceeb,stroke:#333,stroke-width:2px
    style PRES fill:#87ceeb,stroke:#333,stroke-width:2px
    style WEB fill:#dda0dd,stroke:#333,stroke-width:2px
    style DB fill:#dda0dd,stroke:#333,stroke-width:2px
    style DEV fill:#dda0dd,stroke:#333,stroke-width:2px
    style IMPL fill:#dda0dd,stroke:#333,stroke-width:2px
```

## 2. システム全体アーキテクチャ

### 2.1 クリーンアーキテクチャのレイヤー構成

#### Entities Layer (Enterprise Business Rules)
- **責務**: ビジネスルールとデータ構造の定義
- **コンポーネント**: ProcessEntity, ShareEntity, CapsuleEntity, AccessRequestEntity, RekeyFragmentEntity
- **特徴**: 外部依存なし、純粋なビジネスロジック

#### Use Cases Layer (Application Business Rules) 
- **責務**: アプリケーション固有のビジネスルール実装
- **コンポーネント**: UseCase Handlers (Owner/Holder/Requester), Workflow Services
- **特徴**: Entitiesのみに依存、フレームワーク非依存

#### Interface Adapters Layer
- **責務**: データ形式の変換、外部とのインターフェース
- **コンポーネント**: Controllers, Repository Interfaces, Presenters, Gateways
- **特徴**: Use Cases/Entitiesに依存、実装詳細を隠蔽

#### Frameworks & Drivers Layer
- **責務**: 外部システムとの具体的な接続実装
- **コンポーネント**: 
  - Web: O-Browser, A-Browser (TypeScript + WebCrypto)
  - DB: Arweave Storage, IndexedDB
  - External: AO Network, EVM, elciao Bridge
  - Implementations: Repository実装クラス
- **特徴**: 最外層、すべての技術的詳細を含む

### 2.2 クリーンアーキテクチャに基づくデータフロー

```mermaid
sequenceDiagram
    participant FD as Frameworks & Drivers
    participant IA as Interface Adapters
    participant UC as Use Cases
    participant E as Entities
    
    Note over FD: O-Browser, A-Browser<br/>AO Network, Arweave, EVM
    Note over IA: Controllers, Presenters<br/>Repository Interfaces
    Note over UC: UseCase Handlers<br/>Workflow Services
    Note over E: Domain Entities<br/>Business Rules
    
    rect rgb(255, 240, 245)
        Note left of FD: Phase 1: Secret Splitting
        FD->>IA: O-Browser: Split Secret Request
        IA->>UC: MessageHandler: Process Request
        UC->>E: Create ShareEntity, CapsuleEntity
        E-->>UC: Entities Created
        UC->>IA: Repository: Save Entities
        IA->>FD: ArweaveImpl: Store to Arweave
    end
    
    rect rgb(240, 255, 240)
        Note left of FD: Phase 2-3: Access Request & ReKey
        FD->>IA: A-Browser: Access Request
        IA->>UC: AccessRequestWorkflow
        UC->>E: Create AccessRequestEntity
        UC->>IA: EVMVerificationService
        IA->>FD: EVM: Verify Conditions
        FD-->>IA: Verification Result
        IA->>UC: Generate ReKey
        UC->>E: Create RekeyFragmentEntity
    end
    
    rect rgb(240, 240, 255)
        Note left of FD: Phase 4-5: Recovery
        FD->>IA: Holder: Perform Reencryption
        IA->>UC: ReencryptionWorkflow
        UC->>E: Process kFrag/cFrag
        E-->>UC: Cipher Fragments
        UC->>IA: Collect cFrags
        IA->>FD: Return to Requester
        FD->>IA: A-Browser: Recover Secret
        IA->>UC: SecretRecoveryWorkflow
        UC->>E: Reconstruct Secret
    end
```

## 3. 実装配置戦略の比較

### 3.1 Option A: 単一リポジトリ統合

#### 構成案
```
D-TPRES/
├── src/              # 既存: AO WebAssembly (Rust)
├── browser/          # 新規: ブラウザフロントエンド
│   ├── packages/
│   │   ├── core/     # 共通暗号化ライブラリ
│   │   ├── o-browser/ # データ所有者UI
│   │   └── a-browser/ # アクセス者UI
│   ├── shared/       # 共通型定義・ユーティリティ
│   └── package.json
├── contracts/        # 新規: EVM Smart Contracts
├── wasm/            # umbral-pre WebAssembly ビルド成果物
└── docs/
```

#### メリット
- ✅ **技術的整合性**: Rust WebAssembly の共有が容易
- ✅ **開発効率**: 共通型定義・ユーティリティの一元管理
- ✅ **CI/CD統合**: 単一パイプラインでのビルド・テスト・デプロイ
- ✅ **バージョン管理**: コンポーネント間の整合性保証
- ✅ **ドキュメント統一**: 既存の詳細仕様書を活用可能

#### デメリット
- ❌ **ビルド複雑化**: Rust + TypeScript の混在
- ❌ **デプロイ分離困難**: ブラウザ側の独立デプロイが難しい
- ❌ **チーム分業**: フロントエンド・バックエンドの開発分離が困難

### 3.2 Option B: 分離リポジトリ構成

#### 構成案
```
D-TPRES/                    # AO WebAssembly
├── src/
└── docs/

D-TPRES-browser/           # ブラウザフロントエンド
├── packages/
│   ├── core/
│   ├── o-browser/
│   └── a-browser/
└── docs/

D-TPRES-contracts/         # EVM Smart Contracts
├── contracts/
└── docs/
```

#### メリット
- ✅ **開発分離**: 各チームが独立して開発可能
- ✅ **デプロイ独立**: 個別のリリースサイクル
- ✅ **技術スタック特化**: 最適な開発環境の構築
- ✅ **権限管理**: アクセス制御の細分化

#### デメリット
- ❌ **依存関係管理**: WebAssembly共有の複雑化
- ❌ **整合性保証**: バージョン同期の困難
- ❌ **開発効率**: 共通コード・型定義の重複
- ❌ **CI/CD複雑化**: 複数リポジトリ間の統合テスト

## 4. 技術的考慮事項

### 4.1 WebAssembly共有戦略

#### umbral-pre WebAssembly
- **ビルド**: `wasm-pack` でのブラウザ向けビルド
- **配布**: npm パッケージまたはCDN経由
- **サイズ**: WASM + JSバインディングで約500KB-1MB
- **最適化**: `wee_alloc` + 最小化設定

#### 共有方法の選択肢
1. **npm パッケージ**: `@dtpres/umbral-wasm`
2. **Git submodule**: 共通コンポーネントとして管理
3. **CDN配布**: Arweave上での静的ホスティング

### 4.2 開発・ビルド環境

#### 単一リポジトリの場合
```bash
# Rust WebAssembly
make check          # AO向けビルド
make wasm          # ブラウザ向けWASMビルド

# Browser Frontend
cd browser && npm run dev    # 開発サーバー
cd browser && npm run build  # プロダクションビルド

# 統合テスト
make integration-test
```

#### 分離リポジトリの場合
```bash
# 各リポジトリで独立したビルドサイクル
# 依存関係は package.json または Cargo.toml で管理
```

### 4.3 CI/CD パイプライン

#### 単一リポジトリ
- **利点**: 統合テスト・デプロイの自動化
- **課題**: ビルド時間の増大・複雑な設定

#### 分離リポジトリ
- **利点**: 各コンポーネントの独立したパイプライン
- **課題**: クロスリポジトリテストの複雑化

## 5. 議論ポイント

### 5.1 優先度の高い決定事項

#### A. リポジトリ構成の選択
- [ ] **単一リポジトリ**: 技術的整合性を優先
- [ ] **分離リポジトリ**: 開発・デプロイの独立性を優先

**判断基準**:
- 開発チーム構成（フルスタック vs 専門特化）
- デプロイ要件（統合 vs 独立）
- 技術的依存関係の複雑度

#### B. WebAssembly配布戦略
- [ ] **npm パッケージ**: 標準的なフロントエンド配布
- [ ] **Git submodule**: ソースコード直接共有
- [ ] **CDN/Arweave**: 分散型配布

#### C. ブラウザ側技術スタック
- **必須**: TypeScript + WebCrypto API + MetaMask
- **検討**: フレームワーク（React/Vue/Vanilla）
- **ビルドツール**: Vite + esbuild（高速ビルド）

### 5.2 運用面での考慮事項

#### セキュリティ
- **秘密鍵管理**: IndexedDB暗号化保存 + zeroize
- **依存関係**: WebAssembly + 暗号化ライブラリの検証
- **CORS設定**: AO・Arweave との通信

#### パフォーマンス
- **WebAssembly読み込み**: 遅延読み込み + キャッシュ戦略
- **ブラウザ互換性**: Safari WebCrypto API の制限対応
- **メモリ管理**: 大きなファイルの分割処理

#### 開発体制
- **コードレビュー**: Rust + TypeScript の専門知識
- **テスト戦略**: 単体・統合・E2Eテストの分担
- **ドキュメント**: 各コンポーネントの仕様書メンテナンス

## 6. 推奨案

### 6.1 段階的実装アプローチ

#### Phase 1: 単一リポジトリでプロトタイプ
- 技術的実現可能性の検証
- WebAssembly統合の確立
- 基本的な暗号化フローの実装

#### Phase 2: 構成の最適化
- Phase 1の結果を基に最終構成を決定
- 必要に応じてリポジトリ分離を実施
- CI/CD パイプラインの構築

#### Phase 3: 本格運用
- セキュリティ監査・最適化
- ドキュメント・テストの充実
- 運用・保守体制の確立

### 6.2 初期推奨構成

**単一リポジトリ + 段階的分離**を推奨：

1. **開発初期**: 単一リポジトリで技術的整合性を確保
2. **成熟期**: 必要に応じて専門特化リポジトリに分離
3. **運用期**: 各コンポーネントの独立した運用体制

---

## 決定事項（議論後に記入）

- [ ] リポジトリ構成の最終決定
- [ ] WebAssembly配布方法の選択
- [ ] ブラウザ側フレームワークの決定
- [ ] CI/CD パイプライン設計の承認
- [ ] 開発・運用体制の確立

---

**次のステップ**: この議論の結果を `docs/development/architecture_overview.md` に正式文書として作成