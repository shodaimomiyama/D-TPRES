# D-TPRES CWAO統合クラス 技術仕様書

## 概要

`d-tpres-cwao.js` は、**D-TPRES (Deterministic Threshold Proxy Re-Encryption System)** と **CWAO SDK (CosmWasm-AO)** を統合するメインクラスです。AOネットワーク上でD-TPRESプロセスのデプロイメント、管理、実行を行う中心的な実装を提供します。

### 主要機能
- AOネットワークへのWASMモジュールデプロイ
- Owner-Process/Holder-Processのインスタンス化
- ローカルkFrag生成とプロセス間転送
- 完全自動化されたワークフロー実行

## アーキテクチャ

### クラス構造

```javascript
D_TPRES_CWAO
├── Logger (ユーティリティ)
│   ├── info()    // ℹ 一般情報
│   ├── success() // ✅ 成功メッセージ
│   ├── warning() // ⚠️ 警告
│   ├── error()   // ❌ エラー
│   ├── crypto()  // 🔐 暗号操作
│   └── network() // 🌐 ネットワーク操作
└── D_TPRES_CWAO (メインクラス)
    ├── 初期化
    ├── デプロイメント
    ├── kFrag管理
    ├── ワークフロー自動化
    └── 状態管理
```

### 内部状態管理

| プロパティ | 型 | 説明 |
|------------|-------|------|
| `config` | Object | プロトコル設定 (ao, ao_0_1) |
| `cwao` | CWAO | CWAO SDKインスタンス |
| `wallet` | Object | Arweaveウォレット |
| `moduleId` | String | デプロイ済みWASMモジュールID |
| `processes.owner` | String | Owner-ProcessのID |
| `processes.holders` | Array | Holder-ProcessのID配列 |
| `initialized` | Boolean | 初期化状態 |

## 主要メソッド

### 🔧 初期化関連

#### `initialize()`
```javascript
async initialize()
```
CWAO統合の初期化を実行します。

**処理フロー:**
1. ウォレットファイルの読み込み
2. CWAO SDKインスタンスの作成
3. AOネットワーク接続の準備

**戻り値:** `Boolean` - 初期化成功

#### `loadWallet()`
```javascript
async loadWallet()
```
JSONウォレットファイルを読み込み、Arweave認証に必要な秘密鍵を取得します。

**設定パス:** `this.config.walletPath` (デフォルト: `./wallet.json`)

### 🚀 デプロイメント関連

#### `deployModule(wasmPath)`
```javascript
async deployModule(wasmPath = null)
```
WASMバイナリをArweaveにアップロードし、AOネットワークで実行可能にします。

**パラメータ:**
- `wasmPath` (String, optional): WASMファイルパス (デフォルト: `./output/d_tpres.wasm`)

**戻り値:** `String` - デプロイされたモジュールID

**処理フロー:**
1. WASMバイナリの読み込み
2. Arweaveへのアップロード
3. モジュールIDの取得・保存

#### `spawnOwnerProcess(scheduler)`
```javascript
async spawnOwnerProcess(scheduler = null)
```
Owner-Processをインスタンス化します。Owner-Processは秘密鍵管理とkFrag生成の責任を持ちます。

**パラメータ:**
- `scheduler` (String, optional): 使用するスケジューラID

**戻り値:** `String` - スポーンされたOwner-ProcessのID

**初期化データ:**
```javascript
{
  process_type: "owner",
  version: "0.1.0-mvp",
  timestamp: Date.now()
}
```

#### `spawnHolderProcesses(count, scheduler)`
```javascript
async spawnHolderProcesses(count = 1, scheduler = null)
```
指定数のHolder-Processをインスタンス化します。Holder-ProcessはkFragの分散保管を担当します。

**パラメータ:**
- `count` (Number): 生成するHolder-Process数 (デフォルト: 1)
- `scheduler` (String, optional): 使用するスケジューラID

**戻り値:** `Array<String>` - スポーンされたHolder-ProcessのID配列

### 🔐 kFrag管理関連

#### `generateKFragsLocal(secret, threshold, totalShares)`
```javascript
async generateKFragsLocal(secret, threshold = 2, totalShares = 3)
```
WASM-pack経由でローカル環境でkFragsを生成します。

