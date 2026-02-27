# WASM Build Coding Rules

## WASM ビルド用コーディング規約

このドキュメントは、FORMIX暗号システムのWASMビルドにおけるコーディング規約を定義します。

## 1. 基本原則

### 1.1 no_std First
```rust
// ファイル先頭での宣言
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

// 条件付きインポート
#[cfg(not(feature = "std"))]
use alloc::{
    vec::Vec,
    string::{String, ToString},
    collections::BTreeMap,
    boxed::Box,
};

#[cfg(feature = "std")]
use std::{
    vec::Vec,
    string::{String, ToString},
    collections::HashMap,
};
```

### 1.2 プラットフォーム特化コード
```rust
// WASM固有の実装
#[cfg(target_arch = "wasm32")]
pub fn wasm_specific_function() {
    // WASM環境でのみ動作する処理
}

// 非WASM環境
#[cfg(not(target_arch = "wasm32"))]
pub fn native_specific_function() {
    // ネイティブ環境での処理
}
```

## 2. メモリ管理

### 2.1 アロケータ設定
```rust
// Cargo.toml
[dependencies]
wee_alloc = { version = "0.4", optional = true }

// lib.rs
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

### 2.2 メモリ効率的なデータ構造
```rust
// ❌ 悪い例: 大きなメモリ使用
use std::collections::HashMap;

struct LargeData {
    data: HashMap<String, Vec<u8>>,  // メモリ使用量が大きい
}

// ✅ 良い例: 効率的なメモリ使用
use heapless::FnvIndexMap;

struct CompactData {
    data: FnvIndexMap<u32, [u8; 32], 64>,  // 固定サイズ、メモリ効率
}
```

### 2.3 バッファサイズ制限
```rust
pub mod limits {
    #[cfg(target_arch = "wasm32")]
    pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;  // 64KB
    
    #[cfg(not(target_arch = "wasm32"))]
    pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;  // 1MB
    
    pub const MAX_CONCURRENT_OPERATIONS: usize = 10;
}

// 使用例
pub fn process_message(data: &[u8]) -> Result<Vec<u8>, ProcessError> {
    if data.len() > limits::MAX_MESSAGE_SIZE {
        return Err(ProcessError::MessageTooLarge);
    }
    // 処理続行...
    Ok(data.to_vec())
}
```

## 3. エラーハンドリング

### 3.1 パニック回避
```rust
// ❌ 悪い例: パニックを起こす可能性
fn divide(a: u32, b: u32) -> u32 {
    a / b  // b=0でパニック
}

// ✅ 良い例: Resultを使用
fn safe_divide(a: u32, b: u32) -> Result<u32, ArithmeticError> {
    if b == 0 {
        Err(ArithmeticError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}
```

### 3.2 WASM用エラー型
```rust
#[derive(Debug, Clone)]
pub enum WasmError {
    InvalidInput,
    InsufficientMemory,
    OperationTimeout,
    SerializationFailed,
}

// no_std環境でのエラー実装
#[cfg(not(feature = "std"))]
impl core::fmt::Display for WasmError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            WasmError::InvalidInput => write!(f, "Invalid input"),
            WasmError::InsufficientMemory => write!(f, "Insufficient memory"),
            WasmError::OperationTimeout => write!(f, "Operation timeout"),
            WasmError::SerializationFailed => write!(f, "Serialization failed"),
        }
    }
}
```

## 4. シリアライゼーション

### 4.1 効率的なシリアライゼーション
```rust
use serde::{Serialize, Deserialize};

// バイナリシリアライゼーション（サイズ効率）
#[derive(Serialize, Deserialize)]
pub struct CompactMessage {
    #[serde(with = "serde_bytes")]
    payload: Vec<u8>,
    
    timestamp: u64,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<MessageMetadata>,
}

// シリアライゼーション関数
pub fn serialize_compact<T: Serialize>(value: &T) -> Result<Vec<u8>, WasmError> {
    #[cfg(feature = "std")]
    {
        bincode::serialize(value).map_err(|_| WasmError::SerializationFailed)
    }
    
    #[cfg(not(feature = "std"))]
    {
        postcard::to_allocvec(value).map_err(|_| WasmError::SerializationFailed)
    }
}
```

### 4.2 JSONシリアライゼーション（デバッグ用）
```rust
#[cfg(feature = "json")]
pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, WasmError> {
    serde_json::to_string(value)
        .map_err(|_| WasmError::SerializationFailed)
}
```

## 5. 暗号機能のWASM対応

### 5.1 乱数生成
```rust
use getrandom::getrandom;

pub fn secure_random_bytes(size: usize) -> Result<Vec<u8>, WasmError> {
    let mut buffer = vec![0u8; size];
    getrandom(&mut buffer)
        .map_err(|_| WasmError::InsufficientMemory)?;
    Ok(buffer)
}

