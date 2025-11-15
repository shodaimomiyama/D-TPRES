---
title: "Map storage - CosmWasm book"
source: "https://book.cosmwasm.com/cross-contract/map-storage.html"
author:
published:
created: 2025-11-11
description: "Guide to building CosmWasm smart contracts"
tags:
  - "clippings"
status: "unread"
---
管理者コントラクトには直ちに改善すべき点が1つあります。コントラクトの状態を確認してみましょう：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ADMINS: Item<Vec<Addr>> = Item::new("admins");
pub const DONATION_DENOM: Item<String> = Item::new("donation_denom");
}
```

注意していただきたいのは、管理者リストを単一のベクターとして保持している点です。しかし、契約全体を通して、このベクターの特定の要素にのみアクセスするケースがほとんどなのです。

これは理想的とは言えません。現在、単一の管理者エントリにアクセスする必要がある場合、まずすべてのエントリを含むリストをデシリアライズし、その中から目的のエントリを見つけるまで反復処理を行う必要があります。この処理にはかなりの量のガス消費量が大幅に増加し、まったく不要なオーバーヘッドが生じます。 [Map](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html) ストレージアクセサーを使用すれば、この問題を回避できます。

まず、マップを定義しましょう。この文脈では、キーとそれに関連付けられた値の集合を指し、Rust の `HashMap` や多くのプログラミング言語の辞書構造と同様のものです。 `Item` と似た構造として定義しますが、今回は2つの型が必要です：キーの型と値の型です：

```rust
#![allow(unused)]
fn main() {
use cw_storage_plus::Map;

pub const STR_TO_INT_MAP: Map<String, u64> = Map::new("str_to_int_map");
}
```

[`Map`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html) にアイテムを登録するには、 [`save`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.save) メソッドを使用します。これは `Item` の場合と同様です：

```rust
#![allow(unused)]
fn main() {
STR_TO_INT_MAP.save(deps.storage, "ten".to_owned(), 10);
STR_TO_INT_MAP.save(deps.storage, "one".to_owned(), 1);
}
```

マップ内のエントリにアクセスする方法も、アイテムを読み込むのと同じくらい簡単です：

```rust
#![allow(unused)]
fn main() {
let ten = STR_TO_INT_MAP.load(deps.storage, "ten".to_owned())?;
assert_eq!(ten, 10);

let two = STR_TO_INT_MAP.may_load(deps.storage, "two".to_owned())?;
assert_eq!(two, None);
}
```

当然ながら、マップ内に要素が存在しない場合、 [`load`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.load) 関数はエラーを返します。これはアイテムの場合と同様です。一方、 [`may_load`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.may_load) 関数は、要素が存在する場合には `Some` 型を返します。

マップ専用の非常に便利なアクセッサとして、 [`has`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.has) 関数があります。これはマップ内に特定のキーが存在するかどうかを確認するための機能です：

```rust
#![allow(unused)]
fn main() {
let contains = STR_TO_INT_MAP.has(deps.storage, "three".to_owned())?;
assert!(!contains);
}
```

最後に、マップの要素（キーまたはキーと値のペア）を反復処理する方法について説明します：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_std::Order;

for k in STR_TO_INT_MAP.keys(deps.storage, None, None, Order::Ascending) {
    let _addr = deps.api.addr_validate(k?);
}

for item in STR_TO_INT_MAP.range(deps.storage, None, None, Order::Ascending) {
    let (_key, _value) = item?;
}
}
```

まず疑問に思うかもしれないのが、 [`keys`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.keys) および [`range`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.range) 関数に渡される追加の値についてです。これらはそれぞれ、反復処理対象の要素の下限値と上限値、および要素を辿る順序を指定するものです。

通常のRustイテレータを使用する場合、まずすべての要素に対するイテレータを作成し、 次に興味のない要素を何らかの方法でスキップします。その後、最後に興味のある要素で 処理を終了することになります。

通常の場合、フィルタリング対象の要素にもアクセスする必要が生じ、これが問題となります。なぜなら、その要素をストレージから読み込まなければならないからです。ストレージからのデータ読み込みは、可能な限り回避すべきコストのかかる処理なのです。一つの方法として、Mapに対してストレージからの要素のデシリアライズを開始する位置と終了位置を指定する方法があります。これにより、指定した範囲外の要素にアクセスすることがなくなります。

