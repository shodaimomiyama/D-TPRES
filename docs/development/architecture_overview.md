# D-TPRES アーキテクチャ概要

> **目的**: D-TPRES (Deterministic Threshold Proxy Re-Encryption System) の全体設計とPoC実装計画

---

## 1. システム概要

D-TPRESは、Threshold Proxy Re-Encryption (TPRE) を用いた分散型鍵管理システムです。単一リポジトリ構成でのプロトタイプ開発により、早急なPoC実現を目指します。

### 1.1 主要コンポーネント

```mermaid
graph TB
    subgraph "External Systems"
        subgraph "Browser Applications"
            OB[O-Browser<br/>データ所有者UI]
            AB[A-Browser<br/>アクセス者UI]
        end
        
        subgraph "Storage & Blockchain"
            AR[Arweave<br/>Immutable Storage]
            EVM[EVM Networks<br/>Smart Contracts]
        end
    end
    
    subgraph "AO Network (WebAssembly Runtime)"
        subgraph "Application Layer"
            subgraph "UseCase Handlers"
                OH[Owner Handlers<br/>・Initialize-Owner<br/>・Split-Secret<br/>・Generate-ReKey]
                HH[Holder Handlers<br/>・Store-KFrag<br/>・Perform-Reencryption<br/>・Send-CFrag]
                RH[Requester Handlers<br/>・Access-Request<br/>・Collect-CFrag<br/>・Recover-Secret]
            end
            
            subgraph "Controller Components"
                MH[MessageHandler<br/>処理統括]
                MR[MessageRouter<br/>アクション振り分け]
                MV[MessageValidator<br/>妥当性検証]
                MC[MessageContextExtractor<br/>DTO変換]
            end
        end
        
        subgraph "Business Layer"
            subgraph "Workflow Services"
                WS1[AccessWorkflow]
                WS2[RecoveryWorkflow]
                WS3[DistributionWorkflow]
            end
            subgraph "Core Services"
                CS1[CryptoService]
                CS2[ProcessService]
                CS3[StorageService]
                CS4[EVMVerificationService]
            end
        end
        
        subgraph "Domain Layer"
            subgraph "Entities"
                E1[ProcessEntity]
                E2[ShareEntity]
                E3[CapsuleEntity]
                E4[AccessRequestEntity]
                E5[RekeyFragmentEntity]
            end
            subgraph "Repository Interfaces"
                R1[ProcessEntityRepository]
                R2[ShareEntityRepository]
                R3[CapsuleEntityRepository]
                R4[AccessRequestEntityRepository]
            end
        end
        
        subgraph "Infrastructure Layer"
            subgraph "Repository Implementations"
                RI[ArweaveRepositoryImpl<br/>ProcessEntityRepositoryImpl<br/>ShareEntityRepositoryImpl<br/>etc.]
            end
            subgraph "External Adapters"
                EL[elciao Bridge]
                AC[ArweaveClient]
            end
        end
    end
    
    %% Browser to UseCase
    OB --> OH
    AB --> RH
    
    %% UseCase to Controller
    OH --> MH
    HH --> MH
    RH --> MH
    
    %% Controller Internal Flow
    MH --> MR
    MH --> MV
    MH --> MC
    
    %% Controller to Service
    MC --> WS1
    MC --> WS2
    MC --> WS3
    
    %% Workflow to Core Services
    WS1 --> CS1
    WS1 --> CS2
    WS2 --> CS3
    WS3 --> CS4
    
    %% Service to Domain
    CS1 --> E1
    CS2 --> E2
    CS3 --> E3
    CS4 --> E4
    
    %% Service to Repository Interfaces
    CS1 --> R1
    CS2 --> R2
    CS3 --> R3
    CS4 --> R4
    
    %% Repository Interfaces to Implementations (DIP)
    R1 -.-> RI
    R2 -.-> RI
    R3 -.-> RI
    R4 -.-> RI
    
    %% Infrastructure to External Systems
    RI --> AC
    AC --> AR
    CS4 --> EL
    EL --> EVM
    
    %% Inter-process communication within AO
    OH -.->|kFrag配布| HH
    RH -.->|cFrag要求| HH
    
    style "AO Network (WebAssembly Runtime)" fill:#e6f3ff,stroke:#0066cc,stroke-width:3px
    style R1 fill:#f9f,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
    style R2 fill:#f9f,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
    style R3 fill:#f9f,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
    style R4 fill:#f9f,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
```

### 1.2 暗号化フロー

**Phase 0-5の完全な暗号化ワークフロー**:
1. **Phase 0**: プロセス生成・鍵準備
2. **Phase 1**: 秘密分割・公開ストレージ
3. **Phase 2**: アクセス要求・EVM検証
4. **Phase 3**: 再暗号化鍵のkFrag分割
5. **Phase 4**: k-of-n プロキシ再暗号化
6. **Phase 5**: クライアント復号・秘密復元