// Cargo.toml での設定
[dependencies.getrandom]
version = "0.2"
features = ["js"]  # WASM環境でWeb Crypto APIを使用
```

### 5.2 ハッシュ計算
```rust
use sha2::{Sha256, Digest};

pub fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
```

## 6. AOメッセージハンドリング

### 6.1 メッセージ処理の基本構造
```rust
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn handle(msg_ptr: *const u8, msg_len: usize) -> u32 {
    // 安全なポインタ操作
    let msg_slice = unsafe {
        if msg_ptr.is_null() || msg_len == 0 {
            return error_code(WasmError::InvalidInput);
        }
        core::slice::from_raw_parts(msg_ptr, msg_len)
    };
    
    match process_ao_message(msg_slice) {
        Ok(_) => 0,  // 成功
        Err(e) => error_code(e),
    }
}

fn process_ao_message(data: &[u8]) -> Result<(), WasmError> {
    // メッセージ処理ロジック
    Ok(())
}

fn error_code(error: WasmError) -> u32 {
    match error {
        WasmError::InvalidInput => 1,
        WasmError::InsufficientMemory => 2,
        WasmError::OperationTimeout => 3,
        WasmError::SerializationFailed => 4,
    }
}
```

### 6.2 状態管理
```rust
use core::cell::RefCell;

// グローバル状態（WASM環境）
static mut PROCESS_STATE: Option<RefCell<ProcessState>> = None;

pub struct ProcessState {
    phase: CryptoPhase,
    keys: Option<KeyPair>,
    counters: ProcessCounters,
}

#[cfg(target_arch = "wasm32")]
pub fn get_process_state() -> &'static RefCell<ProcessState> {
    unsafe {
        PROCESS_STATE.get_or_insert_with(|| {
            RefCell::new(ProcessState::default())
        })
    }
}
```

## 7. デバッグとロギング

### 7.1 WASM用ロギング
```rust
#[cfg(all(target_arch = "wasm32", feature = "debug"))]
extern "C" {
    fn log_message(ptr: *const u8, len: usize);
}

#[cfg(all(target_arch = "wasm32", feature = "debug"))]
pub fn wasm_log(message: &str) {
    let bytes = message.as_bytes();
    unsafe {
        log_message(bytes.as_ptr(), bytes.len());
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "debug")))]
pub fn wasm_log(_message: &str) {
    // リリースビルドでは何もしない
}

// 使用例
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "debug")]
        wasm_log(&format!($($arg)*));
    };
}
```

### 7.2 プロファイリング
```rust
#[cfg(feature = "profiling")]
pub struct Timer {
    start: u64,
    name: &'static str,
}

#[cfg(feature = "profiling")]
impl Timer {
    pub fn new(name: &'static str) -> Self {
        Self {
            start: current_timestamp(),
            name,
        }
    }
}

#[cfg(feature = "profiling")]
impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = current_timestamp() - self.start;
        debug_log!("{}: {}ms", self.name, elapsed);
    }
}

#[cfg(target_arch = "wasm32")]
fn current_timestamp() -> u64 {
    // JavaScript Date.now() を呼び出し
    js_sys::Date::now() as u64
}
```

## 8. 最適化テクニック

### 8.1 インライン化
```rust
// 小さな関数は積極的にインライン化
#[inline]
pub fn fast_hash(data: &[u8]) -> u64 {
    // 高速ハッシュ実装
    data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

// 大きな関数は選択的にインライン化
#[inline(never)]
pub fn complex_operation(data: &[u8]) -> Result<Vec<u8>, WasmError> {
    // 複雑な処理
    Ok(data.to_vec())
}
```

### 8.2 コンパイル時計算
```rust
// 定数計算をコンパイル時に実行
const PRECOMPUTED_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = (i * 31) as u8;
        i += 1;
    }
    table
};
```

## 9. テスト

### 9.1 WASM対応テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_compatible_function() {
        let input = b"test data";
        let result = process_data(input);
        assert!(result.is_ok());
    }
    
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_wasm_specific() {
        // WASM環境でのみ実行されるテスト
        let state = get_process_state();
        assert!(state.borrow().phase == CryptoPhase::Initialized);
    }
}
```

## 10. ビルド設定

### 10.1 Cargo.toml設定
```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["std"]
std = ["dep:std-crates"]
wasm = ["dep:wasm-bindgen", "dep:js-sys", "dep:web-sys"]
debug = []
profiling = ["debug"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"

[dependencies]
# WASM必須
wasm-bindgen = { version = "0.2", optional = true }
js-sys = { version = "0.3", optional = true }

# サイズ最適化
wee_alloc = { version = "0.4", optional = true }

# no_std対応シリアライゼーション
postcard = { version = "1.0", default-features = false, features = ["alloc"] }
serde = { version = "1.0", default-features = false, features = ["alloc", "derive"] }

# 暗号
getrandom = { version = "0.2", features = ["js"] }
sha2 = { version = "0.10", default-features = false }
```

これらの規約に従うことで、効率的でAOネットワーク対応のWASMビルドを実現できます。