もう一つ重要な点として、keys() と range() 関数が返すイテレータは要素のイテレータではなく、 `Result` 型のイテレータであるという点が挙げられます。これはごく稀なケースですが、特定の項目が存在すべき場合に備えてこのような設計になっています。ただし、ストレージからの読み取り時に何らかのエラーが発生する可能性があります。例えば、保存されている値が 想定していない形式でシリアライズされている場合、デシリアライズ処理が失敗することがあります。これは実際に私が過去に関わったコントラクトの一つで発生した問題です - 私たちはマップの値型を変更したものの、マイグレーション処理を忘れていたために、 さまざまな問題が発生しました。

ここで私が「おかしい」と思われるかもしれませんが、 `Map` について繰り返し説明している理由をご説明しましょう。私たちは現在ベクターを扱っているのに、なぜこのような話をしているのでしょうか？ 明らかに、この2つは全く異なる概念です！ それとも違うのでしょうか？

`ADMINS` ベクターに格納しているデータについて改めて考えてみましょう。ここには一意性が保証されるべきオブジェクトのリストが含まれており、これは数学的な集合の定義そのものです。そこで改めて、マップの基本的な定義を説明します：

> まずマップを定義しましょう。この文脈では、 *集合* としてのキーの集まりであり、各キーに値が割り当てられている状態を指します。これはRustのHashMapや多くのプログラミング言語における辞書構造と同様の概念です。

ここであえて「集合」という言葉を使ったのには理由があります。マップには集合が組み込まれているからです。 これは集合の一般化、あるいは論理の逆転とも言えます - 集合はマップの特殊なケースなのです。 もしマップがあらゆるキーを同一の値にマッピングする集合であると想像するなら、値は無関係となり、このようなマップは実質的に集合として機能します。

