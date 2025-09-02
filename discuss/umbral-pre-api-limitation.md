# umbral-pre ライブラリのAPI制限と回避策

## 概要

D-TPRES の `CryptoService` 実装において、umbral-pre ライブラリの API 制限により、秘密鍵のデシリアライズ処理で一時的な秘密鍵生成が必要となっている。この文書では、その技術的背景と現在の回避策、そして将来的な改善案について記述する。

## 問題の詳細

### 1. umbral-pre の内部構造

umbral-pre ライブラリは、秘密鍵を以下のような構造で管理している：

```rust
pub struct SecretKey {
    inner: SecretBox<GenericArray<u8, U32>>  // プライベートフィールド
}

pub struct SecretBox<T> {
    inner: T  // プライベートフィールド
}
```

`SecretBox` は秘密情報を安全に管理するためのラッパー型だが、そのコンストラクタは公開されていない。

### 2. API の制限

秘密鍵をバイト配列から復元するには、以下の関数を使用する必要がある：

```rust
pub fn try_from_be_bytes(
    bytes: &SecretBox<GenericArray<u8, U32>>
) -> Result<SecretKey, String>
```

しかし、`SecretBox` を直接作成する方法が提供されていない：
- ❌ `SecretBox::new()` - 存在しない
- ❌ `SecretBox::from()` - 実装されていない
- ❌ `SecretKey::from_bytes()` - 存在しない

## 現在の回避策

`src/service/core/crypto.rs` の `deserialize_secret_key` メソッドでは、以下の回避策を実装している：

```rust
fn deserialize_secret_key(&self, key: &SecretKey) -> ServiceResult<umbral_pre::SecretKey> {
    // Step 1: ランダムな秘密鍵を生成（SecretBoxを得る唯一の方法）
    let temp_sk = umbral_pre::SecretKey::random();
    
    // Step 2: そのSecretBoxを取得
    let mut secret_bytes = temp_sk.to_be_bytes();
    
    // Step 3: 内容を実際のデータで上書き
    secret_bytes.as_mut_secret().copy_from_slice(&key.key_data);
    
    // Step 4: 新しいSecretKeyとして再構築
    umbral_pre::SecretKey::try_from_be_bytes(&secret_bytes)
}
```

### 処理フロー図

```
┌─────────────────┐
│ key.key_data    │ ← 復元したい実際の秘密鍵データ（32バイト）
└─────────────────┘
        ↓
    直接変換不可能！
        ↓
┌─────────────────┐
│ temp_sk         │ ← ランダムな秘密鍵を生成（本来不要）
│ (random key)    │
└─────────────────┘
        ↓
┌─────────────────┐
│ secret_bytes    │ ← SecretBox<GenericArray<u8, U32>>型を取得
│ (SecretBox)     │
└─────────────────┘
        ↓
    内容を上書き
        ↓
┌─────────────────┐
│ secret_bytes    │ ← 実際のデータに置き換え
│ (actual data)   │
└─────────────────┘
        ↓
┌─────────────────┐
│ umbral SecretKey│ ← 最終的な秘密鍵
└─────────────────┘
```

## セキュリティ上の考慮事項

### リスク

1. **一時的なメモリ露出**
   - ランダムな秘密鍵が一時的にメモリに存在
   - 上書き前の値がメモリに残る可能性

2. **パフォーマンスへの影響**
   - 不要なランダム数生成のオーバーヘッド
   - 余分なメモリアロケーション

### 緩和策

現在の実装では、以下の理由により実用上のリスクは最小限：

1. **Zeroize による自動クリーンアップ**
   - `temp_sk` と `secret_bytes` は両方 `Zeroize` トレイトを実装
   - スコープ外で自動的にメモリがゼロクリア

2. **短いライフタイム**
   - 関数内のローカルスコープ
   - 即座に上書きされる

3. **ランダム値の特性**
   - 元の値は意味のないランダムデータ
   - 攻撃者が取得しても有用な情報ではない

## 理想的な解決策

### 短期的改善案

umbral-pre ライブラリに以下のようなAPIを追加することを提案：

```rust
// 案1: SecretBoxの公開コンストラクタ
impl<T> SecretBox<T> {
    pub fn from_slice(data: &[u8]) -> Self where T: From<&[u8]> {
        // 実装
    }
}

// 案2: SecretKeyの直接的なコンストラクタ
impl SecretKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, Error> {
        // 実装
    }
}
```

### 長期的改善案

1. **カスタムラッパーの実装**
   ```rust
   pub struct SecureSecretKey {
       inner: Vec<u8>,  // 暗号化された状態で保持
   }
   
   impl SecureSecretKey {
       fn to_umbral_key(&self) -> umbral_pre::SecretKey {
           // 必要な時だけ復号化
       }
   }
   ```

2. **代替ライブラリの検討**
   - より柔軟な API を提供する Proxy Re-Encryption ライブラリの調査
   - 独自実装の検討（セキュリティ監査が必要）

## アクションアイテム

- [ ] umbral-pre リポジトリに Issue を作成し、API 改善を提案
- [ ] 一時的な回避策のパフォーマンス影響を測定
- [ ] 代替ライブラリの調査と評価
- [ ] セキュリティ監査の実施

## 関連ファイル

- `src/service/core/crypto.rs` - 実装箇所
- `CLAUDE.md` - セキュリティガイドライン
- `.claude/modes/rules-crypto-impl/` - 暗号実装ルール

## 更新履歴

- 2025-01-02: 初版作成
- 問題の詳細と現在の回避策を文書化
- セキュリティ上の考慮事項を記載