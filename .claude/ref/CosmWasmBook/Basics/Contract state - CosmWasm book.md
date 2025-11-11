---
title: "Contract state - CosmWasm book"
source: "https://book.cosmwasm.com/basics/state.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
現在開発中のスマートコントラクトには、すでにクエリに応答できる基本的な動作が備わっています。ただし、その応答内容は非常に予測可能で、内容を変更するための仕組みは一切備えていません。本章では、スマートコントラクトに真の動的機能をもたらす「状態」の概念について解説します。

現時点では状態は静的なままです - コントラクトのインスタンス化時に初期化されます。この状態には、将来的にメッセージを送信する権限を持つ管理者のリストが含まれます。

最初に行うべき作業は、 `Cargo.toml` にさらに1つの依存関係を追加することです。具体的には、 [`storage-plus`](https://crates.io/crates/cw-storage-plus) クレートを追加します。このクレートは、CosmWasmスマートコントラクトの状態管理を行うための高レベルAPIを提供します：

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
cw-storage-plus = "0.13.4"

[dev-dependencies]
cw-multi-test = "0.13.4"
```

次に、コントラクト用の状態を保持する新しいファイルを作成します。通常このファイルは `src/state.rs` と名付けます：

```rust
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ADMINS: Item<Vec<Addr>> = Item::new("admins");
```

また、 `src/lib.rs` ファイルでこのモジュールを宣言することを忘れないでください：

```rust
use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

mod contract;
mod msg;
mod state;

#[entry_point]
pub fn instantiate(deps: DepsMut, env: Env, info: MessageInfo, msg: Empty)
  -> StdResult<Response>
{
    contract::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: msg::QueryMsg)
  -> StdResult<Binary>
{
    contract::query(deps, env, msg)
}
```

ここで新たに導入されているのは、 `ADMINS` という型 `Item<Vec<Addr>>` の定数です。ここで興味深い疑問が生じるかもしれません - この状態定数はどのように機能するのでしょうか？定数値である場合、どのように変更すればよいのでしょうか？

答えは少し複雑です。この定数自体は状態を保持しているわけではありません。状態はブロックチェーン上に保存されており、エントリーポイントに渡される `deps` 引数を介してアクセスします。storage-plusの定数は、この状態を構造化された方法でアクセスするための単なるアクセスユーティリティです。

CosmWasmにおいて、ブロックチェーンの状態は単なる大規模なキーバリューストレージです。キーにはメタ情報が付加されており、そのメタ情報によってそのキーを所有するコントラクトが特定されます（このため、他のいかなるコントラクトもこれらのキーを変更できません）。ただし、このメタ情報を削除すると、単一コントラクトの状態はより単純なキーバリューペアとして表現されます。

`storage-plus` は、アイテムキーをインテリジェントに前置することで、より複雑な状態構造を処理します。現時点では、最も単純なストレージエンティティである [`Item<_>`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html) を使用しています。これは、指定された型の単一のオプション値を保持するもので、 `Vec<Addr>` 型の値です。では、このストレージ内のアイテムに対するキーはどのように定義されるのでしょうか？これは問題ありません。 [`new`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html#method.new) 関数に渡される一意の文字列に基づいて、自動的に一意性が確保されます。

状態の初期化に進む前に、より適切な初期化メッセージを作成する必要があります。 `src/msg.rs` に移動して作成してください：

```rust
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
}
```

次に、 `src/contract.rs` ファイルのエントリーポイントを初期化し、インスタンス化メッセージで受け取ったデータに基づいて状態を初期化します：

```rust
use crate::msg::{GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
// --snip--
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(deps.storage, &admins?)?;

    Ok(Response::new())
}

pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
    }
}

#[allow(dead_code)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
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

また、 `src/lib.rs` のエントリポイントでメッセージ型を更新する必要もあります：

```rust
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use msg::InstantiateMsg;
// --snip--

mod contract;
mod msg;
mod state;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    contract::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}
```

これで状態を更新するために必要な作業は完了です！

まず最初に、文字列のベクターをブロックチェーンに保存するためのアドレスのベクターに変換する必要があります。メッセージ引数として直接アドレスを使用することはできません。なぜなら、すべての文字列が有効なアドレスとは限らないからです。テスト作業中にこの点が少し混乱を招く可能性があります。実際、テストコードでは任意の文字列をアドレスの代わりに使用できました。詳しく説明しましょう。

