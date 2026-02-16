# FORMIX コントラクトアーキテクチャの理解

## コアコンセプト: 単一ロジック、複数ロール

### 1. **統一されたWASMモジュール**
FORMIXネットワークのすべてのプロセスは、`ao/contracts/` からコンパイルされた**同一のWASMモジュール**をロードして実行します。このモジュールには3つのロール用のハンドラーがすべて含まれています：
- Ownerハンドラー
- Holderハンドラー
- Requesterハンドラー

```
ao/contracts/
└── 単一のRustコードベース
    ├── Ownerハンドラー
    ├── Holderハンドラー
    └── Requesterハンドラー
    ↓
    WASMにコンパイル
    ↓
    Arweaveにデプロイ (tx_id取得)
    ↓
    すべてのプロセスが同じtx_idでspawn
```

### 2. **動的ロール実行**
プロセスには固定されたロールはありません。代わりに、**受信するメッセージタイプに基づいて異なるハンドラーを実行**します：

```rust
// contract.rs:196-249
execute(msg: ExecuteMsg) {
    match msg {
        // これらのメッセージを受信した時はOwnerとして動作
        ExecuteMsg::ReceiveKFrags { kfrags } => {
            handle_receive_kfrags(...)  // Ownerハンドラー
        }

        // これらのメッセージを受信した時はHolderとして動作
        ExecuteMsg::ReceiveKFrag { kfrag_data } => {
            handle_receive_kfrag(...)   // Holderハンドラー
        }
        ExecuteMsg::GenerateCFrag { kfrag_id } => {
            handle_generate_cfrag(...)  // Holderハンドラー
        }

        // これらのメッセージを受信した時はRequesterとして動作
        ExecuteMsg::StartCFragCollection { ... } => {
            handle_start_cfrag_collection(...)  // Requesterハンドラー
        }
    }
}
```

### 3. **プロセス動作の例**
単一のプロセスがコンテキストに応じて異なるロールで動作できます：

```
Process_A (同じWASMロジック)
├── ReceiveKFrags受信時 → Ownerとして動作
├── ReceiveKFrag受信時 → Holderとして動作
└── StartCFragCollection受信時 → Requesterとして動作

Process_B (同じWASMロジック)
├── ReceiveKFrags受信時 → Ownerとして動作
├── ReceiveKFrag受信時 → Holderとして動作
└── StartCFragCollection受信時 → Requesterとして動作
```

### 4. **初期化 vs ランタイムロール**
`instantiate` 関数は**初期メタデータを設定**しますが、プロセスを特定のロールにロックするわけではありません：

```rust
// contract.rs:103-118
instantiate(msg: InstantiateMsg) {
    match msg.process_role {
        ProcessRole::Owner => instantiate_owner(),      // Ownerメタデータを設定
        ProcessRole::Holder => instantiate_holder(),    // Holderメタデータを設定
        ProcessRole::Requester => instantiate_requester(), // Requesterメタデータを設定
    }
}
```

初期化後も、プロセスは受信するメッセージに基づいて**任意のハンドラーを実行**できます。

### 5. **プロセス間通信フロー**
Process_A（Ownerとして動作）がProcess_B（Holderとして動作）にkFragを送信する場合：

1. **Process_Aが受信** クライアントから`ReceiveKFrags`メッセージ
   - Ownerハンドラーを実行: `handle_receive_kfrags()`

2. **Process_Aが送信** Process_BへAOメッセージ
   - メッセージタイプ: `SendKFragToHolder`

3. **Process_Bが受信** AOメッセージ
   - Holderハンドラーを実行: `execute_send_kfrag_to_holder()`
   - これが呼び出す: `handle_receive_kfrag()`

### 6. **重要な意味**

#### セキュリティの考慮事項
- どのプロセスも任意のハンドラーを実行可能（すべて同じコードを持つため）
- セキュリティはメッセージ検証と送信者確認から得られる
- プロセスレベルでのハードなロール強制はない

#### 簡略化された検証
ミニマル実装のために：
- **assigned_ownersチェック不要**: メッセージ送信者の検証で十分
- **重複署名検証不要**: 送信プロセスで既に検証済み
- まずコア機能に集中し、後からセキュリティレイヤーを追加

#### プロセスのアイデンティティ
- プロセスのアイデンティティは**受信するメッセージ**と**初期化方法**によって決定される
- 実行可能なコードの内在的な制限によるものではない
- すべてのプロセスは能力において平等、使用コンテキストにおいて異なる

### 7. **開発とデプロイメントフロー**

```
開発:
1. ao/contracts/ の単一コードベース
2. WASMにコンパイル
3. Arweaveにデプロイ → tx_id取得

本番使用:
1. ユーザーが同じtx_idでプロセスをspawn
2. 希望する初期ロールメタデータで初期化
3. プロセスは任意の有効なメッセージタイプに応答
4. ロールはメッセージコンテキストによって決定、プロセス制限ではない
```

## まとめ
FORMIXは**均質なプロセスネットワーク**を実装しており、すべてのプロセスが同一のロジックを共有しますが、以下に基づいて異なる動作をします：
1. 初期化パラメーター
2. 受信するメッセージ
3. 動作するコンテキスト

このアーキテクチャは柔軟性とシンプルさを提供します。維持・デプロイするコードベースが1つだけでありながら、閾値プロキシ再暗号化システムに必要な分散ロールを実現できます。