**パラメータ:**
- `secret` (String|Uint8Array): 分割する秘密データ
- `threshold` (Number): 復元に必要な最小シェア数 (デフォルト: 2)
- `totalShares` (Number): 生成する総シェア数 (デフォルト: 3)

**戻り値:** `Object` - kFrag生成結果
```javascript
{
  kfrags: Array<KeyFragment>,
  sharesCount: Number,
  threshold: Number,
  capsuleId: String,
  timestamp: Number,
  source: "local-wasm"
}
```

#### `sendKFragsToOwner(kfrags)`
```javascript
async sendKFragsToOwner(kfrags)
```
生成されたkFragsをOwner-Processに送信します。

**パラメータ:**
- `kfrags` (Array): kFragオブジェクトの配列

**CWAOアクション:** `store-kfrags`

#### `transferKFragsToHolders()`
```javascript
async transferKFragsToHolders()
```
Owner-ProcessからHolder-Processesへkfragsを転送し、分散保管を実現します。

**CWAOアクション:** `transfer-kfrags`

### 🔄 ワークフロー自動化

#### `deployFullWorkflow(wasmPath, holderCount, scheduler)`
```javascript
async deployFullWorkflow(wasmPath = null, holderCount = 1, scheduler = null)
```
完全なデプロイメントワークフローを自動実行します。

**実行ステップ:**
1. WASMモジュールのデプロイ
2. Owner-Processのスポーン
3. Holder-Process群のスポーン

**戻り値:** `Object`
```javascript
{
  moduleId: String,
  processes: {
    owner: String,
    holders: Array<String>
  }
}
```

#### `executeKFragWorkflow(secret, threshold, totalShares)`
```javascript
async executeKFragWorkflow(secret, threshold = 2, totalShares = 3)
```
完全なkFrag処理ワークフローを自動実行します。

**実行ステップ:**
1. ローカルでkFrags生成
2. Owner-Processに送信
3. Holder-Processesに分散

**戻り値:** `Object`
```javascript
{
  localResult: Object,
  processesInvolved: {
    owner: String,
    holders: Array<String>
  }
}
```

### 📊 状態管理

#### `queryProcessState(processId)`
```javascript
async queryProcessState(processId = null)
```
指定されたプロセスの現在の状態を照会します。

**パラメータ:**
- `processId` (String, optional): 照会するプロセスID (デフォルト: Owner-Process)

**CWAOアクション:** `get-state`

#### `getDeploymentSummary()`
```javascript
getDeploymentSummary()
```
デプロイメント全体の状況サマリーを取得します。

**戻り値:** `Object`
```javascript
{
  initialized: Boolean,
  moduleId: String,
  processes: {
    owner: String,
    holders: Array<String>,
    holderCount: Number
  },
  wallet: String // "***loaded***" または null
}
```

## 実行フロー

### 基本的な使用パターン

```javascript
import { D_TPRES_CWAO } from './d-tpres-cwao.js';

// 1. インスタンス作成と初期化
const dtpres = new D_TPRES_CWAO({
  walletPath: './wallet.json'
});
await dtpres.initialize();

// 2. フルデプロイメント (3つのHolder-Process)
const deployment = await dtpres.deployFullWorkflow(
  './output/d_tpres.wasm',
  3  // holderCount
);

console.log('Module ID:', deployment.moduleId);
console.log('Owner Process:', deployment.processes.owner);
console.log('Holder Processes:', deployment.processes.holders);

// 3. kFrag処理ワークフロー実行
const workflow = await dtpres.executeKFragWorkflow(
  'my-secret-data',  // secret
  2,                 // threshold
  3                  // totalShares
);

console.log('kFrags generated:', workflow.localResult.kfrags.length);
console.log('Processes involved:', workflow.processesInvolved);

// 4. 状態確認
const state = await dtpres.queryProcessState();
console.log('Process state:', state);
```

### 段階的実行パターン

