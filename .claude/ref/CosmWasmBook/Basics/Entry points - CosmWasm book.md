---
title: "Entry points - CosmWasm book"
source: "https://book.cosmwasm.com/basics/entry-points.html"
author:
published:
created: 2025-11-05
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
一般的なRustアプリケーションは、オペレーティングシステムによって呼び出される `fn main()` 関数から始まります。 スマートコントラクトも基本的な仕組みは同様です。コントラクトにメッセージが送信されると、 「エントリーポイント」と呼ばれる関数が呼び出されます。ネイティブアプリケーションが単一の `main` エントリーポイントしか持たないのに対し、スマートコントラクトには異なるメッセージタイプに対応する複数のエントリーポイントが存在します： `instantiate` 、 `execute` 、 `query` 、 `sudo` 、 `migrate` などです。

まず、基本的な3つのエントリーポイントについて説明します：

- `instantiate` 関数は、スマートコントラクトのライフサイクル中に1度だけ呼び出されます。これは コントラクトのコンストラクタまたは初期化関数と考えることができます。
- `execute` はスマートコントラクトの状態を変更可能なメッセージを処理するためのエントリーポイントです。これらは 実際の処理を実行するために使用されます。
- `query` はコントラクトから特定の情報を取得するメッセージを扱うためのものです。 `execute` とは異なり、 これらのメッセージはコントラクトの状態を変更することはできず、データベースクエリと同様に使用されます。

`src/lib.rs` ファイルに移動し、 `instantiate` エントリーポイントから始めましょう：

```rust
use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}
```

実際、 `instantiate` はスマートコントラクトを有効化するために必須のエントリーポイントです。この形式自体はあまり実用的ではありませんが、出発点としては適しています。では、このエントリーポイントの構造を詳しく見ていきましょう。

まず、より一貫した使用のためにいくつかの型をインポートします。その後、エントリーポイントを定義します。 `instantiate` 関数は4つの引数を取ります：

- [`deps: DepsMut`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.DepsMut.html) は、外部システムとの通信を行うためのユーティリティ型です。コントラクト状態の照会・更新、他のコントラクト状態の取得、およびCWアドレスを扱うための補助関数群を含む `Api` オブジェクトへのアクセスを提供します。
- [`env: Env`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Env.html) は、メッセージ実行時のブロックチェーン状態を表すオブジェクトです。具体的には、 チェーンの高さとID、現在のタイムスタンプ、および呼び出されたコントラクトアドレスが含まれます。
- [`info: MessageInfo`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.MessageInfo.html) には、実行を引き起こしたメッセージに関するメタ情報が含まれています - 具体的には、 メッセージ送信元のアドレスと、メッセージと共に送信されたチェーン固有のトークン情報です。
- [`msg: Empty`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Empty.html) は実行トリガーとなるメッセージそのものです。現時点では `Empty` 型を使用しており、これは JSON 形式の `{}` を表しますが、この引数の型はデシリアライズ可能な任意の型を使用でき、将来的にはより複雑な型も扱えるようになります。

ブロックチェーン技術に不慣れな方にとって、これらの引数は最初は意味が分かりにくいかもしれませんが、 このガイドを読み進めるにつれて、一つずつ丁寧に解説していきます。

エントリーポイントに付与されている重要な属性 [`#[entry_point]`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/attr.entry_point.html) に注目してください。この属性の目的は、 エントリーポイント全体をWasmランタイムが理解できる形式に変換することです。適切なWasmエントリーポイントでは、 Wasm仕様でネイティブにサポートされている基本型のみを使用する必要があり、Rustの構造体や列挙型はこれらの型はこのセットに含まれていません。このようなエントリポイントを扱う場合、非常に複雑になるため、CosmWasm の開発者は `entry_point` マクロを提供しています。このマクロは生のWasmエントリポイントを生成し、内部で装飾された関数を呼び出すとともに、Rustの高レベルな引数を処理するために必要なすべての処理を自動的に行います。Wasmランタイムから渡される引数を処理します。

次に注目すべきは戻り値の型です。この簡単な例では [`StdResult<Response>`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/type.StdResult.html) を使用していますが、これは `Result<Response, StdError>` のエイリアスです。戻り値の型は常に [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html) 型となり、エラー型は [`ToString`](https://doc.rust-lang.org/std/string/trait.ToString.html) トレイトを実装し、成功時の型は明確に定義されている必要があります。ほとんどのエントリーポイントにおいて、「成功」ケースは [`Response`](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Response.html) 型で表現されます。この型は、コントラクトを 私たちのアクターモデルに適合させるためのもので、これについては間もなく詳しく説明します。

エントリーポイントの本体は非常にシンプルで、常に単純な空のレスポンスで成功するように設計されています。