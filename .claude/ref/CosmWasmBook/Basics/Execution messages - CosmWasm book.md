---
title: "Execution messages - CosmWasm book"
source: "https://book.cosmwasm.com/basics/execute.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
これまでインスタンス化メッセージとクエリーメッセージについて説明してきました。いよいよ最後の基本エントリーポイントである「実行メッセージ」について解説します。これまでの内容と類似しているため、すでに習得した知識を再確認するような感覚で気軽に進めていただけると思います。ここで説明している内容を実際にご自身で実装してみることをお勧めします - ソースコードを参照せずに この演習に取り組んでみてください。

このコントラクトの仕組みはシンプルです。すべてのコントラクト管理者は、以下の2種類の実行メッセージを発行する権限を持ちます：

- `AddMembers` メッセージを使用すると、管理者は管理者リストに別のアドレスを追加できます。
- `Leave` メッセージにより、管理者は自身をリストから削除することができます

特に複雑な内容ではありません。さっそくコーディングを始めましょう。まずはメッセージの定義から始めます：

```rust
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ExecuteMsg {
    AddMembers { admins: Vec<String> },
    Leave {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct AdminsListResp {
    pub admins: Vec<Addr>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
    AdminsList {},
}
```

次に、エントリーポイントを実装します：

```rust
use crate::msg::{AdminsListResp, ExecuteMsg, GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

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
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    use ExecuteMsg::*;

    match msg {
        AddMembers { admins } => exec::add_members(deps, info, admins),
        Leave {} => exec::leave(deps, info),
    }
}

mod exec {
    use cosmwasm_std::StdError;

    use super::*;

    pub fn add_members(
        deps: DepsMut,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> StdResult<Response> {
        let mut curr_admins = ADMINS.load(deps.storage)?;
        if !curr_admins.contains(&info.sender) {
            return Err(StdError::generic_err("Unauthorised access"));
        }

        let admins: StdResult<Vec<_>> = admins
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();

        curr_admins.append(&mut admins?);
        ADMINS.save(deps.storage, &curr_admins)?;

        Ok(Response::new())
    }

    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.update(deps.storage, move |admins| -> StdResult<_> {
            let admins = admins
                .into_iter()
                .filter(|admin| *admin != info.sender)
                .collect();
            Ok(admins)
        })?;

        Ok(Response::new())
    }
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

    use crate::msg::AdminsListResp;

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

エントリーポイント自体も `src/lib.rs` で作成する必要があります：

```rust
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}
```

いくつかの新しい要素がありますが、特に複雑なものではありません。まず重要なのは、メッセージ送信者が管理者かどうかを確認したり、そのリストから削除したりする方法です。ここでは `MessageInfo` の `info.sender` フィールドを使用しています。これは以下のようにメンバーとして定義されています - まさにこのメンバーです。メッセージは常に適切なアドレスから送信されるため、 `sender` は既に `Addr` 型として定義されており、検証の必要はありません。もう一つの新しい要素は、 [`update`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html#method.update) 関数です。これは `Item` に対してエンティティの読み取りと更新を行うもので、処理の効率性を向上させることができます。管理者リストを最初に読み取り、その結果を更新して保存することで実現可能です。

`Item` を扱う際、常に何らかのデータが存在することを前提としていることにお気づきでしょう。しかし、インスタンス化時に `ADMINS` の値を初期化することは必須ではありません！この場合、どうなるのでしょうか？ `load` 関数と `update` 関数の両方がエラーを返すことになります。ただし、 [`may_load`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html#method.may_load) この関数は `StdResult<Option<T>>` 型を返します。ストレージが空の場合、 `Ok(None)` を返します。さらに、既存のアイテムをストレージから削除するには、 [`remove`](https://docs.rs/cw-storage-plus/0.13.4/cw_storage_plus/struct.Item.html#method.remove) 関数を使用できます。

改善すべき点の一つはエラー処理です。送信者の権限を検証する際に、単なる任意の文字列をエラーとして返していますが、より適切な処理が可能です。

当社契約において、ユーザーが管理者権限を持たない状態で `AddMembers` 関数を実行しようとすると、エラーが発生します。この状況を適切に処理するためのエラーケースが [`StdError`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/enum.StdError.html) には存在しないため、汎用的なエラーメッセージを返すしかありません。これは最適な解決策とは言えません。

エラー報告には、 [`thiserror`](https://crates.io/crates/thiserror/1.0.24/dependencies) ライブラリの使用を強く推奨します。 まずは依存関係の更新から始めましょう：

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
thiserror = "1"

[dev-dependencies]
cw-multi-test = "0.13.4"
```

次に、 `src/error.rs` ファイルにエラー型を定義します：

```rust
use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("{sender} is not contract admin")]
    Unauthorized { sender: Addr },
}
```

また、新しいモジュールを `src/lib.rs` に追加する必要があります：

```rust
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

mod contract;
mod error;
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
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}
```

`thiserror` を使用すると、エラーをシンプルな列挙型のように定義でき、このライブラリが その型が [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) トレイトを実装することを保証します。このライブラリの特に便利な機能として、 `#[error]` 属性による [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) トレイトのインライン定義が挙げられます。さらに、 `#[from]` 属性も非常に便利で、これにより、適切な [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) 実装が自動的に生成されるため、`?` 演算子を `thiserror` 型と組み合わせて簡単に使用できます。

