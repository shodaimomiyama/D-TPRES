# CosmWasm AO プロセス状態管理の完全ガイド

## 概要

このドキュメントは、CosmWasm AOにおけるプロセスの状態管理メカニズムについて、実装詳細とベストプラクティスをまとめています。CosmWasm AOは従来のブロックチェーンとは大きく異なる「分散アクターモデル」を採用しており、その結果として状態管理も独特の仕組みになっています。

**なぜこのドキュメントが重要か？**

従来のCosmWasmでは、状態はブロックチェーン上に直接保存され、ノード間で同期されます。しかし、CosmWasm AOでは「メッセージ履歴からの状態復元」という革新的なアプローチを取っています。これにより、同じプロセスを複数のCompute Unit（CU）で並列実行でき、真の分散処理が可能になります。

このアーキテクチャを理解せずにCosmWasm AOでプログラムを書くと、期待通りに動作しないコードになってしまいます。特にD-TPRESプロジェクトのような複雑な分散システムでは、この理解が成功の鍵となります。

## 目次

1. [プロセス再起動時の状態復元フロー](#プロセス再起動時の状態復元フロー)
2. [プロセス間メッセージングでの状態管理](#プロセス間メッセージングでの状態管理)
3. [Arweaveストレージアクセス制約](#arweaveストレージアクセス制約)
4. [KVストレージパターン](#kvストレージパターン)
5. [アーキテクチャの重要ポイント](#アーキテクチャの重要ポイント)

## プロセス再起動時の状態復元フロー

### 基本メカニズム：イベントソーシング

CosmWasm AOは**イベントソーシング方式**を採用しており、状態そのものではなく「状態変更イベント（メッセージ）」をArweaveに永続化します。

**これは従来のデータベースとは根本的に違う考え方です。**

例えば、銀行の口座残高を考えてみてください：
- **従来の方式**: 「田中さんの残高は10万円」という現在の状態をデータベースに保存
- **イベントソーシング**: 「田中さんが5万円入金」「田中さんが2万円出金」「田中さんが7万円入金」という取引履歴を保存し、必要な時に計算して「現在の残高は10万円」を導出

CosmWasm AOも同じ原理で動作します。プロセスの状態を直接保存するのではなく、プロセスに送られたすべてのメッセージ（状態変更イベント）を記録し、必要な時にそれらを順番に再実行して現在の状態を復元します。

**なぜこの方式を採用するのか？**

1. **完全な監査証跡**: どのような経緯で現在の状態になったかが完全に追跡可能
2. **決定論的再生**: 同じメッセージ列を実行すれば必ず同じ状態になる
3. **分散実行**: 任意のCUが独立してプロセスを実行できる
4. **タイムトラベル**: 過去の任意の時点の状態を復元可能

### 復元フローの詳細

実際のプロセス復元は以下の手順で行われます。これは、CU（Compute Unit）が新しくプロセスを担当することになった時や、システム再起動時に自動的に実行されます。

```
1. プロセスID (pid) を指定
   ↓ CUに「このプロセスを実行してください」という指示が来る
2. SU.process(pid) でメッセージグラフ（edges）取得
   ↓ スケジューラーに「このプロセスの実行履歴を教えて」と問い合わせ
3. プロセスのモジュールID経由でWASMをArweaveから取得
   ↓ 元のプログラムコード（バイナリ）をダウンロード
4. 新しいVMインスタンス作成（空の状態）
   ↓ 真っ新なメモリ状態でプログラムを起動
5. edgesの順序でメッセージを再実行
   - instantiate → execute1 → execute2 → ...
   ↓ 過去のメッセージを時系列順に「早送り再生」
6. 最新の状態が復元される
   ↓ 現在時点と同じ状態がメモリ上に再現される
```

**重要なポイント**: この復元プロセスは完全に自動化されており、プログラマが意識する必要はありません。ただし、プログラムを書く時は「メッセージの順序を変えても同じ結果になるか？」を常に考慮する必要があります。

### 実装例

```javascript
// CU (Compute Unit) での復元処理
async restoreProcess(pid) {
    // 1. WASMモジュールの取得
    const wasm = await this.arweave.transactions.getData(txid, { decode: true })
    const vm = new VM({ id: "ao", addr: pr_id })

    // 2. VMの初期化
    this.vms[pid] = await this.getModule(process.module, pid, input)
    await this._instantiate(pid, input)

    // 3. メッセージ履歴の取得と再実行
    const pmap = (await new SU({ url: this.sus[pid].url }).process(pid)).edges
    for (const v of pmap) {
        const id = v.node.message.id
        if (this.results[pid][id]) continue // 処理済みはスキップ

        // メッセージを順次実行
        const res = this.vms[pid].execute(caller, action, input)
        this.results[pid][id] = res.json
    }
}
```

### 重要な特徴

#### 決定論的再生：なぜ必ず同じ結果になるのか

**同じWASM + 同じメッセージ順序 = 必ず同じ状態**

これは単純に聞こえますが、実は非常に強力な保証です。例えば：
- 東京のCUで実行した結果
- ニューヨークのCUで実行した結果
- 1週間後に別のCUで実行した結果

これらは全て完全に同一になります。なぜなら、WASMは仮想マシンによって実行環境が標準化されており、同じ入力（メッセージ）に対して必ず同じ出力（状態変更）を生成するからです。

#### メインネット対応：本番環境での実用性

この仕組みはローカル開発環境の都合ではなく、本格的な分散システムとして設計されています。実際に：
- Arweaveネットワーク上で数千のノードが稼働
- 複数のCUが同時に異なるプロセスを実行
- ネットワーク障害や個別ノードの停止にも対応

#### 分散実行：真の並列処理

従来のブロックチェーンでは、全ノードが同じ計算を行うため、実質的には「多重冗長実行」でした。CosmWasm AOでは、異なるCUが異なるプロセスを担当できるため、真の並列処理が可能です。

例えば：
- CU-A: プロセス1, 2, 3を実行
- CU-B: プロセス4, 5, 6を実行
- CU-C: プロセス7, 8, 9を実行

これにより、システム全体の処理能力がCUの数に比例してスケールします。

## プロセス間メッセージングでの状態管理

### 状況設定：実世界での例

想像してください。あなたが電子商取引サイトを運営しているとします：
- **プロセスA**: 在庫管理システム（商品の在庫数を管理）
- **プロセスB**: 注文処理システム（顧客からの注文を処理）

顧客が商品を注文した時、プロセスBがプロセスAに「商品Xを1個減らしてください」というメッセージを送信します。この時、プロセスAは自分の現在の在庫状態を正確に把握して処理する必要があります。

従来のシステムなら「プロセスAの状態はどこかのデータベースに保存されている」と考えますが、CosmWasm AOでは違います。プロセスAの状態は**メッセージ履歴から毎回復元される**のです。

### なぜこれが重要なのか

CosmWasm AOでは、プロセス間でメッセージが送信された時に以下のことが起こります：

1. **CUは自動的にプロセスAの最新状態を復元する**
2. **復元された状態でプロセスBからのメッセージを処理する**
3. **処理結果を新しい状態として保存する**

この一連の流れは完全に自動化されており、開発者は通常のプログラムを書くのと同じ感覚でコードを記述できます。

### CUによる自動状態管理

```javascript
// CUでの自動処理
async execute(pid) {
    // プロセスAのメッセージキューを取得
    const pmap = (await new SU({ url: this.sus[pid].url }).process(pid)).edges

    for (const v of pmap) {
        const id = v.node.message.id
        if (this.results[pid][id]) continue

        // プロセスAの現在の状態で実行
        if (tags.read_only === "True") {
            res = this.vms[pid].query(tags.action, input)
        } else {
            res = this.vms[pid].execute(v.node.owner.address, tags.action, input)
        }

        this.results[pid][id] = res.json
    }
}
```

### コントラクト側での実装

```rust
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CallFromB { data } => {
            // deps.storageから自動的にプロセスAの状態が読み込まれる
            let current_state = PROCESS_STATE.load(deps.storage)?;

            // プロセスBからのデータで処理
            let new_state = process_with_data(current_state, data);

            // 更新された状態を保存
            PROCESS_STATE.save(deps.storage, &new_state)?;

            Ok(Response::new())
        }
    }
}
```

### 状態管理の自動化：開発者が意識しなくて良いこと

この自動化システムにより、開発者は以下のことを気にする必要がありません：

#### 1. VMインスタンスの管理
CUが各プロセスのVMを`this.vms[pid]`で管理しています。つまり：
- 「プロセスAが前回どこまで実行されていたか」を手動で追跡する必要なし
- 「メモリ不足でプロセスが停止した」場合の復旧処理は自動
- 「他のCUにプロセスを移行する」場合も透過的に実行

#### 2. 状態の透過的復元
`deps.storage`経由で現在の状態に自動アクセスできます：
- コードで`KV_STORE.load(deps.storage, "key")?`と書けば、常に最新の値が取得される
- 「どこから状態を読み込むか」「状態が最新かどうか」を確認する必要なし
- マルチCU環境でも一貫性が自動的に保証される

#### 3. 永続化の自動実行
状態変更後の永続化は自動的に実行されます：
- `KV_STORE.save(deps.storage, "key", &value)?`と書けば、自動的にArweaveに永続化
- 「いつ保存するか」「どこに保存するか」「重複防止」は全て自動処理
- ネットワーク障害時の再試行やデータ整合性も自動保証

**開発者の視点**: 通常のローカルプログラムを書くのと同じ感覚で、状態の読み書きができる。裏で動いている分散システムの複雑さは完全に隠蔽されている。

## Arweaveストレージアクセス制約

### なぜプロセスから直接Arweaveにアクセスできないのか

多くの開発者が最初に疑問に思うのは「なぜプロセスから直接Arweaveに読み書きできないのか？」ということです。これには深い設計思想があります。

#### 1. Querierの無効化：意図的な制限
```javascript
// /cosmwasm-ao/ao/cosmwasm.js:48
//querier: new BasicQuerier(),  // 意図的に無効化
```

従来のCosmWasmでは、`deps.querier`を使って他のコントラクトの状態を読み込めました。しかし、CosmWasm AOではこれを意図的に無効化しています。

**理由**: もしプロセスAがプロセスBの状態を直接読み込めるとしたら：
- プロセスAを実行する前に、プロセスBの最新状態が必要
- プロセスBの最新状態を得るには、プロセスBに関連する全メッセージを先に処理する必要
- これでは並列処理が不可能になってしまう

#### 2. WASMサンドボックス制約：セキュリティ境界
- **外部ネットワークアクセス禁止**: プロセスが勝手に外部サーバーと通信することを防ぐ
- **ファイルシステムアクセス禁止**: ホストマシンのデータを保護
- **`deps.storage`のみ利用可能**: 制御された環境でのデータアクセスのみ許可

これにより、悪意のあるプロセスでもシステム全体に害を与えることができません。

#### 3. アクターモデル設計：ハイパーパラレリズムの実現
アクターモデルでは、各アクター（プロセス）は独立して動作し、メッセージングでのみ通信します：

- **同期的な外部アクセス禁止**: 「他のプロセスの処理完了を待つ」ことがない
- **ハイパーパラレリズム維持**: 数千のプロセスが同時並行で実行可能
- **デッドロック回避**: プロセス間の循環依存を根本的に防止

**具体例**:
従来のシステムでは「プロセスAがプロセスBの応答を待っている間、プロセスAは停止」していました。AOでは「プロセスAはメッセージを送信後すぐに次の処理を継続し、後でコールバックで結果を受信」します。

### 代替実現方法：制約を回避する賢いパターン

直接アクセスできない制約は一見不便に見えますが、実際には明確で予測可能なアーキテクチャを生み出します。以下の方法で、必要な機能を実現できます。

#### 方法1: メッセージングによる間接アクセス

**シナリオ**: プロセスAが大きなファイルをArweaveに保存したい

これは「専用のストレージ管理プロセス」を介して実現します。まるで現実世界で「倉庫管理会社に荷物の保管を依頼する」ようなものです。

```rust
// プロセスA: データ保存要求
pub fn execute_save_to_arweave(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    data: Vec<u8>,
) -> Result<Response, ContractError> {
    let msg = SubMsg::new(WasmMsg::Execute {
        contract_addr: storage_process_addr,
        msg: to_binary(&StorageMsg::SaveToArweave {
            data,
            callback_process: env.contract.address.clone(),
        })?,
        funds: vec![],
    });

    Ok(Response::new().add_submessage(msg))
}

// コールバックで結果受信
pub fn execute_arweave_saved(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    txid: String,
) -> Result<Response, ContractError> {
    ARWEAVE_REFS.save(deps.storage, &txid)?;
    Ok(Response::new())
}
```

#### 方法2: 参照データパターン（推奨）

**シナリオ**: 大容量データは外部保存し、プロセスは「住所録」のみを管理

これは最も実用的なパターンです。図書館で本を借りる時を考えてください：
- **図書カード**: 本のタイトル、著者、棚番号が書かれている（軽量データ）
- **実際の本**: 棚に置かれている（重量データ）

プロセスは「図書カード」に相当する参照情報のみを管理し、実際のデータは別の場所に保存します。

```rust
// データ構造設計
pub struct ProcessData {
    // プロセス内で直接管理
    pub metadata: Metadata,
    pub state: State,

    // Arweaveへの参照のみ保持
    pub large_data_txid: Option<String>,
    pub snapshot_txid: Option<String>,
}

// 参照の保存
pub const EXTERNAL_REFS: Map<String, String> = Map::new("external_refs");

pub fn execute_store_reference(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    key: String,
    txid: String,
) -> Result<Response, ContractError> {
    EXTERNAL_REFS.save(deps.storage, key, &txid)?;
    Ok(Response::new())
}
```

## KVストレージパターン：プロセスのメモリを構築する

### KVストレージとは何か

Key-Value（KV）ストレージは、最もシンプルで強力なデータ構造です。辞書や住所録と同じ概念です：

- **Key（キー）**: 「田中太郎」「商品A」「設定_言語」
- **Value（値）**: 「03-1234-5678」「在庫50個」「日本語」

CosmWasm AOでは、この単純な仕組みを使ってプロセスの全ての状態を管理します。SQLデータベースのような複雑なクエリ機能はありませんが、その分シンプルで高速です。

### なぜKVストレージを使うのか

1. **決定論的**: 同じキーには必ず同じ値が入る
2. **高速**: インデックス検索で瞬時にアクセス
3. **分散対応**: ネットワーク越しでも効率的に同期
4. **型安全**: Rustの型システムと相性が良い

### 基本的な実装

```rust
// state.rs - KVマップの定義
use cw_storage_plus::Map;

pub const KV_STORE: Map<String, String> = Map::new("kv_store");
pub const TYPED_STORE: Map<String, Config> = Map::new("typed_store");

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub threshold: u64,
    pub mode: String,
}
```

### メッセージハンドラでのKV操作

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    SetValue { key: String, value: String },
    SetMultiple { pairs: Vec<(String, String)> },
    ProcessWithValue { key: String, operation: String },
}

pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetValue { key, value } => {
            // CUメモリ上のKVストレージに保存
            KV_STORE.save(deps.storage, key.clone(), &value)?;

            Ok(Response::new()
                .add_attribute("action", "set_value")
                .add_attribute("key", key)
                .add_attribute("value", value))
        },

        ExecuteMsg::ProcessWithValue { key, operation } => {
            // KVから値を取得して処理に使用
            let value = KV_STORE.load(deps.storage, key.clone())?;

            // 取得した値を使って任意のロジックを実行
            let result = match operation.as_str() {
                "double" => format!("{}{}", value, value),
                "reverse" => value.chars().rev().collect(),
                _ => value.clone(),
            };

            KV_STORE.save(deps.storage, format!("{}_result", key), &result)?;

            Ok(Response::new()
                .add_attribute("action", "process")
                .add_attribute("result", result))
        }
    }
}
```

### 状態復元時のKV再構築：魔法のような自動復元

ここがCosmWasm AOの最も魅力的な部分です。プロセスが再起動した時、過去のメッセージを「早送り再生」することで、まるで何事もなかったかのように元の状態が復元されます。

**具体的なシナリオ**:
1. 午前10時: `SetValue { key: "counter", value: "1" }` メッセージ
2. 午前11時: `SetValue { key: "counter", value: "2" }` メッセージ
3. 午後1時: システム再起動が発生
4. 午後1時1分: 過去のメッセージが自動的に再実行される
5. 結果: `counter = "2"` が正確に復元される

プロセス再起動時、メッセージ履歴の再実行により自動的にKVが再構築されます：

```javascript
// 内部的な動作（CU）
async restoreKVState(pid) {
    // 1. 空のKVストレージでVM開始
    this.vms[pid] = new VM({ storage: new BasicKVIterStorage() });

    // 2. メッセージ履歴を順次実行
    const messages = await SU.process(pid).edges;
    for (const msg of messages) {
        if (msg.action === "set_value") {
            // この実行でKVが再構築される
            this.vms[pid].execute(msg.sender, msg.action, {
                key: "a", value: "1"  // 例
            });
        }
    }

    // 3. 完全なKVストレージが復元される
    // { "a": "1", "b": "2", "config": "production" }
}
```

### 効率的なパターン：実用的なテクニック

開発経験を積むと、より効率的なパターンが見えてきます。以下は実際のプロジェクトでよく使われる手法です。

#### バルク初期化：一度に複数のデータを設定

アプリケーション起動時に、設定データを一括で初期化したい場合があります。例えば、ゲームの初期パラメータや、システムの設定値などです。

```rust
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // 初期化時に複数のKVペアを設定
    for (key, value) in msg.initial_kv_pairs {
        KV_STORE.save(deps.storage, key, &value)?;
    }

    Ok(Response::new())
}
```

#### 型安全なストレージ：バグを防ぐ設計

Rustの最大の利点は型安全性です。KVストレージでも、この利点を最大限活用できます。

**問題**: 全てを文字列で管理すると、型の間違いが起きやすい
```rust
// 危険な例
KV_STORE.save(deps.storage, "user_age", &"25")?;  // 文字列として保存
let age = KV_STORE.load(deps.storage, "user_age")?;  // 文字列として読み込み
let next_year = age.parse::<u32>()? + 1;  // 毎回パースが必要、エラーが起きやすい
```

**解決策**: 型別にストレージを分離する

```rust
// 用途別のストレージ分離
pub const STRING_STORE: Map<String, String> = Map::new("strings");
pub const NUMBER_STORE: Map<String, u64> = Map::new("numbers");
pub const CONFIG_STORE: Map<String, Config> = Map::new("configs");

// 複合的な状態管理：型安全で読みやすい
pub fn execute_complex_logic(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    // 型が明確なので、間違いが起きにくい
    let string_val = STRING_STORE.load(deps.storage, "key1".to_string())?;  // 確実に文字列
    let number_val = NUMBER_STORE.load(deps.storage, "key2".to_string())?;  // 確実に数値
    let config = CONFIG_STORE.load(deps.storage, "app_config".to_string())?;  // 確実に設定オブジェクト

    // 型安全な演算（コンパイル時にエラー検出）
    let result = if config.enabled {
        format!("{}: {}", string_val, number_val * config.threshold)
    } else {
        "disabled".to_string()
    };

    STRING_STORE.save(deps.storage, "result".to_string(), &result)?;

    Ok(Response::new())
}
```

## アーキテクチャの重要ポイント

ここでは、CosmWasm AOのアーキテクチャで特に重要な概念を、実例を交えて説明します。これらを理解することで、より堅牢で効率的なプログラムを書けるようになります。

### 1. ステートレス実行モデル：メモリのリセット

**従来のプログラミングとの最大の違い**

一般的なプログラムでは、変数はプログラム実行中ずっとメモリに残り続けます：
```rust
// 通常のプログラム
static mut COUNTER: u32 = 0;  // プログラム終了まで値が保持される

fn increment() {
    unsafe {
        COUNTER += 1;  // 前の値が残っている
    }
}
```

しかし、CosmWasm AOでは**メッセージ処理が終わるたびにメモリがリセット**されます：

#### 重要な特徴

- **メモリ非永続化**: メッセージ間でメモリ状態は保持されない
- **明示的な状態ロード**: 全ての状態は`deps.storage`から明示的にロード
- **分散実行対応**: 異なるCUでも同じ結果を保証

#### 実例で理解する

```rust
// ❌ 間違った書き方（動作しない）
static mut COUNTER: u32 = 0;

pub fn execute_increment(deps: DepsMut, ...) -> Result<Response, ContractError> {
    unsafe {
        COUNTER += 1;  // ⚠️ 次のメッセージ処理時には0にリセットされる
    }
    Ok(Response::new())
}

// ✅ 正しい書き方
pub const COUNTER: Item<u32> = Item::new("counter");

pub fn execute_increment(deps: DepsMut, ...) -> Result<Response, ContractError> {
    let mut count = COUNTER.load(deps.storage).unwrap_or(0);  // 永続化されたストレージから読み込み
    count += 1;
    COUNTER.save(deps.storage, &count)?;  // 永続化ストレージに保存
    Ok(Response::new())
}
```

### 2. AOのアクターモデル設計：分散システムの美しい分業

アクターモデルは、複雑な分散システムを理解しやすくする設計思想です。現実世界の組織に例えると理解しやすくなります。

#### 4つの専門チームによる分業体制

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MU (1985)     │    │   SU (1986)     │    │   CU (1987)     │
│ Messenger Unit  │    │ Scheduler Unit  │    │ Compute Unit    │
│   📮郵便局       │    │   📅秘書室       │    │   🏭工場        │
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
                    │   🏛️図書館       │
                    │                 │
                    │ • データ永続化   │
                    │ • トランザクション│
                    │ • 状態バックアップ│
                    └─────────────────┘
```

#### 各ユニットの役割を日常業務に例えると

**📮 MU（Messenger Unit）- 郵便局**
- 外部からのメッセージを受け取る窓口
- 「この手紙は確実に本人からのものか？」（署名検証）
- 「この手紙はどこに保管すべきか？」（Arweave送信）

**📅 SU（Scheduler Unit）- 秘書室**
- 「今日はどの仕事をどの順番で処理するか？」（スケジュール）
- 「プロジェクトAの最新の進捗状況は？」（プロセス管理）
- 複数の工場（CU）の作業を調整

**🏭 CU（Compute Unit）- 工場**
- 実際にプログラムを実行する
- 秘書室からの指示に従って、順番通りに作業を処理
- 作業結果をきちんと記録

**🏛️ Arweave - 図書館**
- 全ての情報を永久保存
- 「2年前のこの日に何が起きたか？」を調べられる
- 世界中どこからでもアクセス可能

### 3. 必須のメッセージハンドラパターン：3段階の鉄則

CosmWasm AOでメッセージを処理する時は、必ず以下の3段階を踏む必要があります。これは「料理の基本手順」のようなもので、順番を間違えると期待通りの結果が得られません。

#### 基本的なパターン

```rust
pub fn handle_message(msg: AOMessage, repo: &dyn Repository) -> Result<Response> {
    // 🍽️ 1. 材料を準備する（状態をロード）
    let mut state = repo.load_state(msg.process_id)?;

    // 👨‍🍳 2. 料理をする（メッセージを処理）
    let result = process_with_state(&mut state, msg)?;

    // 🏪 3. 冷蔵庫にしまう（状態を永続化）
    repo.save_state(msg.process_id, &state)?;

    Ok(result)
}
```

#### なぜこの順序が重要なのか

1. **状態ロードを忘れる**: 「前回の続きから始めるつもりが、毎回最初からやり直し」になる
2. **状態保存を忘れる**: 「今日一日の作業が全て水の泡」になる
3. **処理と保存の順序を間違える**: 「まだ作業途中なのに結果を保存」してしまう

#### 実際のコード例

```rust
pub fn execute_transfer_funds(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: u64,
) -> Result<Response, ContractError> {
    // 1. 現在の残高をロード
    let sender_balance = BALANCES.load(deps.storage, info.sender.clone())
        .unwrap_or(0);
    let recipient_balance = BALANCES.load(deps.storage, recipient.clone())
        .unwrap_or(0);

    // 2. 送金処理（残高確認 + 計算）
    if sender_balance < amount {
        return Err(ContractError::InsufficientFunds {});
    }

    let new_sender_balance = sender_balance - amount;
    let new_recipient_balance = recipient_balance + amount;

    // 3. 新しい残高を保存
    BALANCES.save(deps.storage, info.sender.clone(), &new_sender_balance)?;
    BALANCES.save(deps.storage, recipient.clone(), &new_recipient_balance)?;

    Ok(Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("amount", amount.to_string()))
}
```

### 4. 決定論的再生の保証

- **同じWASMコード**
- **同じメッセージ順序**
- **同じ入力パラメータ**
→ **必ず同じ状態に到達**

### 5. 制約と最適化

#### 制約
- **async/await不可**: 同期実行のみ
- **外部アクセス禁止**: ネットワーク、ファイルシステム
- **クロスコントラクトクエリ無効**: `deps.querier`は使用不可

#### 最適化戦略
- **メッセージバッチング**: 複数の状態変更を1メッセージで実行
- **スナップショット**: 定期的な状態スナップショットで履歴再生コストを削減
- **型安全性**: 強い型付けでランタイムエラーを防止

## D-TPRESプロジェクトでの適用

### 1. 多重役割プロセス設計

```rust
// 役割別のストレージ分離
pub const OWNER_STATE: Item<OwnerState> = Item::new("owner_state");
pub const HOLDER_STATE: Item<HolderState> = Item::new("holder_state");
pub const REQUESTER_STATE: Item<RequestState> = Item::new("requester_state");

// 共通KVストレージ
pub const SHARED_KV: Map<String, String> = Map::new("shared_kv");
```

### 2. 暗号化データの管理

```rust
// 秘密情報は適切にZeroize
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecretState {
    #[zeroize(skip)]
    pub public_info: PublicInfo,
    pub secret_shares: Vec<SecretShare>,
}

// Arweave参照による大容量データ管理
pub const CAPSULE_REFS: Map<String, String> = Map::new("capsule_refs");
```

### 3. フェーズ管理の実装

```rust
pub const PHASE_STATE: Item<PhaseState> = Item::new("phase_state");

pub fn execute_advance_phase(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut phase = PHASE_STATE.load(deps.storage)?;

    // フェーズの進行チェック
    phase.advance()?;

    // 状態を保存
    PHASE_STATE.save(deps.storage, &phase)?;

    Ok(Response::new())
}
```

## まとめ

CosmWasm AOの状態管理は以下の原則に基づいています：

1. **イベントソーシング**: 状態変更をメッセージとして記録
2. **決定論的再生**: メッセージ履歴から状態を完全再構築
3. **分散実行**: 任意のCUで同じ結果を保証
4. **アクター分離**: プロセス間の独立性を維持

この設計により、AOネットワークは真の分散実行環境として、ブロックチェーンの制約を超えた超並列処理を実現しています。D-TPRESプロジェクトでは、これらの原則を活用して効率的で安全な分散鍵管理システムを構築できます。

---

*このドキュメントは CosmWasm AO の実装分析と実際の議論に基づいて作成されました。*