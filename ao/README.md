# ao/ — HyperBEAM-native AO Contract

AO-native Rust WASMコントラクト。旧来の`ao_cwao/`（CosmWasm版）をHyperBEAM (`~wasm64@1.0`) 向けに再設計。

## 旧バージョンとの違い

| 項目 | `ao_cwao/` (legacy) | `ao/` (this) |
|------|---------------------|--------------|
| フレームワーク | CosmWasm | AO-native WASM |
| デプロイツール | `cwao` v0.5.3 | `@permaweb/aoconnect` ^0.0.93 |
| ネットワーク | ao-testnet.xyz (廃止) | HyperBEAM |
| ストレージ | `cosmwasm_std::Storage` (KVS) | WASM global statics (メモリスナップショット) |
| エントリポイント | `instantiate/execute/query/reply` | `handle(msg_ptr, msg_len) -> result_ptr` |
| 状態管理 | CosmWasm CU が永続化 | HyperBEAM が WASM メモリを snapshot/restore |
| クロスコントラクト | `SubMsg` (同期) | `OutgoingMessage` (AO async message) |
| 暗号ライブラリ | `umbral-pre` (unchanged) | `umbral-pre` (unchanged) |

## ディレクトリ構成

```
ao/
├── contracts/              # Rust WASM contract
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # WASM entry point (handle fn)
│       ├── handlers.rs     # Business logic (dispatch + re-encryption)
│       ├── state.rs        # ProcessState (replaces CosmWasm storage maps)
│       ├── message.rs      # AOMessage / AOResponse types
│       └── getrandom_impl.rs  # Custom WASM getrandom
└── scripts/
    └── deploy.js           # HyperBEAM deployment via @permaweb/aoconnect
```

## ビルド

```bash
# Rust WASM binary
cd contracts
cargo build --target wasm32-unknown-unknown --release
# Output: target/wasm32-unknown-unknown/release/formix_contract.wasm

# wasm64 (Memory-64) の場合 (experimental)
# cargo build --target wasm64-unknown-unknown --release
```

## デプロイ

```bash
npm install
node scripts/deploy.js --wallet /path/to/wallet.json
```

## ⚠️ 残課題 (TODO)

1. **WASM ABI検証**: HyperBEAMのbeamrが `handle(ptr, len)` を呼ぶ正確な方法をPoC検証
2. **wasm32 vs wasm64**: `~wasm64@1.0`がMemory-64要求の場合 `wasm64-unknown-unknown` ターゲットが必要（experimental）
3. **SubMsg → async messaging**: `OutgoingMessage` の HyperBEAM ルーティング検証
4. **getrandom entropy**: 決定論的な placeholder → 本番用エントロピー注入に変更
5. **テスト**: `cw-multi-test` の代替テスト基盤構築