## 2. アーキテクチャ設計

### 2.1 単一リポジトリ構成

```
D-TPRES/
├── src/                    # AO WebAssembly (Rust)
│   ├── main.rs
│   ├── di.rs
│   ├── processes/          # AO Process実装
│   │   ├── owner.rs
│   │   ├── holder.rs
│   │   └── requester.rs
│   └── crypto/             # 暗号化ユーティリティ
│       ├── umbral.rs
│       └── shamir.rs
├── browser/                # ブラウザフロントエンド
│   ├── packages/
│   │   ├── core/          # 共通ライブラリ
│   │   │   ├── crypto/    # WebCrypto + WASM統合
│   │   │   ├── ao/        # AO通信ライブラリ
│   │   │   └── types/     # 共通型定義
│   │   ├── o-browser/     # データ所有者UI
│   │   └── a-browser/     # アクセス者UI
│   ├── shared/            # 共通コンポーネント
│   └── package.json
├── contracts/             # EVM Smart Contracts
│   ├── src/
│   │   └── VerifyAccess.sol
│   └── package.json
├── wasm/                  # WebAssembly ビルド成果物
│   ├── umbral_wasm.js
│   └── umbral_wasm.wasm
├── scripts/              # ビルド・デプロイスクリプト
└── docs/
```

### 2.2 技術スタック

| Layer | Component | Technology |
|-------|-----------|------------|
| **Browser** | UI Framework | TypeScript + Vite |
| | Cryptography | WebCrypto API + umbral-pre WASM |
| | Wallet | MetaMask (ethers.js) |
| | Storage | IndexedDB (暗号化) |
| **AO** | Runtime | Rust WebAssembly |
| | Cryptography | umbral-pre + shamir-secret-sharing |
| | Communication | AO Message Protocol |
| **Storage** | Persistent | Arweave (Capsule + 暗号化シェア) |
| | Access Control | EVM Smart Contract |
| | Bridge | elciao (EVM ↔ AO) |

## 3. PoC実装計画

### 3.1 最小実装フロー

**目標**: Phase 0-5の基本フローを最小限の機能で実現

#### PoC Phase 1: 基本暗号化 (Week 1-2)
- [ ] umbral-pre WebAssembly ビルド
- [ ] O-Browser: 鍵生成・Capsule作成・Arweave投稿
- [ ] A-Browser: 鍵生成・復号処理
- [ ] 暗号化→復号の単体テスト

#### PoC Phase 2: AO統合 (Week 3-4)
- [ ] Owner-Process: 基本的な再暗号化鍵生成
- [ ] Holder-Process: kFrag保持・プロキシ再暗号化
- [ ] Requester-Process: cFrag収集・バッチ送信
- [ ] AO Network上での動作確認

#### PoC Phase 3: EVM連携 (Week 5-6)
- [ ] VerifyAccess スマートコントラクト
- [ ] elciao連携による ProofPkg 生成
- [ ] E2E フロー統合テスト

### 3.2 検証項目

#### 技術的実現可能性
- ✅ umbral-pre の WebAssembly 変換
- ✅ AO Network での Rust WebAssembly 実行
- ✅ ブラウザ WebCrypto API との統合
- ✅ MetaMask による EVM トランザクション

#### パフォーマンス要件
- WebAssembly 読み込み時間 < 3秒
- 暗号化処理時間 < 5秒 (1MB ファイル)
- k-of-n 再暗号化時間 < 10秒

#### セキュリティ要件
- 秘密鍵のブラウザメモリ外漏洩防止
- TPRE による確率的暗号化の確認
- k-of-n 閾値での正確な秘密復元

## 4. 開発環境・ビルド設定

### 4.1 Makefile 拡張

```makefile
# 既存のRust WebAssembly向け
check:
	cargo check
fmt:
	cargo fmt --all
clippy:
	cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings
test:
	cargo test

# PoC用追加ターゲット
wasm:
	wasm-pack build --target web --out-dir wasm
	
browser-dev:
	cd browser && npm run dev
	
browser-build:
	cd browser && npm run build
	
contracts-compile:
	cd contracts && npm run compile
	
poc-test:
	make test && make wasm && cd browser && npm test
	
poc-integration:
	make poc-test && npm run test:e2e
```

### 4.2 WebAssembly ビルド設定

#### Cargo.toml の拡張
```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# AO Network向け
# ... 既存の依存関係

# WebAssembly向け追加
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = "0.3"
umbral-pre = { version = "0.13", features = ["wasm"] }
getrandom = { version = "0.2", features = ["js"] }

[dependencies.shamir-secret-sharing]
version = "0.1"
features = ["wasm"]
```

#### wasm-pack 設定
```bash
# ブラウザ向けWASMビルド
wasm-pack build --target web --out-dir wasm --features wasm
```

### 4.3 ブラウザ側設定

