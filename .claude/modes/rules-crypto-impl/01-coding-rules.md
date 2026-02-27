# Crypto Implementation Coding Rules

## 暗号実装のコーディング規約

このドキュメントは、FORMIX暗号システムにおける暗号実装のコーディング規約を定義します。

## 1. 基本原則

### 1.1 セキュリティファースト
```rust
// ❌ 悪い例: 秘密情報がメモリに残る
fn process_secret(secret: Vec<u8>) {
    // secretが関数終了後もメモリに残る可能性
}

// ✅ 良い例: Zeroizeで自動的にメモリをクリア
use zeroize::Zeroize;

#[derive(Zeroize)]
#[zeroize(drop)]
struct Secret(Vec<u8>);

fn process_secret(mut secret: Secret) {
    // Drop時に自動的にメモリがゼロクリアされる
}
```

### 1.2 定数時間実装
```rust
// ❌ 悪い例: タイミング攻撃に脆弱
fn compare_secrets(a: &[u8], b: &[u8]) -> bool {
    a == b  // 早期リターンによりタイミングが異なる
}

// ✅ 良い例: 定数時間比較
use subtle::ConstantTimeEq;

fn compare_secrets(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}
```

## 2. 暗号ライブラリの使用

### 2.1 Umbral-PREの使用
```rust
use umbral_pre::{
    SecretKey, PublicKey, Capsule, 
    VerifiedKeyFrag, VerifiedCapsuleFrag,
    encrypt, decrypt_original, reencrypt, decrypt_reencrypted
};

// 鍵ペア生成
pub fn generate_owner_keys() -> Result<(SecretKey, PublicKey), CryptoError> {
    let sk = SecretKey::random();
    let pk = sk.public_key();
    Ok((sk, pk))
}

// 暗号化
pub fn encrypt_data(
    plaintext: &[u8],
    public_key: &PublicKey,
) -> Result<(Vec<u8>, Capsule), CryptoError> {
    encrypt(public_key, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))
}
```

### 2.2 Shamir秘密分散
```rust
use shamir::{SecretSharing, Share};

pub fn split_secret(
    secret: &[u8],
    threshold: usize,
    shares: usize,
) -> Result<Vec<Share>, CryptoError> {
    if threshold > shares {
        return Err(CryptoError::InvalidThreshold);
    }
    
    let sharing = SecretSharing::new(threshold, shares)?;
    Ok(sharing.split(secret)?)
}
```

## 3. エラーハンドリング

### 3.1 情報を漏らさないエラー
```rust
// ❌ 悪い例: 秘密情報を含むエラー
#[derive(Debug, thiserror::Error)]
pub enum BadError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),  // 鍵の内容が漏れる
}

// ✅ 良い例: 抽象的なエラー
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key format")]
    InvalidKeyFormat,
    
    #[error("Decryption failed")]
    DecryptionFailed,
    
    #[error("Invalid threshold parameters")]
    InvalidThreshold,
}
```

### 3.2 エラーの適切な伝播
```rust
pub fn process_encrypted_data(
    capsule: &Capsule,
    cfrags: Vec<VerifiedCapsuleFrag>,
    bob_sk: &SecretKey,
) -> Result<Vec<u8>, CryptoError> {
    // エラーを適切に変換して伝播
    decrypt_reencrypted(&bob_sk, &capsule, cfrags)
        .map_err(|_| CryptoError::DecryptionFailed)
}
```

## 4. メモリ管理

### 4.1 秘密情報の構造体
```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use serde::{Serialize, Deserialize};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterSecret {
    #[zeroize(skip)]  // 公開情報はスキップ可能
    pub id: String,
    
    secret_key: Vec<u8>,
    threshold: usize,
}

// シリアライズ時の注意
#[derive(Serialize, Deserialize)]
pub struct SerializableSecret {
    #[serde(with = "base64")]
    encrypted_key: Vec<u8>,  // 必ず暗号化してからシリアライズ
}
```

### 4.2 一時的な秘密情報
```rust
pub fn temporary_secret_handling() -> Result<(), CryptoError> {
    let mut temp_secret = vec![0u8; 32];
    
    // スコープを使って確実にクリア
    {
        getrandom::getrandom(&mut temp_secret)?;
        // 秘密情報を使用
        process_secret(&temp_secret)?;
    }
    
    // 明示的にゼロクリア
    temp_secret.zeroize();
    Ok(())
}
```

