---
title: "Creating a query - CosmWasm book"
source: "https://book.cosmwasm.com/basics/query.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
すでに、空のインスタンス化メッセージに応答するシンプルなコントラクトを作成しました。残念ながら、これはあまり実用的ではありません。もう少し反応性の高いものに改良しましょう。

まず最初に、 [`serde`](https://crates.io/crates/serde) クレートをプロジェクトの依存関係に追加する必要があります。これにより、クエリメッセージのシリアライズとデシリアライズが容易になります。 `Cargo.toml` ファイルを以下のように更新してください：

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

次に、 `src/lib.rs` ファイルに移動し、新しいクエリエントリポイントを追加します：

```rust
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdResult,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct QueryResp {
    message: String,
}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    let resp = QueryResp {
        message: "Hello World".to_owned(),
    };

    to_binary(&resp)
}
```

簡潔さを保つため、以前に作成したインスタンス化エンドポイントについては省略しています。コードが複雑になりすぎないように、変更された部分のみを常に表示するようにしています。

クエリから返す構造体を定義する必要があります。常にシリアライズ可能なオブジェクトを返すようにします。ここでは単に、 `serde` クレートから `Serialize` と `Deserialize` トレイトを派生させています。

次に、エントリーポイントを実装します。これは `instantiate` 関数と非常によく似ています。 最も重要な違いは、 `deps` 引数の型です。 `instantiate` の場合は `DepMut` でしたが、ここでは [`Deps`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Deps.html) オブジェクトを使用しています。これは、クエリがスマートコントラクトの内部状態を変更できないためです。クエリは状態を読み取ることしかできません。これにはいくつかの制約が生じます - 例えば、 将来のクエリ用にキャッシュを実装することはできません（キャッシュにデータを書き込む必要がある場合があるため）。

もう一つの重要な違いは、 `info` 引数が存在しない点です。これは、このエントリーポイントが （インスタンス化や実行などの）アクションを実行する場合、その実行方法がメッセージのメタデータ - 例えば、特定のアクションを実行できるユーザーを制限したり（その際、 メッセージの `送信者` を確認することで実施します）します。クエリの場合はこのような仕組みは適用されません。クエリの目的は、単に契約の状態を何らかの形で変換して返すことにあります。この変換処理は、チェーンのメタデータに基づいて計算することが可能です（そのため、状態は「自動的に」時間の経過とともに変更される場合があります）が、メッセージ情報には影響しません。

注意していただきたいのは、エントリーポイントの引数 `msg` の型は依然として `Empty` 型であることです。これはつまり、コントラクトに送信するクエリメッセージ自体が空のJSON形式であることを意味します： `{}`

変更された最後の点は戻り値の型です。成功時には従来 `Response` 型を返していましたが、現在は任意のシリアライズ可能なオブジェクトを返すようになっています。これは、クエリが標準的なアクターモデルメッセージフローのモデル - これらのメッセージはいかなるアクションもトリガーできず、他のコントラクトとの通信もクエリ実行時以外の方法で行うことができません（この処理は `deps` 引数によって管理されます）。クエリは常にプレーンなデータを返すため、このデータは直接クエリー側に提示される必要があります。

次に実装内容を確認しましょう。特に複雑な処理は行われていません。単に返したいオブジェクトを作成し、 [`Binary`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Binary.html) 型に変換するために、 [`to_binary`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/fn.to_binary.html) 関数を使用しています。

クエリ自体は存在しますが、クエリメッセージに問題があります。常に空のJSON形式となっています。これは重大な問題です。将来的に新たなクエリを追加したい場合、各クエリタイプを合理的に区別することが困難になります。

実際の実装では、空でないクエリメッセージ型を使用することでこの問題に対処します。契約コードを改善しましょう：

```rust
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct QueryResp {
    message: String,
}

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    Greet {},
}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => {
            let resp = QueryResp {
                message: "Hello World".to_owned(),
            };

            to_binary(&resp)
        }
    }
}
```

ここでクエリメッセージ用の適切なメッセージ型を導入しました。これは [列挙型](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html) であり、デフォルトでは単一のフィールドを持つJSONとしてシリアライズされます。フィールド名は列挙型の値になります（現時点では常に「greet」となります）。また、このフィールドの値は、この列挙型のバリアントに割り当てられたオブジェクトとなります。

注意：本列挙型には唯一の `Greet` バリアントに型が割り当てられていません。Rustでは通常、バリアント名の後に追加の `{}` を指定せずにこのようなバリアントを定義します。この場合、中括弧には明確な役割があります - これを省略すると、バリアントは単に文字列としてシリアライズされてしまいます。type - このため、 `{ "greet": {} }` の代わりに、この列挙型の JSON 表現は `"greet"` となります。この動作はメッセージスキーマに一貫性の欠如をもたらします。一般的に、serde でシリアライズ可能な空の列挙型には必ず `{}` を追加することが推奨されます。これにより、JSON 表現がより適切になります。

しかし現在、コードをさらに改善できる余地があります。現在、 `query` 関数には2つの役割があります。1つ目は明白です - クエリ自体の処理です。これは最初に想定されていた機能であり、現在も存在しています。しかし新たに追加された機能として、クエリメッセージの振り分け処理が存在します。一見すると分かりにくいかもしれませんが、単一のバリアントしかない場合でも、クエリ関数は巨大な読みにくい `match` 文の温床になりがちです。コードをより [SOLID原則](https://en.wikipedia.org/wiki/SOLID) に準拠したものにするため、この部分をリファクタリングし、 `greet` メッセージの処理を別の関数に分離します。

```rust
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GreetResp {
    message: String,
}

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    Greet {},
}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
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
```

これでコードの可読性が大幅に向上しました。さらにいくつかの改善点があります。クエリ応答型 `GreetResp` の名前を変更しました。これは、異なるクエリに対して異なる応答を返す可能性があるためです。名前はメッセージ全体ではなく、特定のバリアントにのみ関連付けられるべきです。

次に、新しく作成した関数をモジュール `query` 内に配置します。これにより、名前衝突を回避しやすくなります。将来的にクエリと実行メッセージで同じバリアントを使用した場合でも、それらのハンドラは別々の名前空間に収められるため、競合が発生しません。

議論の余地がある設計として、 `StdResult` 型を `GreetResp` 型の代わりに `greet` 関数から返す方法が挙げられます。この関数では決してエラーが発生しないため、このような設計は適切かどうか疑問です。これはスタイルの問題ではありますが、私はメッセージハンドラの一貫性を重視する立場です。実際、大多数のメッセージハンドラには失敗ケースが存在します - 例えば状態を読み取る際などにエラーが発生する可能性があります。

また、すべてのクエリハンドラーに対して `deps` と `env` 引数を統一的に渡すことも検討できます。ただし、これは不要な定型コードを増加させ、コードの可読性を低下させるため、個人的にはあまり好ましくないと考えています。とはいえ、一貫性を保つという観点も理解できますので、最終的にはご自身の判断にお任せします。

ご覧の通り、当社のコントラクトは現在少しずつ規模が大きくなっています。約50行というのは一見それほど多くないように思えますが、単一のファイル内に非常に多くの異なる要素が混在しており、より適切な構造化が可能だと考えます。コード内で既に3種類の異なる要素を識別できます：エントリーポイント、メッセージ処理関数、およびハンドラーです。通常、スマートコントラクトは3つのファイルに分割して管理します。まずすべてのメッセージを `src/msg.rs` ファイルに抽出します：

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GreetResp {
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    Greet {},
}
```

`GreetResp` 構造体のフィールドを公開属性に設定したことにお気づきでしょう。これは、これらのフィールドが 異なるモジュールからアクセスされる必要があるためです。次に `src/contract.rs` ファイルに移動します：

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
```

ほとんどのロジックをここに移動したため、 `src/lib.rs` は単なる非常にシンプルなライブラリエントリとなり、 モジュール定義とエントリポイント定義以外は何も含まれていません。 `#[entry_point]` 属性を `query` 関数から `src/contract.rs` ファイルから削除しました。この属性を持つ関数は 別途用意する予定です。さらに関数の役割を明確に分離するため、 `contract::query` 関数はクエリメッセージのディスパッチを担当する最上位レベルのクエリ処理関数とし、 `query` 関数はクレートレベルでは単にエントリポイントとして機能するように変更しました。この違いは微妙なものですが、今後の開発において重要な意味を持つでしょうエントリーポイントを生成せず、代わりにディスパッチ処理関数を保持したい場合について説明します。この分割を導入したのは、典型的なコントラクト構造を示すためです。

最後に、 `src/lib.rs` ファイルについて説明します：

```rust
use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

mod contract;
mod msg;

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

単純なトップレベルモジュールです。サブモジュールとエントリーポイントの定義以外には何もありません。

契約が動作する準備が整ったところで、実際にテストを行ってみましょう。