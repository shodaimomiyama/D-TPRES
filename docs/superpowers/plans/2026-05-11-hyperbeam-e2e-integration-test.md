# HyperBEAM E2E Integration Test Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the full FORMIX share → recover flow against a real local HyperBEAM node using `HyperBEAMClient`.

**Architecture:** Manual service wiring bypassing the existing DI type aliases to combine in-memory Arweave storage with a real `HyperBEAMClient`. The test file includes helpers for WASM caching and process spawning via HTTP, reusing existing `signer.rs` and `wallet.rs`. A thin shell wrapper validates prerequisites and runs the test.

**Tech Stack:** Rust (tokio async test), HyperBEAMClient, RFC-9421 signing, reqwest HTTP, Umbral TPRE

**Spec:** `docs/superpowers/specs/2026-05-11-hyperbeam-e2e-integration-test-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `client/tests/integration.rs` | Modify (1 line) | Add `#[cfg(feature = "hyperbeam")] mod test_hyperbeam_e2e;` |
| `client/tests/integration/test_hyperbeam_e2e.rs` | Create (~250 lines) | E2E test: wallet load, WASM cache, process spawn, share, recover |
| `ao/scripts/deploy-hyperbeam.sh` | Create (~40 lines) | Prereq check + WASM build + test runner |
| `deploy.example.json` | Modify | Add `hyperbeam` section |

---

## Chunk 1: Module Registration + Test Skeleton

### Task 1: Register the test module conditionally

**Files:**
- Modify: `client/tests/integration.rs:18-25`

- [ ] **Step 1: Add the feature-gated module to integration.rs**

Add `#[cfg(feature = "hyperbeam")] mod test_hyperbeam_e2e;` inside the existing `mod integration { ... }` block, after the last existing mod:

```rust
mod integration {
    #[allow(deprecated)]
    mod actions_integration_test;
    mod builder_api_test;
    mod secret_recovery_workflow_test;
    mod secret_sharing_workflow_test;
    mod workflow_roundtrip_test;
    #[cfg(feature = "hyperbeam")]
    mod test_hyperbeam_e2e;
}
```

- [ ] **Step 2: Verify compilation without the feature (default build)**

Run: `cargo check -p formix --tests 2>&1 | tail -5`
Expected: SUCCESS (the new module is gated and ignored)

- [ ] **Step 3: Commit**

```bash
git add client/tests/integration.rs
git commit -m "test: register hyperbeam E2E module (feature-gated)"
```

---

### Task 2: Create the test file skeleton with imports

**Files:**
- Create: `client/tests/integration/test_hyperbeam_e2e.rs`

- [ ] **Step 1: Write the skeleton file with all imports and a placeholder test**

```rust
//! HyperBEAM E2E Integration Test — share() + recover() on real HyperBEAM
//!
//! Requires:
//! - A running HyperBEAM node at localhost:10000
//! - WALLET_PATH env var pointing to an Arweave JWK file
//! - WASM binary built: cargo build -p formix-ao-contract --target wasm32-unknown-unknown --release
//!
//! Run: cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture

use std::collections::BTreeMap;
use std::sync::Arc;

use zeroize::Zeroizing;

use formix::adapter::external::ao::signer;
use formix::adapter::external::ao::wallet::ArweaveJWK;
use formix::adapter::external::ao::{AOClient, AOConfig, AOExecuteMsg, HyperBEAMClient};
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl as CoreCryptoServiceImpl};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl,
    StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{
    SecretRecoveryWorkflowServiceImpl, SecretSharingWorkflowService,
    SecretSharingWorkflowServiceImpl, SecretRecoveryWorkflowService,
};
use formix::usecase::{SecretRecoveryRequest, SecretSharingRequest};

#[tokio::test]
#[ignore]
async fn test_hyperbeam_e2e_share_and_recover() {
    eprintln!("placeholder — will be implemented in subsequent tasks");
}
```

**Import notes (review fix):**
- `AOConfig` and `AOExecuteMsg` are re-exported from `formix::adapter::external::ao` — the sub-modules `config` and `message` are private. Use the re-export path.
- `AOClient` trait must be in scope for `hb_client.execute()` calls.
- `CryptoService` trait must be in scope for `core_crypto.generate_keypair()`.
- `SecretSharingWorkflowService` and `SecretRecoveryWorkflowService` traits must be in scope for `execute_secret_sharing()` / `execute_secret_recovery()` calls.

