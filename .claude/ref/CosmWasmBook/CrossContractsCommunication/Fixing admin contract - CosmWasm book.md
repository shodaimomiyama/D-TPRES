---
title: "Fixing admin contract - CosmWasm book"
source: "https://book.cosmwasm.com/cross-contract/fixing-admin.html"
author:
published:
created: 2025-11-11
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
達成すべき目標が明確になった今、既存のコントラクトを管理者用コントラクトとして機能するように調整することから始めましょう。現時点では基本的な部分は問題ありませんが、クリーンアップ作業を行います。

最初に行うべき作業は、 `Greet` クエリを削除することです。これは学習用のサンプルクエリとしては適していましたが、実際には実用的な用途がなく、単に不要なノイズを生成しているに過ぎません。

クエリ列挙型から不要なバリアントを削除する必要があります：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
    pub donation_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Leave {},
    Donate {},
}

#[cw_serde]
pub struct AdminsListResp {
    pub admins: Vec<Addr>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AdminsListResp)]
    AdminsList {},
}
}
```

次に、クエリディスパッチャー内の無効なパスも削除します：

```rust
#![allow(unused)]
fn main() {
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        AdminsList {} => to_binary(&query::admins_list(deps)?),
    }
}
}
```

最後に、 `contract::query` モジュールから関連性のないハンドラを削除します。 また、このハンドラへのすべての参照が完全に削除されていることを確認する必要があります（テストコード内に存在する場合など）。

本書の冒頭で、 `Cargo.toml` ファイル内の `crate-type` を `"cdylib"` に設定しました。これはWasm出力を生成するために必要な設定でしたが、以下のような欠点があります - 動的ライブラリとして生成されるため、他のプロジェクトの依存関係として使用できないという問題です。以前は問題ありませんでしたが、実際には多くの場合、他のコントラクトに依存して特定のタイプの機能（例えば定義済みメッセージなど）を利用したいことがあります。

これは私たちにとって好都合です。簡単に修正可能です。 `crate-type` が単一の文字列ではなく配列であることにお気づきでしょう。この理由は、私たちのプロジェクトが複数のターゲットを生成できるためです。具体的には、デフォルトの `"rlib"` ライブラリ型を追加して、「Rustライブラリ」形式の出力を生成するように設定します。これは他のプロジェクトで依存関係として使用する場合に必要な形式です。 `Cargo.toml` ファイルを以下のように更新しましょう：

```toml
[package]
name = "admin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]
# 
# [features]
# library = []
# 
# [dependencies]
# cosmwasm-std = { version = "1.1.4", features = ["staking"] }
# serde = { version = "1.0.103", default-features = false, features = ["derive"] }
# cw-storage-plus = "0.15.1"
# thiserror = "1"
# schemars = "0.8.1"
# cw-utils = "0.15.1"
# cosmwasm-schema = "1.1.4"
# 
# [dev-dependencies]
# cw-multi-test = "0.15.1"
```

また、コントラクト名を変更しました - 「contract」という名前はあまり説明的ではないため、 「admin」に更新しています。

最後に、プロジェクトの構造をより適切に整理します。これまで私たちは単一のコントラクトのみを管理していたため、プロジェクト全体を一括して扱っていました。今後は作成する各コントラクト間の関係性を適切に反映したディレクトリ構造を構築したいと考えています。

まずプロジェクト用のディレクトリを作成します。次に、そのディレクトリ内に「contracts」（契約）というサブディレクトリを作成します。これはRustの技術的要件ではありませんが、ワークスペース最適化ツールなど、環境内のツールが契約ファイルを探す標準的な場所として認識するため、この構成が推奨されます。これはCosmWasmコントラクトリポジトリでよく見られる標準的なパターンです。

次に、前章で作成したプロジェクトディレクトリ全体を `contracts` ディレクトリにコピーし、 `admin` にリネームします。

最後に、すべてのプロジェクトを連携させます（現時点では1つですが、将来的には複数になる予定です）。そのために、最上位レベルのプロジェクトディレクトリ内にワークスペース用の `Cargo.toml` ファイルを作成します：

```toml
[workspace]
members = ["contracts/*"]
resolver = "2"
```

この `Cargo.toml` ファイルは、通常のプロジェクトレベルのものとやや異なります - これはワークスペースを定義するものです。最も重要なフィールドは `members` で、これはワークスペースを構成するプロジェクトを指定します。

もう一つの重要な設定項目は `resolver` です。これは必ず設定しておくべき項目で、 Cargoに対して依存関係解決にバージョン2を使用するよう指示します。この設定はRust 2021以降、 非ワークスペース環境ではデフォルトで有効になっていますが、互換性維持のため、従来のデフォルト設定はワークスペースでは変更できませんでしたが、新たに作成するすべてのワークスペースにこの設定を追加することが推奨されます。

ワークスペースで役立つ可能性のある最後のフィールドは \`exclude\` です。これは、このワークスペースの一部ではないプロジェクトをワークスペースディレクトリツリー内に作成することを可能にします - ここでは使用しませんが、知っておくと便利です。

ここで明確にするため、最上位ディレクトリ構造を確認しておきましょう：

```js
.
├── Cargo.lock
├── Cargo.toml
├── contracts
│  └── admin
└── target
   ├── CACHEDIR.TAG
   └── debug
```

ツリー構造内に表示される対象ディレクトリと `Cargo.lock` ファイルが確認できるのは、 すでに `admin` コントラクトのテストビルドと実行を完了しているためです。Rustワークスペースでは、 \`cargo\` は最上位ディレクトリからビルドを行うため、たとえ内部ディレクトリからビルドする場合でもこの動作が適用されます。