#### package.json (browser/)
```json
{
  "name": "@dtpres/browser",
  "private": true,
  "workspaces": [
    "packages/*"
  ],
  "scripts": {
    "dev": "vite serve packages/o-browser",
    "build": "vite build packages/o-browser && vite build packages/a-browser",
    "test": "vitest",
    "test:e2e": "playwright test"
  },
  "devDependencies": {
    "vite": "^5.0.0",
    "typescript": "^5.0.0",
    "vitest": "^1.0.0",
    "@playwright/test": "^1.40.0"
  }
}
```

#### TypeScript設定
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "WebWorker"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "strict": true,
    "noEmit": true
  },
  "include": ["packages/**/*"],
  "references": [
    { "path": "./packages/core" },
    { "path": "./packages/o-browser" },
    { "path": "./packages/a-browser" }
  ]
}
```

## 5. 共通コンポーネント設計

### 5.1 暗号化ライブラリ統合

#### browser/packages/core/crypto/umbral.ts
```typescript
import * as wasm from '../../../wasm/umbral_wasm.js';

export class UmbralCrypto {
  static async init() {
    await wasm.default();
  }
  
  static generateKeyPair(): { publicKey: Uint8Array, secretKey: Uint8Array } {
    // WebAssembly バインディング
  }
  
  static encrypt(publicKey: Uint8Array, message: Uint8Array): Capsule {
    // PRE暗号化
  }
  
  static generateReencryptionKey(
    secretKey: Uint8Array, 
    targetPublicKey: Uint8Array
  ): ReencryptionKey {
    // 再暗号化鍵生成
  }
}
```

### 5.2 AO通信ライブラリ

#### browser/packages/core/ao/client.ts
```typescript
export class AOClient {
  async spawnProcess(module: string, init: ProcessInit): Promise<string> {
    // AO Process spawn
  }
  
  async sendMessage(processId: string, data: MessageData): Promise<void> {
    // メッセージ送信
  }
  
  async getResults(processId: string): Promise<MessageResult[]> {
    // 結果取得
  }
}
```

### 5.3 共通型定義

#### browser/packages/core/types/crypto.ts
```typescript
export interface Capsule {
  point_e: Uint8Array;
  point_v: Uint8Array;
  signature: Uint8Array;
}

export interface KeyFragment {
  id: number;
  key: Uint8Array;
  precursor: Uint8Array;
}

export interface CipherFragment {
  fragment_id: number;
  ciphertext: Uint8Array;
  proof: Uint8Array;
}
```

## 6. セキュリティ設計

### 6.1 ブラウザ側セキュリティ

#### 秘密鍵管理
- **生成**: WebCrypto API（ハードウェア支援）
- **保存**: IndexedDB + AES-GCM暗号化
- **使用**: メモリ上でのみ、使用後即座にzeroize
- **転送**: 秘密鍵は一切ネットワーク送信しない

#### Content Security Policy
```html
<meta http-equiv="Content-Security-Policy" 
      content="default-src 'self'; 
               script-src 'self' 'wasm-unsafe-eval';
               connect-src 'self' https://arweave.net https://ao-cu-url.net;
               style-src 'self' 'unsafe-inline';">
```

### 6.2 AO Process セキュリティ

#### 秘密情報の扱い
- kFrag は一時的にメモリ保持、処理後即座にクリア
- ProofPkg 検証によるアクセス制御
- メッセージ署名による認証

## 7. テスト戦略

### 7.1 単体テスト

#### Rust (AO Layer)
```bash
cargo test
```

#### TypeScript (Browser Layer)
```bash
cd browser && npm test
```

### 7.2 統合テスト

#### E2E フロー
1. O-Browser での暗号化・アップロード
2. A-Browser でのアクセス要求・復号
3. AO Process間の連携確認

#### テストシナリオ
- 正常系: k-of-n 閾値での秘密復元
- 異常系: 閾値未満での復元失敗
- セキュリティ: 不正アクセスの拒否

### 7.3 パフォーマンステスト

- WebAssembly 読み込み・初期化時間
- 暗号化・復号処理時間
- ネットワーク通信レイテンシ

## 8. 運用・デプロイ

### 8.1 PoC デプロイ戦略

#### 開発環境
- ローカル AO Network（ao-dev-cli）
- ローカル Ethereum ネットワーク（Hardhat）
- 静的ホスティング（Vite dev server）

#### テスト環境
- AO Testnet
- Ethereum Sepolia
- Vercel/Netlify デプロイ

### 8.2 PoC後の展開

- セキュリティ監査の実施
- パフォーマンス最適化
- ユーザビリティ改善
- 本格運用環境への移行

---

## 次のステップ

1. **PoC Phase 1**: umbral-pre WebAssembly統合
2. **PoC Phase 2**: AO Process実装
3. **PoC Phase 3**: E2E統合テスト

このアーキテクチャ設計に基づいて、段階的にPoCを実装し、D-TPRESの技術的実現可能性を検証します。