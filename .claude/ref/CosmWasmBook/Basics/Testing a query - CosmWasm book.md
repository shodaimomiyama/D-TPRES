---
title: "Testing a query - CosmWasm book"
source: "https://book.cosmwasm.com/basics/query-testing.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
前回は新しいクエリーを作成しましたが、今回は実際にテストを行う段階です。まずは基本となるユニットテストから始めましょう。この手法はシンプルで、Rustの知識以外には特に必要なものはありません。 `src/contract.rs` ファイルに移動し、そのモジュール内にテストを追加してください：

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
    use super::*;

    #[test]
    fn greet_query() {
        let resp = query::greet().unwrap();
        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_owned()
            }
        );
    }
}
```

Rustでユニットテストを書いたことがある方なら、ここで驚く点は何もありません。単にテスト専用モジュール内にローカル関数のユニットテストを記述するだけです。ただし問題は、このテストがまだビルドできないことです。メッセージ型を少し修正する必要があります。 `src/msg.rs` を更新してください：

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
    Greet {},
}
```

両方のメッセージ型に3つの新しい派生型を追加しました。 [`PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html) は型の等価比較を可能にするために必須です。これにより、型が等しいかどうかを確認できます。 [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) はデバッグ出力を生成するトレイトです。 [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html)で使用され、アサーションが失敗した場合の不一致情報を出力します。注意： `QueryMsg` 自体をテスト対象としていないため、これらの追加トレイト派生は任意です。ただし、テストの容易性と一貫性を保つため、すべてのメッセージ型に対して `PartialEq` と `Debug` を実装しておくことは推奨されるベストプラクティスです。 最後の [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html) は現時点では必須ではありませんが、ただし、メッセージのクローン作成を許可しておくことも推奨されるプラクティスです。後ほど必ず必要になるため、今のうちに追加しておきます。

これでテストを実行できる状態になりました：

```js
$ cargo test

...
running 1 test
test contract::tests::greet_query ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

成功！テストが通過しました！

さらに一歩進めてみましょう。Rustのテストユーティリティは、より高度なレベルのテストを構築するための優れたツールです。現在はスマートコントラクトの内部動作をテストしていますが、スマートコントラクトが外部世界からどのように見えるかを考えてみましょう。これは単一のエンティティであり、何らかの入力メッセージによってトリガーされます。 `query` 関数を介して契約全体をブラックボックスとして扱うテストを作成できます。テストを更新してみましょう：

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
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    use super::*;

    #[test]
    fn greet_query() {
        let resp = query(
            mock_dependencies().as_ref(),
            mock_env(),
            QueryMsg::Greet {}
        ).unwrap();
        let resp: GreetResp = from_binary(&resp).unwrap();

        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_owned()
            }
        );
    }
}
```

`query` 関数を実行するには、 `deps` と `env` の2つのエンティティを作成する必要がありました。 幸いなことに、 `cosmwasm-std` ライブラリにはこれらのテスト用ユーティリティが用意されています： [`mock_dependencies`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/testing/fn.mock_dependencies.html) および [`mock_env`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/testing/fn.mock_env.html) 関数です。

ここで必要な `Deps` 型ではなく、代わりに [`OwnedDeps`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.OwnedDeps.html) 型のモック依存関係を使用していることにお気づきでしょう。これは [`as_ref`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.OwnedDeps.html#method.as_ref) 関数がこのオブジェクトに対して呼び出されているためです。もし `DepsMut` オブジェクトを探していた場合、代わりに [`as_mut`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.OwnedDeps.html#method.as_mut) 関数を使用することになります。

このテストを再実行しても、問題なくパスするはずです。しかし、このテストが実際の使用ケースを正確に反映しているかどうかを考えると、不正確であることがわかります。コントラクトはクエリされていますが、一度もインスタンス化されていないのです！ソフトウェアエンジニアリングの観点から言えば、これはオブジェクトを生成せずにゲッターメソッドを呼び出すのと同等の行為です -これは全く根拠のないテスト手法です。より適切な方法があります：

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
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn greet_query() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("sender", &[]),
            Empty {},
        )
        .unwrap();

        let resp = query(deps.as_ref(), env, QueryMsg::Greet {}).unwrap();
        let resp: GreetResp = from_binary(&resp).unwrap();
        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_owned()
            }
        );
    }
}
```

ここで新たに導入された点が2つあります。まず、 `deps` と `env` 変数をそれぞれ独立した変数として抽出し、関数呼び出し時に引数として渡すようにしました。これらの変数はブロックチェーンの永続状態を表しており、各関数呼び出しごとに新たに生成する必要はないためです。契約状態の変更は `instantiate` 関数で設定された状態が `query` で正しく反映されるようにします。さらに、クエリ実行時とインスタンス化時で環境条件を制御できるようにしたいと考えています。

`info` 引数については別の扱いが必要です。このメッセージ情報は送信される各メッセージごとに固有のものです。 `info` モックを作成するには、 [`mock_info`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/testing/fn.mock_info.html) 関数に2つの引数を渡す必要があります。

最初の引数は呼び出しを実行するアドレスです。一見すると、 `sender` を謎の `wasm` とハッシュ値の代わりにアドレスとして渡すのは奇妙に思えるかもしれませんが、これは有効なアドレス形式です。テスト用途においては、このような形式の方が好ましい場合が多いです。なぜなら、テストが失敗した場合により詳細な情報が得られるからです。

2番目の引数はメッセージとともに送信された資金です。現時点では空のスライスのままにしておきます。トークン転送についてはまだ説明したくないためです - これについては後ほど解説します。

これでより現実的なシナリオになりました。ただ1つ気になる点があります。私は契約を単一のブラックボックスと表現していますが、ここでは `instantiate` 呼び出しと対応する `query` 呼び出しの間に明確な関連性が見当たりません。どうやら何らかのグローバル契約が存在することを前提としているようです。しかし、単一のテストケース内で2つの異なるコントラクトをインスタンス化したい場合、非常に煩雑な作業になることが予想されます。このプロセスを抽象化してくれるツールがあれば、どれほど便利でしょうか。