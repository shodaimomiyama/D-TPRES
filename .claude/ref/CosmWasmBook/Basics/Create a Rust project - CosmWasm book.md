---
title: "Create a Rust project - CosmWasm book"
source: "https://book.cosmwasm.com/basics/rust-project.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
スマートコントラクトはRustのライブラリパッケージとして作成するため、まずライブラリの作成から始めましょう：

```js
$ cargo new --lib ./empty-contract
```

シンプルなRustライブラリは作成できましたが、これだけではまだスマートコントラクトとして機能しません。まず最初に行うべきは、 `Cargo.toml` ファイルの更新です：

```toml
[package]
name = "contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cosmwasm-std = { version = "1.0.0-beta8", features = ["staking"] }
```

ご覧の通り、ライブラリセクションに `crate-type` フィールドを追加しました。 `cdylib` を生成することは、適切なWebAssemblyバイナリを作成する上で必須です。この方法の欠点として、このようなライブラリは他のRustライブラリの依存関係として使用できません。現時点では必須ではありませんが、後ほどこの依存関係の活用方法について説明します。契約を依存関係として再利用する方法について説明します。

さらに、スマートコントラクトには必須のコア依存関係が1つあります： `cosmwasm-std` です。このライブラリは スマートコントラクト向けの標準ライブラリで、外部システムとの通信に必要な基本的なユーティリティや、 いくつかの補助関数・型を提供します。私たちが構築するすべてのスマートコントラクトは、この依存関係を使用します。