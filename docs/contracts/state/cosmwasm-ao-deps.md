# CosmWasm AO における deps（Dependencies）の完全解説

## 目次
1. [deps の概要](#deps-の概要)
2. [実際の通信先の特定](#実際の通信先の特定)
3. [データ永続化の詳細プロセス](#データ永続化の詳細プロセス)
4. [アーキテクチャと役割](#アーキテクチャと役割)
5. [CosmWasm標準との違い](#cosmwasm標準との違い)

## deps の概要

### CosmWasm における deps とは

CosmWasmコントラクトにおいて、`deps`（Dependencies）はブロックチェーンとの相互作用を提供する必須インターフェースです。

```rust
// @cosmwasm-ao/tools/myapp/contracts/src/contract.rs:15
let n = NUM.load(deps.storage)?;

// @cosmwasm-ao/tools/myapp/contracts/src/contract.rs:27
NUM.save(deps.storage, &msg.num)?;
```

### deps の構成要素

1. **`deps.storage`** - ストレージアクセス
   - 値の読み取り（`load`）
   - 値の保存（`save`）

2. **`deps.api`** - APIユーティリティ
   - アドレス検証
   - アドレス形式変換

3. **`deps.querier`** - 他コントラクトへのクエリ
   - **CosmWasm AO では無効化されている**

### 読み取り専用 vs 書き込み可能

- **`Deps`**: クエリ関数用（読み取り専用）
- **`DepsMut`**: execute/instantiate用（書き込み可能）

## 実際の通信先の特定

### **結論：depsはArweaveチェーンと通信している（AO経由）**

以下の具体的なファイルと行番号から判断できます：

### 1. Querierの無効化
`/cosmwasm-ao/ao/cosmwasm.js:48`
```javascript
//querier: new BasicQuerier(),
```
他のチェーンへのクエリ機能が意図的に無効化されている

### 2. チェーンIDの設定
`/cosmwasm-ao/ao/cosmwasm.js:60`
```javascript
chain_id: "ao",
```
CosmWasmコントラクトが認識するチェーンIDが"ao"

### 3. Arweaveアドレス形式の検証
`/cosmwasm-ao/ao/cosmwasm.js:27-28`
```javascript
if (!/^[a-z0-9_-]{43}$/i.test(source.str)) {
  throw new Error("Invalid address.")
}
```
43文字のBase64URL形式（Arweave特有のアドレス形式）

### 4. Arweaveクライアントの初期化
`/cosmwasm-ao/cwao-sdk/cwao.js:24`
```javascript
this.arweave = new Arweave(arweave)
```

### 5. デフォルトのArweaveローカル設定
`/cosmwasm-ao/cwao-sdk/cwao.js:13-17`
```javascript
arweave = {
  host: "localhost",
  port: 1984,
  protocol: "http",
},
```

### 6. コントラクトデータのArweaveへの保存
`/cosmwasm-ao/cwao-sdk/cwao.js:38-41`
```javascript
const tx = await this.arweave.createTransaction({ data: mod })
for (let v of tags) tx.addTag(v.name, v.value)
await this.arweave.transactions.sign(tx, this.wallet)
await this.arweave.transactions.post(tx)
```

### 7. CUからArweaveへのデータ取得
`/cosmwasm-ao/ao/cu.js:68`
```javascript
const wasm = await this.arweave.transactions.getData(txid, { decode: true })
```

## データ永続化の詳細プロセス

### `NUM.save(deps.storage, &value)` から Arweave永続化までの完全な流れ

### 📝 **Step 1: コントラクトでのデータ保存**
```rust
NUM.save(deps.storage, &msg.num)?
```
↓
**実装**: `/cosmwasm-ao/ao/cosmwasm.js:47`
```javascript
storage: new BasicKVIterStorage()
```
→ **CUのWASMメモリ内**のKey-Valueストレージに即座に保存

---

### 📤 **Step 2: Execute関数の実行**
**実装**: `/cosmwasm-ao/cwao-sdk/cwao.js:58-70`
```javascript
async execute({ process, action, input, query = false, custom }) {
  let tags = this.data.tag.message({ input, action, read_only }, custom)
  const item = await this.data.dataitem({ target: process, tags }, "", signer)
  return await this.mu.post(item)  // MUに送信
}
```

---

### 📨 **Step 3: DataItemの作成（Arweave形式）**
**実装**: `/cosmwasm-ao/cwao-sdk/data.js:57-61`
```javascript
async dataitem(tags = {}, data = "", signer) {
  const dataitem = createData(data, _signer, tags)  // Arbundlesライブラリ使用
  await dataitem.sign(_signer)  // 署名
  return dataitem
}
```
→ **Arweaveのデータ形式**に変換され、署名される

---

### 🚀 **Step 4: MU（Messenger Unit）への送信**
**実装**: `/cosmwasm-ao/cwao-sdk/mu.js:10-20`
```javascript
async post(msg) {
  return await fetch(this.url, {
    method: "POST",
    headers: {
      "Content-Type": "application/octet-stream",
      Accept: "application/json",
    },
    body: msg.getRaw(),  // 署名済みDataItem
  }).then(r => r.json())
}
```
→ MUがメッセージを受信し、**メッセージIDを返す**

---

### 📊 **Step 5: SU（Scheduler Unit）での順序付け**
**実装**: `/cosmwasm-ao/ao/cu.js:156`
```javascript
const pmap = (await new SU({ url: this.sus[pid].url }).process(pid)).edges
```
→ SUが**メッセージの実行順序を管理**

**SU API**: `/cosmwasm-ao/cwao-sdk/su.js:14-16`
```javascript
async process(pid) {
  return await fetch(`${this.url}/${pid}`).then(r => r.json())
}
```

---

### ⚙️ **Step 6: CU（Compute Unit）での実行**
**実装**: `/cosmwasm-ao/ao/cu.js:142-153`
```javascript
async _execute({ pid, tags, input, id, v }) {
  let res = null
  if (tags.read_only === "True") {
    res = this.vms[pid].query(tags.action, input)
  } else {
    let caller = tags.from_process ?? v.node.owner.address
    res = this.vms[pid].execute(caller, tags.action, input)
  }
  this.results[pid][id] = res.json  // 実行結果をメモリに保存
}
```

**VM実行**: `/cosmwasm-ao/ao/cosmwasm.js:72-75`
```javascript
execute(sender, action, input) {
  this.height++
  return this.vm.execute(this.env(), this.info(sender), { [action]: input })
}
```

---

### 💾 **Step 7: Arweaveへの永続化**
**実装**: `/cosmwasm-ao/cwao-sdk/data.js:96-98`
```javascript
async post(tx) {
  await this.arweave.transactions.post(tx)  // Arweaveに送信
}
```

**トランザクション作成**: `/cosmwasm-ao/cwao-sdk/data.js:80-93`
```javascript
async tx(bundle) {
  let tx = null
  if (this.wallet.sign) {
    tx = await this.arweave.createTransaction({
      data: bundle.binary,
    })
    tx.addTag("Bundle-Format", "binary")
    tx.addTag("Bundle-Version", "2.0.0")
    await this.arweave.transactions.sign(tx)
  } else {
    tx = await bundle.toTransaction({}, this.arweave, this.wallet)
    await this.arweave.transactions.sign(tx, this.wallet)
  }
  return tx
}
```

---

### 🔄 **Step 8: プロセス再起動時の状態復元**
**モジュール取得**: `/cosmwasm-ao/ao/cu.js:68-69`
```javascript
const wasm = await this.arweave.transactions.getData(txid, { decode: true })
const vm = new VM({ id: "ao", addr: pr_id })
```

**状態復元**: `/cosmwasm-ao/ao/cu.js:106-107`
```javascript
this.vms[pid] = await this.getModule(process.module, pid, input)
await this._instantiate(pid, input)  // 初期状態を復元
```

**初期化プロセス**: `/cosmwasm-ao/ao/cu.js:74-78`
```javascript
async _instantiate(pid, input) {
  let result = null
  result = this.vms[pid].instantiate(this.msgs[pid].owner.address, input)
  this.results[pid][pid] = result.json
}
```

## アーキテクチャと役割

### AO Units の構成

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MU (1985)     │    │   SU (1986)     │    │   CU (1987)     │
│ Messenger Unit  │    │ Scheduler Unit  │    │ Compute Unit    │
│                 │    │                 │    │                 │
│ • メッセージ受信  │    │ • メッセージ順序  │    │ • WASM実行      │
│ • Arweave送信   │    │ • スケジュール   │    │ • 状態管理      │
│ • 署名検証      │    │ • プロセス管理   │    │ • 結果計算      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │ Arweave (1984)  │
                    │   永続ストレージ   │
                    │                 │
                    │ • データ永続化   │
                    │ • トランザクション│
                    │ • 状態バックアップ│
                    └─────────────────┘
```

### CosmWasm VM の実装

**実装**: `/cosmwasm-ao/ao/cosmwasm.js:44-53`
```javascript
async getVM(wasm) {
  const backend = {
    backend_api: new BasicBackendApi("ao"),
    storage: new BasicKVIterStorage(),
    //querier: new BasicQuerier(),  // 無効化
  }
  this.vm = new VMInstanceAO(backend)
  await this.vm.build(wasm)
  return this.vm
}
```

### 環境設定

**実装**: `/cosmwasm-ao/ao/cosmwasm.js:55-64`
```javascript
env() {
  return {
    block: {
      height: this.height,
      time: Number(Date.now()).toString(),
      chain_id: "ao",
    },
    contract: { address: this.addr },
  }
}
```

## CosmWasm標準との違い

### 1. **Querierの無効化**

**標準CosmWasm**:
```rust
// 他のコントラクトの状態をクエリ可能
let balance: BalanceResponse = deps.querier.query(&QueryRequest::Bank(
    BankQuery::Balance {
        address: "some_address".to_string(),
        denom: "token".to_string(),
    }
))?;
```

**CosmWasm AO**:
```javascript
// /cosmwasm-ao/ao/cosmwasm.js:48
//querier: new BasicQuerier(),  // コメントアウト = 無効
```

**理由**: `/cosmwasm-ao/README.md:79-81`
> "Querier to read states from other contracts are disabled in CosmWasm on AO. This behaviour requires inefficient and blocking synchronous operations between multiple processes, and would be the biggest bottleneck to the hyperparallelism and hyper scalability of the AO network."

### 2. **非原子的メッセージ処理**

**標準CosmWasm**:
```rust
// add_message でアトミックな失敗が可能
let msg = CosmosMsg::Bank(BankMsg::Send { ... });
response = response.add_message(msg);  // 失敗時は全体がロールバック
```

**CosmWasm AO**:
```rust
// add_submessage を使用（非同期処理）
let submsg = SubMsg::new(msg);
response = response.add_submessage(submsg);  // 独立して処理される
```

**理由**: `/cosmwasm-ao/README.md:73-75`
> "In the original CosmWasm, you can call cross contract functions with add_message and atomically fail if the target contract execution fails. But this is not the case with AO since AO is pure actor model."

### 3. **アドレス形式の違い**

**標準CosmWasm** (Cosmos SDK):
```
cosmos1abc123def456...  (Bech32形式)
```

**CosmWasm AO**:
```javascript
// /cosmwasm-ao/ao/cosmwasm.js:27-28
if (!/^[a-z0-9_-]{43}$/i.test(source.str)) {
  throw new Error("Invalid address.")
}
```
```
ABC123def456ghi789...  (43文字のBase64URL)
```

**変換サポート**: `/cosmwasm-ao/ao/cosmwasm.js:19-24`
```javascript
function toBech32(arweaveAddress, prefix = "ao") {
  const decodedBytes = base64url.toBuffer(arweaveAddress)
  const words = bech32.toWords(decodedBytes)
  const bech32Address = bech32.encode(prefix, words)
  return bech32Address
}
```

## まとめ

### 重要なポイント

1. **非同期永続化**: データはまずCUメモリ→その後Arweaveへ
2. **メッセージ駆動**: すべての状態変更はメッセージとして記録
3. **決定論的再生**: メッセージ履歴から状態を完全に再構築可能
4. **アクターモデル**: 各プロセス（コントラクト）は独立したメモリ空間
5. **超並列性**: Querierの無効化により、プロセス間の依存関係を排除

### 実際の動作

```rust
let n = NUM.load(deps.storage)?;
```

この行は：
- **即座に**: CUのWASMメモリから読み取り
- **最終的に**: Arweaveに永続化された状態から復元可能
- **非同期的に**: AOプロトコルがArweaveへの永続化を管理

つまり、**AOがArweaveへの永続化を完全に抽象化**しており、開発者は通常のCosmWasmのように書けますが、裏で全てがArweaveに保存されています。

---

*このドキュメントは CosmWasm AO のソースコード分析に基づいて作成されました。*