すべてのキーを同一の値にマッピングするマップを作成するにはどうすればよいでしょうか？単一の値を持つ型を選択します。Rustでは通常、ユニット型（ `()` ）が使用されますが、CosmWasmではCW標準ライブラリの [`Empty`](https://docs.rs/cosmwasm-std/1.2.4/cosmwasm_std/struct.Empty.html) 型を使用するのが一般的です：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

pub const ADMINS: Map<Addr, Empty> = Map::new("admins");
}
```

次に、コントラクト内でのマップの使用方法を修正する必要があります。まずはコントラクトの初期化処理から始めましょう：

```rust
#![allow(unused)]
fn main() {
use crate::msg::InstantiateMsg;
use crate::state::{ADMINS, DONATION_DENOM};
use cosmwasm_std::{
    DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for addr in msg.admins {
        let admin = deps.api.addr_validate(&addr)?;
        ADMINS.save(deps.storage, admin, &Empty {})?;
    }
    DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;

    Ok(Response::new())
}
}
```

特に簡素化されたわけではありませんが、これでアドレスを収集する必要がなくなりました。次に、離脱時の処理ロジックに移りましょう：

```rust
#![allow(unused)]
fn main() {
use crate::state::ADMINS;
use cosmwasm_std::{DepsMut, MessageInfo};

pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    ADMINS.remove(deps.storage, info.sender.clone());

    let resp = Response::new()
        .add_attribute("action", "leave")
        .add_attribute("sender", info.sender.as_str());

    Ok(resp)
}
}
```

ここで注目すべき違いは、ベクトル全体を読み込む必要がない点です。代わりに、 [`remove`](https://docs.rs/cw-storage-plus/1.0.1/cw_storage_plus/struct.Map.html#method.remove) 関数を使用して単一の要素を削除します。

これまで強調してこなかった重要なポイントですが、 `Map` は各キーをそれぞれ独立した要素として保持します。このため、単一の要素にアクセスする場合、ベクターを使用するよりも効率的に処理できます。

ただし、これには重大な欠点があります。Mapを使用してすべての要素にアクセスすると、 ガス消費量が大幅に増加します。一般的に、このような状況は可能な限り避けるべきです。 契約の線形的な複雑さは、非常にコストの高い実行（ガス使用量の面で）や、潜在的な脆弱性 を引き起こす可能性があります。ユーザーが巧妙な方法でこのベクトルに多数のダミー要素が含まれている場合、ユーザーは実行コストをガス制限を超えるレベルまで引き上げられる可能性があります。

残念ながら、この契約にはこのような反復処理が含まれており、資金分配の流れは以下のようになります：

```rust
#![allow(unused)]
fn main() {
use crate::error::ContractError;
use crate::state::{ADMINS, DONATION_DENOM};
use cosmwasm_std::{
    coins, BankMsg,DepsMut, MessageInfo, Order, Response
};

pub fn donate(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let denom = DONATION_DENOM.load(deps.storage)?;
    let admins: Result<Vec<_>, _> = ADMINS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let admins = admins?;

    let donation = cw_utils::must_pay(&info, &denom)?.u128();

    let donation_per_admin = donation / (admins.len() as u128);

    let messages = admins.into_iter().map(|admin| BankMsg::Send {
        to_address: admin.to_string(),
        amount: coins(donation_per_admin, &denom),
    });

    let resp = Response::new()
        .add_messages(messages)
        .add_attribute("action", "donate")
        .add_attribute("amount", donation.to_string())
        .add_attribute("per_admin", donation_per_admin.to_string());

    Ok(resp)
}
}
```

もし私がこのようなコントラクトを書く必要があり、 `donate` 関数が重要な処理で、頻繁に呼び出されるものであれば、 `Item<Vec<Addr>>` の使用を強く推奨します。 幸いなことに、今回のケースではその必要はありません。分配処理は必ずしも線形的な複雑さを持つ必要はないのです！すべての受取人に対して資金を分配するために全受信者を反復処理する必要があると聞くと、一見非合理的に思えるかもしれません。しかし実際には、一定時間で処理可能な非常に効率的な方法が存在します。この方法については本書の後半で詳しく説明します。現時点では、この問題は現状のままとし、後ほど契約の不具合を修正する方法について説明します。

最後に修正が必要な関数は `admins_list` クエリハンドラーです：

```rust
#![allow(unused)]
fn main() {
use crate::state::ADMINS;
use cosmwasm_std::{Deps, Order, StdResult};

pub fn admins_list(deps: Deps) -> StdResult<AdminsListResp> {
    let admins: Result<Vec<_>, _> = ADMINS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let admins = admins?;
    let resp = AdminsListResp { admins };
    Ok(resp)
}
}
```

ここでも線形的な計算量の問題がありますが、これは比較的軽微な問題です。

まず、クエリは通常、ガスコストなしでローカルノード上で呼び出すことを想定しています - 契約に対して何度でもクエリを実行することが可能です。

さらに、実行時間やコストに制限がある場合でも、毎回すべての項目をクエリする必要はありません。この関数は後ほど修正し、ページネーション機能を追加します。これにより、クエリを実行する際の実行時間とコストを制限できるようになります。指定されたアイテムから一定の数の項目を取得する機能です。本章の内容を理解すれば、おそらくご自身で実装方法を考案できると思いますが、一般的な Cosmos スマートコントラクトの実装手法について説明する際に、この機能の標準的な実装方法についても解説します。

マップの使用方法を改善するために、1つの微妙な点を考慮する必要があります。

現在の実装では、マップのインデックスとして所有アドレス（Addr）を使用しています。このため、同じキーを再利用する場合（特に「leave」実装時）にはキーのクローン作成が必要になります。これは大きなオーバーヘッドではありませんが、回避可能な問題です。具体的には、マップのキーを参照型として定義することで解決できます：

```rust
#![allow(unused)]
fn main() {
use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

pub const ADMINS: Map<&Addr, Empty> = Map::new("admins");
pub const DONATION_DENOM: Item<String> = Item::new("donation_denom");
}
```

最後に、マップの使用箇所を2か所修正する必要があります：

```rust
#![allow(unused)]
fn main() {
use crate::state::{ADMINS, DONATION_DENOM};
use cosmwasm_std::{
    DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for addr in msg.admins {
        let admin = deps.api.addr_validate(&addr)?;
        ADMINS.save(deps.storage, &admin, &Empty {})?;
    }

    // ...

   DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;

   Ok(Response::new())
}

pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    ADMINS.remove(deps.storage, &info.sender);

    // ...

   let resp = Response::new()
       .add_attribute("action", "leave")
       .add_attribute("sender", info.sender.as_str());

   Ok(resp)
}
}
```