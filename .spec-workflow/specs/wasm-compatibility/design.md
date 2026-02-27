# WASM Compatibility Design

## Overview

This document describes the technical design for making the FORMIX client library compatible with both WASM (`wasm32-unknown-unknown`) and native targets.

## Architecture

### Platform Detection Strategy

Use Rust's `cfg` attributes for compile-time platform detection:

```rust
#[cfg(target_arch = "wasm32")]        // WASM target
#[cfg(not(target_arch = "wasm32"))]   // Native target
```

### Dependency Configuration

#### Cargo.toml Structure

```toml
[dependencies]
# Shared dependencies (both platforms)
reqwest = { version = "0.12", default-features = false, features = ["json"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
# Native-only dependencies
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["time"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
# WASM-only dependencies
getrandom = { version = "0.2", features = ["js"] }
```

## Component Design

### 1. ArweaveClient (client.rs)

#### Sleep/Backoff Helper

Platform-specific implementation for retry delays:

```rust
#[cfg(not(target_arch = "wasm32"))]
async fn sleep_backoff(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
async fn sleep_backoff(_duration: Duration) {
    // WASM: immediate retry (no delay mechanism)
}
```

#### HTTP Client Configuration

Conditional timeout setting (not available in WASM):

```rust
#[cfg(not(target_arch = "wasm32"))]
let http_client = Client::builder()
    .timeout(Duration::from_secs(config.timeout_secs()))
    .build()?;

#[cfg(target_arch = "wasm32")]
let http_client = Client::builder().build()?;
```

#### Trait Implementation

Conditional async_trait attribute:

```rust
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ArweaveClient for ArweaveClientImpl { ... }
```

### 2. Repository Traits

#### Base Repository Trait

WASM cannot use `Send + Sync` bounds due to single-threaded execution model:

```rust
// Native version with Send + Sync
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{ ... }

// WASM version without Send + Sync
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Repository<T, ID> { ... }
```

#### Affected Repository Interfaces

All 5 repository interfaces require conditional compilation:

| Interface | File |
|-----------|------|
| CapsuleRepository | `capsule_interface.rs` |
| CFragRepository | `cfrag_interface.rs` |
| KFragRepository | `kfrag_interface.rs` |
| SecretRepository | `secret_interface.rs` |
| ShareCollectionRepository | `share_interface.rs` |

### 3. Repository Implementations

All implementations use the `cfg_attr` pattern for async_trait:

```rust
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C: ArweaveClient> Repository<Entity, Id> for ArweaveEntityRepository<C> { ... }
```

### 4. ArweaveClient Trait

The trait definition also requires conditional Send + Sync:

```rust
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait ArweaveClient: Send + Sync { ... }

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait ArweaveClient { ... }
```

## Data Flow

### WASM Build Flow

```
Source Code
    ↓
Conditional Compilation (#[cfg(target_arch = "wasm32")])
    ↓
WASM-specific code paths selected
    ↓
No Send + Sync bounds
    ↓
reqwest uses browser fetch API
    ↓
getrandom uses crypto.getRandomValues()
    ↓
wasm32-unknown-unknown binary
```

### Native Build Flow

```
Source Code
    ↓
Conditional Compilation (#[cfg(not(target_arch = "wasm32"))])
    ↓
Native-specific code paths selected
    ↓
Send + Sync bounds enabled
    ↓
reqwest uses rustls-tls
    ↓
tokio runtime available
    ↓
Native binary for testing
```

## Error Handling

### Dead Code Warnings

GraphQL response structs contain fields required by the API but not used in code:

```rust
#[allow(dead_code)]
struct TransactionNode {
    id: String,
    block: Option<BlockInfo>,  // Required by API, not used
}

#[allow(dead_code)]
struct BlockInfo {
    height: i64,      // Required by API, not used
    timestamp: i64,   // Required by API, not used
}
```

## Testing Strategy

### Build Verification

```bash
# Primary target (WASM)
cargo check --target wasm32-unknown-unknown

# Secondary target (Native for tests)
cargo check

# Lint verification
make clippy
```

### Unit Tests

Unit tests run on native target only (WASM test infrastructure not implemented).

## Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Platform-specific dependencies |
| `client.rs` | Conditional compilation, dead_code |
| `repositories/mod.rs` | Conditional Send+Sync on Repository trait |
| `repositories/*_interface.rs` (5 files) | Conditional async_trait |
| `repository_impl/mod.rs` | Conditional ArweaveClient trait |
| `repository_impl/*_impl.rs` (5 files) | Conditional async_trait |

## Security Considerations

- No security impact from conditional compilation changes
- WASM builds run in browser sandbox
- Native builds maintain existing security model