技術的に言えば、すべての文字列はアドレスとして扱うことが可能です。しかし、すべての文字列が実際にブロックチェーン上に存在する有効なアドレスというわけではありません。コントラクト内で `Addr` 型の値を保持する場合、それはブロックチェーン上の正当なアドレスであると仮定します。このため、 [`addr_validate`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/trait.Api.html#tymethod.addr_validate) この事前条件を検証するための関数が存在します。

保存するデータを用意したら、 [`save`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html#method.save) 関数を使用してコントラクト状態に書き込みます。 `save` の第一引数は [`&mut Storage`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/trait.Storage.html) であり、これは実際のブロックチェーンストレージです。強調されているように、 `Item` オブジェクト自体はデータを格納するものではなく、単なるアクセサです。データの保存方法を決定する役割を担っていますこの関数は指定されたストレージにデータを保存します。2番目の引数は、実際に保存するシリアル化可能なデータです。

回帰テストが正常に通過するかどうかを確認する良いタイミングです。以下のテストを実行してみてください：

```js
> cargo test

...

running 1 test
test contract::tests::greet_query ... FAILED

failures:

---- contract::tests::greet_query stdout ----
thread 'contract::tests::greet_query' panicked at 'called \`Result::unwrap()\` on an \`Err\` value: error executing WasmMsg:
sender: owner
Instantiate { admin: None, code_id: 1, msg: Binary(7b7d), funds: [], label: "Contract" }

Caused by:
    Error parsing into type contract::msg::InstantiateMsg: missing field \`admins\`', src/contract.rs:80:14
note: run with \`RUST_BACKTRACE=1\` environment variable to display a backtrace

failures:
    contract::tests::greet_query

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass '--lib'
```

やってしまいました！でも落ち着いてください。まずはエラーメッセージを丁寧に読み解いてみましょう：

> 型 contract::msg::InstantiateMsg への解析エラー: フィールド `admins` が欠落しています', src/contract.rs:80:14

問題は、テストコード内で初期化メッセージとして空のメッセージを送信している点です。現在のエンドポイントでは、 `admin` フィールドが必須となっています。マルチテストフレームワークはエントリーポイントから結果までの契約動作をテストするため、MTフレームワークの関数を使用してメッセージを送信する場合、まずそのメッセージがシリアライズされます。次にコントラクトはエントリーポイントでこれらのデータをデシリアライズします。しかし現在、空のJSONデータを何らかの非空メッセージにデシリアライズしようとしています！テストを更新することで迅速に修正できます：

```rust
use crate::msg::{GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(deps.storage, &admins?)?;

    Ok(Response::new())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
        AdminsList {} => to_binary(&query::admins_list(deps)?),
    }
}

#[allow(dead_code)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    unimplemented!()
}

mod query {
    use crate::msg::AdminsListResp;

    use super::*;

    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp {
            message: "Hello World".to_owned(),
        };

        Ok(resp)
    }

    pub fn admins_list(deps: Deps) -> StdResult<AdminsListResp> {
        let admins = ADMINS.load(deps.storage)?;
        let resp = AdminsListResp { admins };
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
                &InstantiateMsg { admins: vec![] },
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

状態が初期化された際、その動作を検証する方法が必要です。初期化処理が状態に正しく影響を与えるかどうかを確認するためのクエリ機能を実装したいと考えています。まずは、すべての管理者を一覧表示するシンプルなクエリを作成しましょう。 `src/msg.rs` ファイルに、クエリメッセージ用のバリアントと対応するレスポンスメッセージを追加してください。このバリアントを `AdminsList` 、レスポンスを `AdminsListResp` と名付け、 `cosmwasm_std::Addr` 型のベクターを返すようにします。

```rust
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub admins: Vec<Addr>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct AdminsListResp  {
    pub admins: Vec<Addr>,
}

[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
    AdminsList {},
}
```

そして実装を `src/contract.rs` に記述します：

```rust
use crate::msg::{AdminsListResp, GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(deps.storage, &admins?)?;

    Ok(Response::new())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
        AdminsList {} => to_binary(&query::admins_list(deps)?),
    }
}
 
#[allow(dead_code)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
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

    pub fn admins_list(deps: Deps) -> StdResult<AdminsListResp> {
        let admins = ADMINS.load(deps.storage)?;
        let resp = AdminsListResp { admins };
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
               &InstantiateMsg { admins: vec![] },
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

インスタンス化をテストするためのツールが整ったところで、テストケースを作成しましょう：

```rust
use crate::msg::{AdminsListResp, GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(deps.storage, &admins?)?;

    Ok(Response::new())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_binary(&query::greet()?),
        AdminsList {} => to_binary(&query::admins_list(deps)?),
    }
}

#[allow(dead_code)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
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

    pub fn admins_list(deps: Deps) -> StdResult<AdminsListResp> {
        let admins = ADMINS.load(deps.storage)?;
        let resp = AdminsListResp { admins };
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, ContractWrapper, Executor};

    use super::*;

    #[test]
    fn instantiation() {
        let mut app = App::default();

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: AdminsListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();

        assert_eq!(resp, AdminsListResp { admins: vec![] });

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg {
                    admins: vec!["admin1".to_owned(), "admin2".to_owned()],
                },
                &[],
                "Contract 2",
                None,
            )
            .unwrap();

        let resp: AdminsListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();

        assert_eq!(
            resp,
            AdminsListResp {
                admins: vec![Addr::unchecked("admin1"), Addr::unchecked("admin2")],
            }
        );
    }

    #[test]
    fn greet_query() {
        let mut app = App::default();

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
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

このテストは非常にシンプルです - 異なる初期管理者を指定してコントラクトを2回インスタンス化し、クエリ結果が毎回正しく取得できることを確認します。これは私たちがコントラクトをテストする際の一般的な手法です。具体的には、コントラクトに対して一連のメッセージを送信し、その後、特定のデータをクエリし、応答結果が期待通りであることを確認します。

私たちはコントラクトの開発を順調に進めています。次は、状態管理機能を活用し、実際の処理を実行させる段階です。