## 5. 暗号パラメータ

### 5.1 定数の定義
```rust
pub mod constants {
    /// 最小閾値
    pub const MIN_THRESHOLD: usize = 2;
    
    /// 最大分散数
    pub const MAX_SHARES: usize = 20;
    
    /// 鍵サイズ（ビット）
    pub const KEY_SIZE_BITS: usize = 256;
    
    /// ナンスサイズ（バイト）
    pub const NONCE_SIZE: usize = 12;
}
```

### 5.2 パラメータ検証
```rust
pub fn validate_threshold_params(k: usize, n: usize) -> Result<(), CryptoError> {
    if k < constants::MIN_THRESHOLD {
        return Err(CryptoError::ThresholdTooLow);
    }
    
    if n > constants::MAX_SHARES {
        return Err(CryptoError::TooManyShares);
    }
    
    if k > n {
        return Err(CryptoError::InvalidThreshold);
    }
    
    Ok(())
}
```

## 6. テスト可能な設計

### 6.1 モックフレンドリーな設計
```rust
#[cfg_attr(test, mockall::automock)]
pub trait CryptoOperations {
    fn generate_keypair(&self) -> Result<(SecretKey, PublicKey), CryptoError>;
    fn encrypt(&self, pk: &PublicKey, data: &[u8]) -> Result<(Vec<u8>, Capsule), CryptoError>;
    fn decrypt(&self, sk: &SecretKey, capsule: &Capsule, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

pub struct UmbralCrypto;

impl CryptoOperations for UmbralCrypto {
    // 実装...
}
```

### 6.2 決定論的テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deterministic_keygen() {
        // 固定シードで決定論的な鍵生成
        let seed = [42u8; 32];
        let sk = SecretKey::from_seed(&seed);
        let pk = sk.public_key();
        
        // 同じシードから同じ鍵が生成される
        let sk2 = SecretKey::from_seed(&seed);
        assert_eq!(sk.to_bytes(), sk2.to_bytes());
    }
}
```

## 7. WASM互換性

### 7.1 no_std対応
```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String};

#[cfg(feature = "std")]
use std::{vec::Vec, string::String};
```

### 7.2 WASM特有の考慮
```rust
#[cfg(target_arch = "wasm32")]
pub fn get_random_bytes(size: usize) -> Result<Vec<u8>, CryptoError> {
    // WASM環境では getrandom が自動的に crypto.getRandomValues を使用
    let mut bytes = vec![0u8; size];
    getrandom::getrandom(&mut bytes)?;
    Ok(bytes)
}
```

## 8. ドキュメント

### 8.1 セキュリティ関連のドキュメント
```rust
/// 秘密鍵を生成します
/// 
/// # セキュリティ
/// 
/// - 生成された秘密鍵は使用後自動的にメモリからクリアされます
/// - 暗号学的に安全な乱数生成器を使用します
/// 
/// # エラー
/// 
/// 乱数生成に失敗した場合、`CryptoError::RandomGenerationFailed`を返します
pub fn generate_secret_key() -> Result<SecretKey, CryptoError> {
    // 実装
}
```

## 9. 監査とレビュー

### 9.1 セキュリティアノテーション
```rust
/// SECURITY: この関数は定数時間で実行される必要があります
#[must_use = "比較結果を確認してください"]
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}
```

### 9.2 安全でない操作の明示
```rust
/// # Safety
/// 
/// この関数は以下の条件を満たす場合のみ安全です：
/// - `ptr`は有効な`size`バイトのメモリを指している
/// - メモリ領域は他のスレッドからアクセスされない
pub unsafe fn zero_memory(ptr: *mut u8, size: usize) {
    std::ptr::write_bytes(ptr, 0, size);
}
```

## 10. 共通パターン

### 10.1 暗号コンテキスト
```rust
pub struct CryptoContext {
    owner_keys: (SecretKey, PublicKey),
    threshold_params: (usize, usize),
    phase: CryptoPhase,
}

impl CryptoContext {
    pub fn new(k: usize, n: usize) -> Result<Self, CryptoError> {
        validate_threshold_params(k, n)?;
        let owner_keys = generate_owner_keys()?;
        
        Ok(Self {
            owner_keys,
            threshold_params: (k, n),
            phase: CryptoPhase::Initialized,
        })
    }
}
```

これらの規約に従うことで、セキュアで保守性の高い暗号実装を実現できます。