```javascript
// 段階的にコントロールしたい場合
const dtpres = new D_TPRES_CWAO({ walletPath: './wallet.json' });
await dtpres.initialize();

// ステップ1: モジュールデプロイ
const moduleId = await dtpres.deployModule('./output/d_tpres.wasm');

// ステップ2: プロセススポーン
const ownerProcess = await dtpres.spawnOwnerProcess();
const holderProcesses = await dtpres.spawnHolderProcesses(2);

// ステップ3: kFrag生成と送信
const kfragResult = await dtpres.generateKFragsLocal('secret', 2, 3);
await dtpres.sendKFragsToOwner(kfragResult.kfrags);
await dtpres.transferKFragsToHolders();
```

## 設計特徴

### 🛡️ エラーハンドリング

- **段階的検証**: 各メソッドで前提条件をチェック
- **詳細ログ**: Loggerクラスで各ステップの進捗を可視化
- **グレースフルエラー**: try-catchで適切なエラーメッセージを提供

### 🔄 状態管理

- **内部状態の一貫性**: moduleId、processIdを内部で管理
- **初期化チェック**: `ensureInitialized()`で操作前の状態確認
- **プロセス追跡**: Owner/Holderプロセスを自動的に追跡

### 📈 スケーラビリティ

- **動的プロセス数**: Holder-Process数を実行時に指定可能
- **柔軟な設定**: config objectで各種パラメータを設定
- **モジュール化**: 各機能が独立して実行可能

### 🎯 ユーザビリティ

- **ビジュアルフィードバック**: 絵文字付きログで進捗を直感的に表示
- **ワンライナー実行**: 複雑なワークフローを1行で実行
- **デバッグサポート**: 詳細な状態サマリーとエラー情報

## CWAO SDK統合

### プロトコル設定
```javascript
{
  protocol: "ao",        // AOネットワーク使用
  variant: "ao_0_1",     // AO v0.1バリアント
  wallet: walletObject   // Arweaveウォレット
}
```

### アクション定義

| アクション | 対象プロセス | 説明 |
|------------|-------------|------|
| `store-kfrags` | Owner-Process | kFragsをOwnerに保存 |
| `transfer-kfrags` | Owner-Process | kFragsをHoldersに転送 |
| `get-state` | Any Process | プロセスの状態照会 |

## セキュリティ考慮事項

### ウォレット管理
- ウォレットファイルは適切なパーミッションで保護
- 秘密鍵はメモリ上でのみ処理
- エラーログにウォレット情報を出力しない

### kFrag処理
- ローカル生成によりネットワーク暴露を最小化
- 閾値暗号による分散セキュリティ
- プロセス間通信の暗号化 (CWAO提供)

## 依存関係

### 外部依存
- `cwao`: CWAO SDK
- `chalk`: ログ表示
- `fs/promises`: ファイルシステム操作

### 内部依存
- `./local-kfrag-generator.js`: WASM-packベースのkFrag生成器

## 使用シナリオ

### 1. 開発・テスト環境
```javascript
// テスト用の簡単なセットアップ
const dtpres = new D_TPRES_CWAO({ walletPath: './test-wallet.json' });
await dtpres.initialize();
await dtpres.deployFullWorkflow(null, 1); // 1つのHolder
```

### 2. プロダクション環境
```javascript
// 本格的な分散セットアップ
const dtpres = new D_TPRES_CWAO({
  walletPath: './production-wallet.json',
  protocol: "ao",
  variant: "ao_0_1"
});
await dtpres.initialize();
await dtpres.deployFullWorkflow('./optimized.wasm', 5); // 5つのHolder
```

### 3. カスタムワークフロー
```javascript
// 特定の要件に合わせたカスタム処理
const dtpres = new D_TPRES_CWAO(config);
await dtpres.initialize();

// カスタムスケジューラ使用
const customScheduler = "custom-scheduler-id";
await dtpres.deployModule('./custom.wasm');
await dtpres.spawnOwnerProcess(customScheduler);

// 非対称な閾値設定
await dtpres.executeKFragWorkflow('secret', 3, 7); // 3-of-7
```

このクラスは、D-TPRESの複雑な分散暗号処理を簡潔なAPIで実現し、AOネットワークとローカル暗号処理を統合する重要な橋渡し役を果たしています。