次に、実行エンドポイントを更新して新しいエラー型を使用するようにします：

```rust
use crate::error::ContractError;
use crate::msg::{AdminsListResp, ExecuteMsg, GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

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
 
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        AddMembers { admins } => exec::add_members(deps, info, admins),
        Leave {} => exec::leave(deps, info).map_err(Into::into),
    }
}

mod exec {
    use super::*;

    pub fn add_members(
        deps: DepsMut,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response, ContractError> {
        let mut curr_admins = ADMINS.load(deps.storage)?;
        if !curr_admins.contains(&info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let admins: StdResult<Vec<_>> = admins
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();

        curr_admins.append(&mut admins?);
        ADMINS.save(deps.storage, &curr_admins)?;

        Ok(Response::new())
    }

    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.update(deps.storage, move |admins| -> StdResult<_> {
            let admins = admins
                .into_iter()
                .filter(|admin| *admin != info.sender)
                .collect();
            Ok(admins)
        })?;

        Ok(Response::new())
    }
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

    use crate::msg::AdminsListResp;

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

エントリーポイントの戻り値型も更新する必要があります：

```rust
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use error::ContractError;
use msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

mod contract;
mod error;
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
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}
```

適切なカスタムエラー型を使用する利点の一つは、マルチテストが [`anyhow`](https://crates.io/crates/anyhow) クレートを使用してエラー型を管理している点です。このクレートは `thiserror` の兄弟プロジェクトとして設計されており、型消去されたエラーを実装しつつ、元のエラー情報を保持できるようにしています。

管理者権限を持たないユーザーがリストに自分自身を追加できないかどうかを確認するテストを作成しましょう：

```rust
use crate::error::ContractError;
use crate::msg::{AdminsListResp, ExecuteMsg, GreetResp, InstantiateMsg, QueryMsg};
use crate::state::ADMINS;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

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

pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        AddMembers { admins } => exec::add_members(deps, info, admins),
        Leave {} => exec::leave(deps, info).map_err(Into::into),
    }
}

mod exec {
    use super::*;

    pub fn add_members(
        deps: DepsMut,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response, ContractError> {
        let mut curr_admins = ADMINS.load(deps.storage)?;
        if !curr_admins.contains(&info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let admins: StdResult<Vec<_>> = admins
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();

        curr_admins.append(&mut admins?);
        ADMINS.save(deps.storage, &curr_admins)?;

        Ok(Response::new())
    }

    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.update(deps.storage, move |admins| -> StdResult<_> {
            let admins = admins
                .into_iter()
                .filter(|admin| *admin != info.sender)
                .collect();
            Ok(admins)
        })?;

        Ok(Response::new())
    }
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

    use crate::msg::AdminsListResp;

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

    #[test]
    fn unauthorized() {
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

        let err = app
            .execute_contract(
                Addr::unchecked("user"),
                addr,
                &ExecuteMsg::AddMembers {
                    admins: vec!["user".to_owned()],
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(
            ContractError::Unauthorized {
                sender: Addr::unchecked("user")
            },
            err.downcast().unwrap()
        );
    }
}
```

コントラクトの実行は、他の通常の呼び出しとほとんど同じです。私たちは [`execute_contract`](https://docs.rs/cw-multi-test/0.13.4/cw_multi_test/trait.Executor.html#method.execute_contract) 関数を使用します。実行中にエラーが発生する可能性があるため、この呼び出しからはエラー型が返されますが、 `unwrap` を使って値を抽出するのではなく、エラーが発生することを期待します。これが [`unwrap_err`](https://doc.rust-lang.org/std/result/enum.Result.html#method.unwrap_err) 呼び出しの目的です。エラー値を取得したので、 `assert_eq!`を使って、期待したエラーと一致するか確認できます。ただし若干の注意点があります： `execute_contract` から返されるエラーは [`anyhow::Error`](https://docs.rs/anyhow/1.0.57/anyhow/struct.Error.html) 型ですが、実際には `ContractError` 型であることを期待しています。幸いなことに、前述の通り、 `anyhow` エラーは [`downcast`](https://docs.rs/anyhow/1.0.57/anyhow/struct.Error.html#method.downcast) 関数を使用することで元の型を復元できます。直後に `unwrap` が必要なのは、ダウンキャストが失敗する可能性があるからです。これは、 `downcast` が内部エラーに保持されている型を魔法のように把握できないためです。これは文脈から推論されます - ここでは比較対象が `ContractError` であるため、 この型が暗黙的に推論されるという魔法のような仕組みが働いています。ただし、もし実際のエラーが `ContractError` でない場合、 `unwrap` はパニック（強制終了）を引き起こします。

ここでは実行時エラーの簡単なテストケースを作成しましたが、これだけでは契約が本番環境対応済みと主張するには不十分です。 すべての合理的な正常ケースを網羅する必要があります。本書のこの章を読み終えたら、実際にいくつかのテストケースを作成し、実験してみることをお勧めします。