- [ ] **Step 2: Verify compilation with the hyperbeam feature**

Run: `cargo check -p formix --features hyperbeam --tests 2>&1 | tail -5`
Expected: SUCCESS (compiles with placeholder test)

- [ ] **Step 3: Verify the test is listed but skipped in normal runs**

Run: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e 2>&1 | tail -10`
Expected: `test ... test_hyperbeam_e2e_share_and_recover ... ignored` (0 passed, 0 failed, 1 ignored)

- [ ] **Step 4: Commit**

```bash
git add client/tests/integration/test_hyperbeam_e2e.rs
git commit -m "test: add hyperbeam E2E test skeleton with imports"
```

---

## Chunk 2: Helper Functions

### Task 3: Implement wallet loading with skip logic

**Files:**
- Modify: `client/tests/integration/test_hyperbeam_e2e.rs`

- [ ] **Step 1: Add the `load_wallet` helper function**

Add above the test function:

```rust
/// Load ArweaveJWK from WALLET_PATH env var.
/// Returns None (skip) if env var is unset or file is missing.
fn load_wallet() -> Option<ArweaveJWK> {
    let path = match std::env::var("WALLET_PATH") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("SKIP: WALLET_PATH not set — set it to an Arweave JWK file path");
            return None;
        }
    };
    match ArweaveJWK::from_file(&path) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("SKIP: failed to load wallet from {path}: {e}");
            None
        }
    }
}
```

- [ ] **Step 2: Add the `check_hyperbeam_reachable` async helper**

```rust
/// Check if HyperBEAM node is reachable. Returns false (skip) if not.
async fn check_hyperbeam_reachable(base_url: &str) -> bool {
    let url = format!("{}/~meta@1.0/info", base_url);
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => true,
        Ok(resp) => {
            eprintln!("SKIP: HyperBEAM returned HTTP {}", resp.status());
            false
        }
        Err(e) => {
            eprintln!("SKIP: HyperBEAM unreachable at {base_url}: {e}");
            false
        }
    }
}
```

- [ ] **Step 3: Update the test to use the skip helpers**

Replace the placeholder test body:

```rust
#[tokio::test]
#[ignore]
async fn test_hyperbeam_e2e_share_and_recover() {
    // ── Prerequisites ──
    let wallet = match load_wallet() {
        Some(w) => w,
        None => return,
    };

    let config = AOConfig::new(
        "http://localhost:10000",
        "http://localhost:10000",
        "http://localhost:10000",
        60_000,
    )
    .expect("AOConfig");

    if !check_hyperbeam_reachable(config.mu_url()).await {
        return;
    }

    eprintln!("prerequisites OK — wallet loaded, HyperBEAM reachable");

    // TODO: cache WASM, spawn process, run share/recover
    eprintln!("placeholder — remaining steps not yet implemented");
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p formix --features hyperbeam --tests 2>&1 | tail -5`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add client/tests/integration/test_hyperbeam_e2e.rs
git commit -m "test: add wallet loading and HyperBEAM reachability helpers"
```

---

### Task 4: Implement WASM cache helper

**Files:**
- Modify: `client/tests/integration/test_hyperbeam_e2e.rs`

- [ ] **Step 1: Add the `locate_wasm_binary` helper**

```rust
/// Locate the WASM binary. Checks WASM_PATH env var, then the default build output.
fn locate_wasm_binary() -> Option<Vec<u8>> {
    let path = std::env::var("WASM_PATH").unwrap_or_else(|_| {
        "ao/contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm".to_string()
    });
    match std::fs::read(&path) {
        Ok(bytes) => {
            eprintln!("WASM binary loaded: {} ({} bytes)", path, bytes.len());
            Some(bytes)
        }
        Err(e) => {
            eprintln!(
                "SKIP: WASM binary not found at {path}: {e}\n\
                 Build with: cargo build -p formix-ao-contract --target wasm32-unknown-unknown --release"
            );
            None
        }
    }
}
```

- [ ] **Step 2: Add the `cache_wasm` async helper**

This caches the WASM binary on HyperBEAM via `POST /~cache@1.0/write` with RFC-9421 signing:

```rust
/// Cache WASM binary on HyperBEAM. Returns the cache-id (SHA2-256 hash).
async fn cache_wasm(
    http: &reqwest::Client,
    base_url: &str,
    wasm_bytes: &[u8],
    rsa_key: &rsa::RsaPrivateKey,
    sig_name: &str,
) -> Result<String, String> {
    let url = format!("{}/~cache@1.0/write", base_url);

    let signed = signer::sign_request(rsa_key, wasm_bytes, sig_name)
        .map_err(|e| format!("signing WASM cache request: {e}"))?;

    let resp = http
        .post(&url)
        .header("content-type", "application/wasm")
        .header("content-digest", &signed.content_digest_header)
        .header("signature", &signed.signature_header)
        .header("signature-input", &signed.signature_input_header)
        .body(wasm_bytes.to_vec())
        .send()
        .await
        .map_err(|e| format!("cache POST: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("read body: {e}"))?;

    if !status.is_success() {
        return Err(format!("cache write failed (HTTP {status}): {body}"));
    }

    // Parse response — look for cache-id in JSON "path" field or "id" field
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("parse cache response: {e}"))?;
    let cache_id = v
        .get("path")
        .or_else(|| v.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("no path/id in cache response: {body}"))?;

    eprintln!("WASM cached: {cache_id}");
    Ok(cache_id.to_string())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p formix --features hyperbeam --tests 2>&1 | tail -5`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add client/tests/integration/test_hyperbeam_e2e.rs
git commit -m "test: add WASM binary locator and cache helper"
```

---

### Task 5: Implement process spawn helper

**Files:**
- Modify: `client/tests/integration/test_hyperbeam_e2e.rs`

- [ ] **Step 1: Add the `spawn_process` async helper**

This spawns a FORMIX contract process on HyperBEAM via `POST /schedule`:

```rust
/// Spawn a new FORMIX contract process on HyperBEAM.
/// Returns the process ID (derived from signed process-definition hash).
async fn spawn_process(
    http: &reqwest::Client,
    base_url: &str,
    cache_id: &str,
    scheduler_location: &str,
    rsa_key: &rsa::RsaPrivateKey,
    sig_name: &str,
) -> Result<String, String> {
    let test_seed = uuid::Uuid::new_v4().to_string();

    let header_fields: BTreeMap<String, String> = BTreeMap::from([
        ("device".to_string(), "process@1.0".to_string()),
        ("type".to_string(), "Process".to_string()),
        ("scheduler-device".to_string(), "scheduler@1.0".to_string()),
        ("scheduler-location".to_string(), scheduler_location.to_string()),
        ("execution-device".to_string(), "stack@1.0".to_string()),
        (
            "device-stack".to_string(),
            "WASI@1.0,JSON-Iface@1.0,WASM-64@1.0,Multipass@1.0".to_string(),
        ),
        ("image".to_string(), cache_id.to_string()),
        ("input-prefix".to_string(), "process".to_string()),
        ("output-prefix".to_string(), "wasm".to_string()),
        ("passes".to_string(), "2".to_string()),
        ("stack-keys".to_string(), "init,compute".to_string()),
        ("test-random-seed".to_string(), test_seed),
    ]);

    let body = b"";
    let signed = signer::sign_message(rsa_key, &header_fields, body, sig_name)
        .map_err(|e| format!("signing process spawn: {e}"))?;

    let url = format!("{}/schedule", base_url);
    let mut req = http.post(&url);
    for (name, value) in &header_fields {
        req = req.header(name.as_str(), value.as_str());
    }
    if !signed.content_digest_header.is_empty() {
        req = req.header("content-digest", &signed.content_digest_header);
    }
    let resp = req
        .header("signature", &signed.signature_header)
        .header("signature-input", &signed.signature_input_header)
        .body(body.to_vec())
        .send()
        .await
        .map_err(|e| format!("spawn POST: {e}"))?;

    let status = resp.status();
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body_text = resp.text().await.map_err(|e| format!("read body: {e}"))?;

    if !status.is_success() {
        return Err(format!("spawn failed (HTTP {status}): {body_text}"));
    }

    // Extract process ID from response headers or body
    let process_id = resp_headers
        .iter()
        .find(|(k, _)| k == "process-id" || k == "process_id")
        .map(|(_, v)| v.clone())
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(&body_text)
                .ok()
                .and_then(|v| {
                    v.get("process")
                        .or_else(|| v.get("id"))
                        .or_else(|| v.get("process-id"))
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string())
                })
        })
        .ok_or_else(|| {
            format!("could not extract process ID from spawn response: headers={resp_headers:?}, body={body_text}")
        })?;

    // Verify process exists (spec requirement)
    let verify_url = format!("{}/{}/compute", base_url, process_id);
    let verify_resp = http
        .get(&verify_url)
        .header("slot", "0")
        .send()
        .await
        .map_err(|e| format!("verify process: {e}"))?;
    if !verify_resp.status().is_success() {
        return Err(format!(
            "process {} not found after spawn (HTTP {})",
            process_id,
            verify_resp.status()
        ));
    }

    eprintln!("process spawned and verified: {process_id}");
    Ok(process_id)
}
```

- [ ] **Step 2: Add uuid to dev-dependencies in Cargo.toml**

Check if `uuid` is already in `client/Cargo.toml` dev-dependencies. If not, add it:

```toml
[dev-dependencies]
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p formix --features hyperbeam --tests 2>&1 | tail -5`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add client/tests/integration/test_hyperbeam_e2e.rs client/Cargo.toml
git commit -m "test: add process spawn helper with RFC-9421 signing"
```

