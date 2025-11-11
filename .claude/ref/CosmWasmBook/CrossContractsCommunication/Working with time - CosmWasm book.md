---
title: "Working with time - CosmWasm book"
source: "https://book.cosmwasm.com/cross-contract/working-with-time.html"
author:
published:
created: 2025-11-11
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
ブロックチェーンにおける時間の概念は複雑です。分散システム全般に言えることですが、すべてのノードの時計を同期させることは容易ではありません。

ただし、ブロックチェーンには「単調増加」する時間の概念が存在します。これはつまり、処理の実行間で「逆戻り」することが決して許されないということです。さらに重要な点として、時間はトランザクション全体を通じて常に一意であり、複数のトランザクションで構成されるブロック全体においても同様です。

時間情報は [`Env`](https://docs.rs/cosmwasm-std/1.2.4/cosmwasm_std/struct.Env.html) 型の [`block`](https://docs.rs/cosmwasm-std/1.2.4/cosmwasm_std/struct.BlockInfo.html) フィールドにエンコードされており、その形式は以下の通りです：

```rust
#![allow(unused)]
fn main() {
pub struct BlockInfo {
    pub height: u64,
    pub time: Timestamp,
    pub chain_id: String,
}
}
```

`time` フィールドには、処理されたブロックのタイムスタンプが記録されています。 `height` フィールドにも注目する価値があります。このフィールドには処理されたブロックのシーケンス番号が含まれています。この `height` フィールドは時間よりも有用な場合があります。なぜなら、 `height` フィールドは確実に増加することが保証されているからです。ブロック間の時間間隔について言及しています。ただし、同じ `time` 値を持つ2つのブロックが同時に処理される可能性もあります（ただしこれは比較的稀なケースです）。

また、1つのブロック内で複数のトランザクションが実行される場合があります。 つまり、特定のメッセージの実行に対して一意のIDが必要な場合、より適切な方法を検討する必要があります。 この目的で使用するのが、 `Env` 型の `transaction` フィールドです：

```rust
#![allow(unused)]
fn main() {
pub struct TransactionInfo {
    pub index: u32,
}
}
```

ここでの `index` は、ブロック内におけるトランザクションの一意のインデックス番号です。つまり、ブロック全体でトランザクションを一意に識別するには、 `(高さ, トランザクションインデックス)` の組み合わせを使用できます。

システム内の時間を利用して、管理者の加入時刻を管理したいと考えています。現時点ではグループに新たなメンバーを追加する予定はありませんが、初期管理者の加入時刻はすでに設定可能です。それでは、状態の更新を開始しましょう：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::Map;
use cw_storage_plus::Item;

pub const ADMINS: Map<&Addr, Timestamp> = Map::new("admins");
pub const DONATION_DENOM: Item<String> = Item::new("donation_denom");
}
```

ご覧のとおり、管理者リストは適切なマップ形式になりました。これから、各管理者に対して加入日時を割り当てていきます。

ここでマップの初期化方法を更新する必要があります。これまでは `Empty` データを格納していましたが、これはもはや値型と一致しません。更新された初期化関数を確認してみましょう：

このマップの値用に別の構造体を定義すべきだと主張する方もいるかもしれません。そうすれば、将来的に追加が必要になった場合でも対応できます。しかし私の意見では、これは時期尚早です。なぜなら、将来的に値の型自体を変更する可能性もあり、その場合は同様に互換性を損なう変更となってしまうからです。

現在、マップの初期化方法を更新する必要があります。これまでは `Empty` データを保存していましたが、これはもはや値型と一致しません。更新された初期化関数を確認してみましょう：

```rust
#![allow(unused)]
fn main() {
use crate::state::{ADMINS, DONATION_DENOM};
use cosmwasm_std::{
    DepsMut, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for addr in msg.admins {
        let admin = deps.api.addr_validate(&addr)?;
        ADMINS.save(deps.storage, &admin, &env.block.time)?;
    }
    DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;

    Ok(Response::new())
}
}
```

管理者値として `&Empty {}` を格納する代わりに、 `&env.block.time` から読み取った参加時刻を保存します。また、 `env` ブロックの名前からアンダースコアを削除したことにご注意ください。これは元々、Rust コンパイラに対してこの変数が意図的に未使用であり、単なるバグではないことを明示するためだけに付けられていたものでした。

最後に、プロジェクト内の使用されていない `Empty` インポートをすべて削除することを忘れないでください。コンパイラが未使用のインポートを指摘してくれるはずです。

ジョイン時間に関する最後の追加機能として、特定の管理者のジョイン時間を問い合わせる新しいクエリを実装します。必要な処理はすでに説明済みですので、演習としてご自身で考えてください。クエリの実装例は以下の通りです：

```rust
#![allow(unused)]
fn main() {
#[returns(JoinTimeResp)]
JoinTime { admin: String },
}
```

そして応答型のサンプルは以下の通りです：

```rust
#![allow(unused)]
fn main() {
#[cw_serde]
pub struct JoinTimeResp {
    pub joined: Timestamp,
}
}
```

応答型の仕様に関して疑問に思われるかもしれませんが、常に `joined` 値を返すことを推奨しています。しかし、該当する管理者が追加されていない場合はどうすればよいでしょうか？このような場合、 `load` 関数がエラーの詳細な説明を返すという仕様を活用できます。ストレージに値が存在しない場合 - ただし、このようなケースに対して独自のエラー定義を定義したり、 `joined` フィールドをオプションとして扱い、要求された管理者が存在する場合にのみ返すように設計することも可能です。

最後に、新機能のテストを作成することをお勧めします。インスタンス化直後に新しいクエリを実行し、初期管理者の参加時間が適切に設定されていることを確認してください（既存のインスタンス化テストを拡張する方法もあります）。

テスト環境で実行時刻を取得する方法について、サポートが必要になる場合があります。OSのシステム時刻を使用する方法は信頼性に欠けるため、代わりに [`block_info`](https://docs.rs/cw-multi-test/0.16.4/cw_multi_test/struct.App.html#method.block_infohttps://docs.rs/cw-multi-test/0.16.4/cw_multi_test/struct.App.html#method.block_info) 関数を呼び出して [`BlockInfo`](https://docs.rs/cosmwasm-std/latest/cosmwasm_std/struct.BlockInfo.html) 構造体を取得してください。この構造体には、アプリケーションの特定時点におけるブロック状態が含まれています。具体的には、インスタンス化の直前にこれを行うことで、呼び出し時にシミュレートされるのと同じ状態を確実に操作できるようになります。