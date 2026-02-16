# リポジトリインターフェース

`client/src/repositories/`

## ベースリポジトリトレイト

```rust
#[async_trait]
pub trait Repository<T, ID>: Send + Sync {
    async fn save(&self, entity: &T) -> DomainResult<()>;
    async fn find_by_id(&self, id: &ID) -> DomainResult<Option<T>>;
    async fn delete(&self, id: &ID) -> DomainResult<()>;
    async fn exists(&self, id: &ID) -> DomainResult<bool>;
    async fn find_by_ids(&self, ids: &[ID]) -> DomainResult<Vec<T>>;
}
```

**WASM互換性:** `cfg`によるデュアル実装:

| ターゲット | トレイト境界 |
|-----------|------------|
| `not(wasm32)` | `Send + Sync` |
| `wasm32` | `#[async_trait(?Send)]` |

## 特化リポジトリトレイト

### SecretRepository (`secret_interface.rs`)

`Repository<Secret, SecretId>`を拡張。追加メソッドなし。

### CapsuleRepository (`capsule_interface.rs`)

`Repository<Capsule, CapsuleId>`を拡張。

| メソッド | 戻り値 | 説明 |
|---------|--------|------|
| `find_by_secret_id(&self, secret_id)` | `Option<Capsule>` | 指定Secretに対応するCapsule |

### KFragRepository (`kfrag_interface.rs`)

`Repository<KFrag, KFragId>`を拡張。

| メソッド | 戻り値 | 説明 |
|---------|--------|------|
| `find_by_secret_id(&self, secret_id)` | `Vec<KFrag>` | 指定Secretの全KFrag |
| `find_by_holder_index(&self, secret_id, holder_index)` | `Option<KFrag>` | ホルダーインデックスでKFragを検索 |
| `delete_by_secret_id(&self, secret_id)` | `()` | 一括削除 |

### CFragRepository (`cfrag_interface.rs`)

`Repository<CFrag, CFragId>`を拡張。

| メソッド | 戻り値 | 説明 |
|---------|--------|------|
| `find_by_secret_id(&self, secret_id)` | `Vec<CFrag>` | 指定Secretの全CFrag |
| `find_by_kfrag_id(&self, kfrag_id)` | `Option<CFrag>` | ソースKFragでCFragを検索 |
| `delete_by_secret_id(&self, secret_id)` | `()` | 一括削除 |
| `count_by_secret_id(&self, secret_id)` | `usize` | 指定SecretのCFrag数を取得 |

### ShareCollectionRepository (`share_interface.rs`)

`Repository<ShareCollection, ShareCollectionId>`を拡張。

| メソッド | 戻り値 | 説明 |
|---------|--------|------|
| `find_by_secret_id(&self, secret_id)` | `Option<ShareCollection>` | 指定Secretに対応するShareCollection |

## 設計原則

- **依存性逆転:** ドメインがインターフェースを定義し、Adapterが実装を提供
- **非同期:** すべてのメソッドがasync（ブラウザ/ネットワーク互換性）
- **冪等な削除:** 存在しないエンティティの削除はエラーにならない
- **Save = upsert:** 作成または更新
- **すべて`DomainResult<T>`を返す:** `DomainError`による統一的なエラーハンドリング

## 実装

Arweaveバックエンドのリポジトリ実装については[Adapter層](adapter.md)を参照。