---

## Chunk 3: E2E Test Body

### Task 6: Wire services and implement the full test

**Files:**
- Modify: `client/tests/integration/test_hyperbeam_e2e.rs`

- [ ] **Step 1: Replace the placeholder test with the full implementation**

Replace the entire `test_hyperbeam_e2e_share_and_recover` function:

```rust
#[tokio::test]
#[ignore]
async fn test_hyperbeam_e2e_share_and_recover() {
    // ── Prerequisites ──
    let wallet = match load_wallet() {
        Some(w) => w,
        None => return,
    };

    let config = AOConfig::new(
        "http://localhost:10000",
        "http://localhost:10000",
        "http://localhost:10000",
        60_000,
    )
    .expect("AOConfig");
    let base_url = config.mu_url().to_string();

    if !check_hyperbeam_reachable(&base_url).await {
        return;
    }

    let wasm_bytes = match locate_wasm_binary() {
        Some(b) => b,
        None => return,
    };

    // ── Setup signing material ──
    let rsa_key = wallet
        .to_rsa_private_key()
        .expect("RSA key from wallet");
    let sig_name = wallet.sig_name().expect("sig_name from wallet");
    let scheduler_location = wallet.address().expect("wallet address");

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("HTTP client");

    // ── Cache WASM ──
    let cache_id = cache_wasm(&http, &base_url, &wasm_bytes, &rsa_key, &sig_name)
        .await
        .expect("WASM cache failed");

    // ── Spawn process ──
    let process_id = spawn_process(
        &http,
        &base_url,
        &cache_id,
        &scheduler_location,
        &rsa_key,
        &sig_name,
    )
    .await
    .expect("process spawn failed");

    // ── Build HyperBEAMClient and send Init ──
    let hb_client = Arc::new(
        HyperBEAMClient::new(config, wallet).expect("HyperBEAMClient"),
    );

    let init_resp = hb_client
        .execute(&process_id, AOExecuteMsg::init("Owner"))
        .await
        .expect("Init message failed");
    assert!(
        init_resp.ok,
        "Init returned error: {:?}",
        init_resp.error
    );
    eprintln!("Init OK for process {process_id}");

    // ── Wire workflow services (in-memory Arweave + real HyperBEAM) ──
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(hb_client));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

    let sharing = SecretSharingWorkflowServiceImpl::new(
        Arc::clone(&service_crypto),
        Arc::clone(&storage),
    );
    let recovery = SecretRecoveryWorkflowServiceImpl::new(service_crypto, storage);

    // ── Generate keypairs ──
    let (owner_sk, owner_pk) = core_crypto.generate_keypair().unwrap();
    let (requester_sk, requester_pk) = core_crypto.generate_keypair().unwrap();

    let original_secret = b"HyperBEAM E2E test secret (k=3, n=5)";
    let secret_data: Vec<u8> = original_secret.to_vec();

    // ── Phase 1: share ──
    eprintln!("Phase 1: executing secret sharing (k=3, n=5)...");
    let phase1 = sharing
        .execute_secret_sharing(SecretSharingRequest {
            secret: Zeroizing::new(secret_data.clone()),
            owner_secret_key: owner_sk,
            owner_public_key: owner_pk.clone(),
            requester_public_key: requester_pk,
            threshold: 3,
            total_shares: 5,
            owner_process_id: process_id.clone(),
            metadata: None,
        })
        .await
        .expect("Phase 1 (share) failed");

    eprintln!(
        "Phase 1 OK: secret_id={}, kfrag_count={}",
        phase1.secret_id, phase1.kfrag_count
    );
    assert_eq!(phase1.kfrag_count, 5);
    assert_eq!(phase1.share_tx_ids.len(), 5);

    // ── Phase 3: recover ──
    eprintln!("Phase 3: executing secret recovery...");
    let phase3 = recovery
        .execute_secret_recovery(SecretRecoveryRequest {
            secret_id: phase1.secret_id,
            requester_secret_key: requester_sk,
            owner_public_key: owner_pk,
            requester_process_id: process_id.clone(),
        })
        .await
        .expect("Phase 3 (recover) failed");

    // ── Assertions ──
    assert_eq!(
        phase3.recovered_secret, secret_data,
        "recovered secret must match original"
    );
    eprintln!("E2E PASS: recovered secret matches original ({} bytes)", secret_data.len());
}
```

