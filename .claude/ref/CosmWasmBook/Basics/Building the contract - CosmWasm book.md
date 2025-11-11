---
title: "Building the contract - CosmWasm book"
source: "https://book.cosmwasm.com/basics/building-contract.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
いよいよコントラクトのビルド作業に入ります。ローカル環境でのテスト用には、従来のcargoビルドパイプラインを使用できます： `cargo build` でコンパイルを行い、 `cargo test` ですべてのテストを実行します（現時点ではテストケースが存在しませんが、近日中に実装する予定です）。

ただし、ブロックチェーンにコントラクトをデプロイするためには、wasmバイナリを作成する必要があります。ビルドコマンドに追加の引数を指定することでこれを実行できます：

```js
$ cargo build --target wasm32-unknown-unknown --release
```

`--target` 引数を指定すると、Cargo は実行中の OS 向けのネイティブバイナリではなく、指定されたターゲット向けのクロスコンパイルを実行します。この場合、 `wasm32-unknown-unknown` を指定していますが、これは WebAssembly ターゲットを指す正式な名称です。

さらに、コマンドに `--release` 引数を指定しました。これは必須ではありませんが、オンチェーン環境で実行する場合、通常デバッグ情報はほとんど役に立ちません。ガスコストを最小限に抑えるためには、アップロードするバイナリサイズを可能な限り小さくすることが重要です。 [CosmWasm用Rustツールキット](https://github.com/CosmWasm/rust-optimizer) [オプティマイザ](https://github.com/CosmWasm/rust-optimizer) ツールを使用すると、さらに小さなバイナリを生成できます。本番環境では、すべてのコントラクトをこのツールでコンパイルする必要がありますが、学習目的であれば必須の作業ではありません。

おそらく、単純な `cargo build` コマンドではなく、複雑なコマンドでコントラクトをビルドすることに不満を感じていらっしゃるのでしょう。ご安心ください、実際にはそのようなことはありません。一般的なベストプラクティスとして、ビルドコマンドにエイリアスを設定することで、ネイティブアプリケーションをビルドするのと同じくらい簡単に行えるようにしています。

契約プロジェクトディレクトリ内に以下の内容を記述した`.cargo/config` ファイルを作成します：

```toml
[alias]
wasm = "build --target wasm32-unknown-unknown --release"
wasm-debug = "build --target wasm32-unknown-unknown"
```

Wasmバイナリのビルドは、 `cargo wasm` を実行するのと同じくらい簡単です。また、デバッグ情報を含ませたWasmバイナリをビルドする必要がある稀なケースに備えて、追加の `wasm-debug` コマンドも実装しました。

コントラクトのビルドが完了したら、最後のステップとしてそのコントラクトが有効な CosmWasm コントラクトであることを確認するために、 `cosmwasm-check` コマンドを実行する必要があります。

```js
$ cargo wasm
...
$ cosmwasm-check ./target/wasm32-unknown-unknown/release/contract.wasm
Available capabilities: {"cosmwasm_1_1", "staking", "stargate", "iterator", "cosmwasm_1_2"}

./target/wasm32-unknown-unknown/release/contract.wasm: pass
```