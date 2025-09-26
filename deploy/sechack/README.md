# D-TPRES MVP Demo Scripts

このディレクトリには、D-TPRES（Deterministic Threshold Proxy Re-Encryption System）のMVPデモ用スクリプトが含まれています。

## 概要

D-TPRES MVPは以下のフローを実装しています：

1. **ローカルでのkFrag生成まで** (`local-keygen.js`)
2. **Owner ProcessへのkFragのinput** (`spawn-processes.js`)
3. **Owner Processから1つのHolder Processへの送信** (`test-flow.js`)
4. **Holder ProcessでのkFragのプロキシ再暗号化処理** (`test-flow.js`)

## 前提条件

- Node.js 14以上
- D-TPRESプロジェクトのRustコードベース

## デモ実行手順

### 1. ローカル鍵生成とkFrag作成

```bash
cd deploy
node local-keygen.js
```

この処理では：
- 秘密データのShamir分散
- Owner/Accessor鍵ペア生成
- カプセル作成
- kFrag生成
- 結果をJSONファイルで保存

**生成されるファイル：**
- `output/local-keygen-result.json` - 完全な処理結果
- `output/owner-message.json` - Owner-Process用メッセージ
- `output/holder-message.json` - Holder-Process用メッセージ

### 2. AO Processのspawn

```bash
node spawn-processes.js
```

この処理では：
- Owner-Processのシミュレート spawn
- Holder-Processのシミュレート spawn
- Process IDの生成と管理

**生成されるファイル：**
- `output/spawned-processes.json` - プロセス情報

**オプション：**
- `--status` - 現在のプロセス状況を表示
- `--force` - 既存プロセスを強制再生成

### 3. エンドツーエンドテスト

```bash
node test-flow.js
```

この処理では：
- Owner-ProcessへのkFrag転送テスト
- Holder-ProcessでのkFrag保存テスト
- cFrag生成テスト
- 全体フローの検証

**生成されるファイル：**
- `output/test-results/test-results-[timestamp].json` - テスト結果
- `output/generated-cfrag.json` - 生成されたcFrag

## ファイル構成

```
deploy/
├── README.md                 # このファイル
├── local-keygen.js          # ローカル暗号化処理
├── spawn-processes.js       # AO Process spawn
├── test-flow.js            # エンドツーエンドテスト
└── output/                  # 生成ファイル格納ディレクトリ
    ├── local-keygen-result.json
    ├── owner-message.json
    ├── holder-message.json
    ├── spawned-processes.json
    ├── generated-cfrag.json
    └── test-results/
        └── test-results-*.json
```

## コンポーネント詳細

### local-keygen.js

**機能：**
- O-Browser（ローカル）でのPhase 1処理をシミュレート
- 秘密データの分散と暗号化
- kFrag生成
- AOメッセージ形式での出力

**使用する技術：**
- Node.js Crypto API
- シミュレートされたShamir秘密分散
- シミュレートされたUmbral PRE

### spawn-processes.js

**機能：**
- AO Network上でのプロセス管理をシミュレート
- Owner-ProcessとHolder-Processのspawn
- Process IDの生成と追跡

**プロセス情報：**
```json
{
  "id": "dtpres-owner-1640995200000-abc123",
  "type": "owner",
  "status": "active",
  "tags": {
    "Data-Protocol": "DTPRES",
    "Type": "Owner-Process",
    "Version": "0.1.0-mvp"
  }
}
```

### test-flow.js

**機能：**
- 完全なエンドツーエンドフローのテスト
- メッセージ送信のシミュレート
- 各段階での結果検証
- テストレポート生成

**テスト項目：**
1. 前提条件チェック
2. ローカル結果読み込み
3. プロセス情報読み込み
4. Owner-Processテスト
5. Holder-Processテスト
6. cFrag生成テスト

## MVPの制限事項

このMVPは以下の制限があります：

### 暗号化実装
- 実際のUmbral PRE実装ではなくシミュレーション
- 簡略化されたShamir秘密分散
- AES-GCMの代わりにXOR暗号化

### プロセス管理
- 実際のAO Networkではなくローカルシミュレーション
- RandAOによるHolder選択なし
- 1つのHolder-Processのみ使用

### ネットワーク通信
- 実際のArweaveストレージなし
- AOメッセージングのシミュレーション
- メモリ内状態管理

## 実際の実装への拡張

このMVPを実際のD-TPRESに拡張するには：

1. **Rust WASM統合**
   - `umbral-pre`と`sssa`ライブラリの使用
   - Rust実装のブラウザ統合

2. **AO Network統合**
   - 実際のAOプロセスのdeploy
   - Arweaveストレージの使用
   - メッセージングプロトコルの実装

3. **セキュリティ強化**
   - 適切な乱数生成
   - メモリセキュリティの実装
   - 暗号学的検証の追加

## トラブルシューティング

### よくある問題

**1. ファイルが見つからない**
```bash
# 順序を確認
node local-keygen.js    # 最初
node spawn-processes.js # 次
node test-flow.js      # 最後
```

**2. プロセスが既に存在する**
```bash
node spawn-processes.js --force
```

**3. テスト失敗**
```bash
# 個別のログを確認
ls -la output/
cat output/test-results/test-results-*.json
```

### デバッグ

各スクリプトは詳細なログを出力します：
- `🔐` 暗号化処理
- `🚀` プロセス管理
- `📤` メッセージ送信
- `✅` 成功
- `❌` エラー

## 開発者向け情報

### カスタマイズ

設定は各ファイルの`CONFIG`オブジェクトで変更可能：

```javascript
const CONFIG = {
    secret: "カスタム秘密データ",
    threshold: 3,      // 閾値
    totalShares: 5,    // 総シェア数
    outputDir: "./custom-output"
};
```

### モジュール使用

各スクリプトはモジュールとしても使用可能：

```javascript
const { LocalOwnerProcessor } = require('./local-keygen');
const processor = new LocalOwnerProcessor();
// カスタム処理...
```

## ライセンス

D-TPRESプロジェクトと同じライセンスが適用されます。