- [ ] **Step 2: Verify all trait imports are present**

Confirm the import block (from Task 2) includes these traits:
- `AOClient` — for `hb_client.execute()`
- `CryptoService` — for `core_crypto.generate_keypair()`
- `SecretSharingWorkflowService` — for `sharing.execute_secret_sharing()`
- `SecretRecoveryWorkflowService` — for `recovery.execute_secret_recovery()`

These were already added in Task 2's corrected import block.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p formix --features hyperbeam --tests 2>&1 | tail -5`
Expected: SUCCESS

- [ ] **Step 4: Verify the test is listed**

Run: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --list 2>&1`
Expected: `test_hyperbeam_e2e_share_and_recover: test` listed

- [ ] **Step 5: Commit**

```bash
git add client/tests/integration/test_hyperbeam_e2e.rs
git commit -m "test: implement full HyperBEAM E2E share/recover test"
```

---

## Chunk 4: Deploy Script + Config

### Task 7: Create the deploy script

**Files:**
- Create: `ao/scripts/deploy-hyperbeam.sh`

- [ ] **Step 1: Write the deploy script**

```bash
#!/usr/bin/env bash
set -euo pipefail

# HyperBEAM E2E test runner
# Prerequisites: running HyperBEAM node, WALLET_PATH, WASM target installed
#
# Usage: WALLET_PATH=/path/to/wallet.json ./ao/scripts/deploy-hyperbeam.sh

