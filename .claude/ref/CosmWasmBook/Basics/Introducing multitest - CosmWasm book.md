---
title: "Introducing multitest - CosmWasm book"
source: "https://book.cosmwasm.com/basics/multitest-intro.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
[`multitest`](https://crates.io/crates/cw-multi-test) をご紹介します - これはRustでスマートコントラクトのテストを作成するためのライブラリです。

`multitest` の中核的なコンセプトは、スマートコントラクトのエンティティを抽象化し、 テスト目的でブロックチェーン環境をシミュレートすることです。この機能の目的は、 スマートコントラクト間の通信をテスト可能にすることにあります。このツールはとはいえ、単一コントラクトのシナリオをテストするための優れたツールでもあります。

まず最初に、 `Cargo.toml` ファイルにmultitestを追加します。

```toml
[package]
name = "contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cosmwasm-std = { version = "1.0.0-beta8", features = ["staking"] }
serde = { version = "1.0.103", default-features = false, features = ["derive"] }

[dev-dependencies]
cw-multi-test = "0.13.4"
```

新たに [`[dev-dependencies]`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies) セクションを追加しました。このセクションには、最終的なバイナリでは使用されないものの、開発プロセスで使用されるツール（例えばテストなど）のための依存関係を定義します。

依存関係の準備が整ったら、テストコードをフレームワーク対応に更新します：

```rust
use crate::msg::{GreetResp, QueryMsg};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
    }
}

#[allow(dead_code)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty
) -> StdResult<Response> {
    unimplemented!()
}

mod query {
    use super::*;

    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp {
            message: "Hello World".to_owned(),
        };

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, ContractWrapper, Executor};

    use super::*;

    #[test]
    fn greet_query() {
        let mut app = App::default();

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &Empty {},
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: GreetResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::Greet {})
            .unwrap();

        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_owned()
            }
        );
    }
}
```

おそらくお気づきかと思いますが、 `execute` エントリーポイント用の関数を追加しました。エントリーポイント自体や関数の実装は追加していませんが、マルチテスト用のコントラクトには、少なくとも以下の3つのハンドラが含まれている必要があります： - インスタンス化 - クエリ - 実行 この関数には [`#[allow(dead_code)]`](https://doc.rust-lang.org/reference/attributes/diagnostics.html#lint-check-attributes) 属性を付与しています。このため、 `cargo` はこの依存関係がどこにも使用されていないことをエラーとして報告しません。 `#[cfg(test)]` ディレクティブを使用してテスト環境のみで有効化するのも有効な方法です。

テストの冒頭で、私は [`App`](https://docs.rs/cw-multi-test/0.13.4/cw_multi_test/struct.App.html#) オブジェクトを作成しました。これは仮想ブロックチェーンを表現するマルチテストの中核的なエンティティで、ここで契約の実行を行います。ご覧の通り、このオブジェクトに対しても `wasmd` を使用してブロックチェーンとやり取りするのと同様に関数を呼び出すことができます！

`app` オブジェクトを作成直後に、ブロックチェーンに「アップロード」される `code` の表現を準備しました。multitest はネイティブの Rust テストであるため、Wasm バイナリは関与しませんが、この名称は実際の運用シナリオで発生する事象とよく一致しています。このオブジェクトはブロックチェーンに [`store_code`](https://docs.rs/cw-multi-test/0.13.4/cw_multi_test/struct.App.html#method.store_code) 関数を使用して保存します。これにより、コントラクトをインスタンス化する際に必要となるコードIDが取得されます。

次のステップはコントラクトのインスタンス化です。単一の [`instantiate_contract`](https://docs.rs/cw-multi-test/0.13.4/cw_multi_test/trait.Executor.html#method.instantiate_contract) 関数呼び出しにおいて、 `wasmd` 経由で提供するすべての情報を指定します：コントラクトコードID、インスタンス化を実行するアドレス、

トリガーとなるメッセージと、メッセージに付随する資金（現時点ではいずれも空）を指定します。また、移行管理用にコントラクトのラベルと管理者情報を追加します - `None` と設定します。現時点ではこの情報は不要であるためです。

契約がオンライン状態になったら、その状態を照会できます。 [`wrap`](https://docs.rs/cw-multi-test/0.13.4/cw_multi_test/struct.App.html?search=in#method.wrap) 関数はクエリ API へのアクセス機能です（クエリは他の呼び出し方法と若干異なる処理が行われます）。 [`query_wasm_smart`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.QuerierWrapper.html#method.query_wasm_smart) クエリでは、契約とメッセージを含むオブジェクトを指定します。また、クエリ結果については `Binary` 形式のまま扱う必要はありません。multitest では、これらを何らかのレスポンス型にデシリアライズすることを想定しているため、Rust の型省略機能を活用して、使いやすい API を提供してくれます。

テストを再実行するタイミングです。テストは依然としてパスするはずですが、今回はテスト用コントラクト全体を 適切に抽象化できました。今後はさらに、状態変数を追加することでコントラクトをより興味深いものにする方法を 解説していく予定です。