HB_PORT="${HB_PORT:-10000}"
HB_URL="http://localhost:${HB_PORT}"

echo "=== HyperBEAM E2E Test Runner ==="

# 1. Check wallet
if [ -z "${WALLET_PATH:-}" ]; then
    echo "ERROR: WALLET_PATH is not set"
    echo "  export WALLET_PATH=/path/to/arweave-wallet.json"
    exit 1
fi
if [ ! -f "$WALLET_PATH" ]; then
    echo "ERROR: wallet file not found: $WALLET_PATH"
    exit 1
fi
echo "[OK] Wallet: $WALLET_PATH"

# 2. Check HyperBEAM health
if ! curl -sf "${HB_URL}/~meta@1.0/info" > /dev/null 2>&1; then
    echo "ERROR: HyperBEAM not reachable at ${HB_URL}"
    echo "  Start it with: cd hyperbeam-sandbox && ./scripts/start.sh"
    exit 1
fi
echo "[OK] HyperBEAM: ${HB_URL}"

# 3. Build WASM
echo "Building WASM contract..."
cargo build -p formix-ao-contract --target wasm32-unknown-unknown --release
WASM_PATH="ao/contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm"
if [ ! -f "$WASM_PATH" ]; then
    echo "ERROR: WASM binary not found at $WASM_PATH"
    exit 1
fi
echo "[OK] WASM: $WASM_PATH ($(wc -c < "$WASM_PATH") bytes)"

# 4. Run E2E test
echo "Running E2E test..."
export WASM_PATH
cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture
echo "=== DONE ==="
```

- [ ] **Step 2: Make the script executable**

Run: `chmod +x ao/scripts/deploy-hyperbeam.sh`

- [ ] **Step 3: Commit**

```bash
git add ao/scripts/deploy-hyperbeam.sh
git commit -m "ci: add HyperBEAM E2E deploy/test runner script"
```

---

### Task 8: Update deploy.example.json

**Files:**
- Modify: `deploy.example.json`

- [ ] **Step 1: Add the hyperbeam section**

Update `deploy.example.json` to:

```json
{
  "module_id": "YOUR_AO_MODULE_ID",
  "process_id": "YOUR_AO_PROCESS_ID",
  "gateways": {
    "ao_mu": "https://mu.ao-testnet.xyz",
    "ao_cu": "https://cu.ao-testnet.xyz",
    "arweave": "https://arweave.net"
  },
  "hyperbeam": {
    "url": "http://localhost:10000",
    "module_id": "YOUR_CACHED_WASM_ID",
    "process_id": "YOUR_HYPERBEAM_PROCESS_ID"
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add deploy.example.json
git commit -m "docs: add hyperbeam section to deploy.example.json"
```

---

## Chunk 5: Final Verification

### Task 9: Run lint and full verification

**Files:** (no new files)

- [ ] **Step 1: Format check**

Run: `make fmt`
Expected: no changes needed (or auto-fixed)

- [ ] **Step 2: Lint check**

Run: `make lint`
Expected: 0 errors

- [ ] **Step 3: Run existing tests to ensure no regression**

Run: `make test`
Expected: all existing tests pass, hyperbeam test is not run (it's `#[ignore]` and feature-gated)

- [ ] **Step 4: Verify the hyperbeam test compiles with feature**

Run: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --list 2>&1`
Expected: `test_hyperbeam_e2e_share_and_recover: test` is listed

- [ ] **Step 5: Commit any remaining changes**

If lint/fmt made changes:
```bash
git add -A
git commit -m "style: apply formatting fixes"
```

---

## Implementation Notes

### Key API References

- **`HyperBEAMClient::new(config, wallet)`** — `client/src/adapter/external/ao/hyperbeam_client.rs:19`
- **`AOConfig::new(mu, cu, gw, timeout)`** — `client/src/adapter/external/ao/config.rs:27`
- **`ArweaveJWK::from_file(path)`** — `client/src/adapter/external/ao/wallet.rs:46`
- **`signer::sign_request(key, body, sig_name)`** — `client/src/adapter/external/ao/signer.rs:18`
- **`signer::sign_message(key, headers, body, sig_name)`** — `client/src/adapter/external/ao/signer.rs:58`
- **`AOExecuteMsg::init(role)`** — `client/src/adapter/external/ao/message.rs:96`
- **`setup_e2e_services()` pattern** — `client/tests/integration/workflow_roundtrip_test.rs:656-679`
- **`SecretSharingRequest` struct** — `client/src/usecase/dto.rs:21-39`
- **`SecretRecoveryRequest` struct** — `client/src/usecase/dto.rs:83-92`
- **`ContractStorageImpl::new_single_process(ao_client)`** — `client/src/usecase/core/contract_storage.rs:88`

### Service Wiring Pattern (from workflow_roundtrip_test.rs:666-679)

```
CoreCryptoServiceImpl
    └→ ServiceCryptoServiceImpl
ArweaveStorageServiceImpl (in-memory)  ──┐
ContractStorageImpl<HyperBEAMClient>  ──┤
                                         └→ ServiceStorageServiceImpl
                                              ├→ SecretSharingWorkflowServiceImpl
                                              └→ SecretRecoveryWorkflowServiceImpl
```

### Error Skip Strategy

All external dependencies use `eprintln!` + `return` for clean skip:
- `WALLET_PATH` not set → skip
- Wallet file unreadable → skip
- HyperBEAM unreachable → skip
- WASM binary missing → skip
- WASM cache failure → **fail** (actionable error)
- Process spawn failure → **fail**
- Init failure → **fail**
- Phase 1/3 failure → **fail**
