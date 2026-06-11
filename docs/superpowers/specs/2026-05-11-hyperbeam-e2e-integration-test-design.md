# HyperBEAM E2E Integration Test Design

## Goal

Verify the full FORMIX flow (share → recover) against a real local HyperBEAM
node, using the HyperBEAMClient adapter (Phase B, issue #88) and the migrated
contract ABI (Phase C, issue #89).

## Context

- Parent issue: #86
- This issue: #90
- Dependencies: #88 (closed), #89 (closed)
- Branch: `feat/hyperbeam-e2e-share-recover`

The existing E2E tests in `workflow_roundtrip_test.rs` verify the Phase 1 → 2 → 3
flow using `MockAOClient`. This design adds a test that replaces MockAOClient
with a real `HyperBEAMClient` talking to a local HyperBEAM node.

## Scope

- `ao/scripts/deploy-hyperbeam.sh` — WASM build + test runner wrapper
- `client/tests/integration/test_hyperbeam_e2e.rs` — E2E integration test
- `deploy.example.json` — updated template with HyperBEAM section
- `client/tests/integration.rs` — module registration (conditional)

Out of scope: `cargo run --example basic_usage` (deferred to Phase E, issue #91).

## Approach: Two-Phase Testing

### Phase 1 (this issue): Mock Arweave + Real HyperBEAM

Arweave storage uses the existing in-memory `ArweaveStorageServiceImpl::default()`.
Only AO contract operations go through the real HyperBEAM node. This isolates
contract behavior verification from Arweave network concerns.

### Phase 2 (future): Full Real Stack

Add a second test variant that also persists data to real Arweave (arweave.net
or ArLocal). Not part of this issue.

## Architecture

### Service Wiring Strategy

The existing DI types have a type mismatch for this test scenario:

- `DefaultActionsContainer` is hardcoded to `ContractStorageImpl<MockAOClient>`
- `HyperBEAMActionsContainer` uses `ProductionArweaveStorageService<ArweaveClientImpl>`
  (real Arweave), not in-memory

This test needs a hybrid: in-memory Arweave + real HyperBEAM. Instead of adding
yet another type alias to `di.rs`, the test wires workflow services manually
following the same pattern as `setup_e2e_services()` in
`workflow_roundtrip_test.rs`:

```rust
// Manual wiring — replaces MockAOClient with HyperBEAMClient
let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
let arweave = Arc::new(ArweaveStorageServiceImpl::default()); // in-memory
let contract = Arc::new(ContractStorageImpl::new_single_process(hb_client));
let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

let sharing = SecretSharingWorkflowServiceImpl::new(
    Arc::clone(&service_crypto), Arc::clone(&storage));
let recovery = SecretRecoveryWorkflowServiceImpl::new(
    service_crypto, storage);
```

This tests the workflow services directly (Phase 1 → Phase 2 → Phase 3),
which is the same layer verified by the existing `test_e2e_full_pipeline_*`
tests. The Actions/Controller layers are already covered by `builder_api_test.rs`.

### Feature Gate

The test file requires `#[cfg(feature = "hyperbeam")]` because `HyperBEAMClient`
and related modules (signer, wallet, TABM encoder) are gated behind this feature
in the source code. On native targets, the underlying crate dependencies (`rsa`,
`reqwest`) are unconditionally available, but the HyperBEAM-specific API surface
is feature-gated. The module registration in `integration.rs` must be conditional
and placed inside the existing `mod integration { ... }` block:

```rust
mod integration {
    // ... existing mods ...
    #[cfg(feature = "hyperbeam")]
    mod test_hyperbeam_e2e;
}
```

The deploy script and manual test execution must include `--features hyperbeam`:

```bash
cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored
```

### Deploy Script (`ao/scripts/deploy-hyperbeam.sh`)

A thin shell wrapper that:

1. Validates prerequisites (HyperBEAM reachable, wallet exists, WASM built)
2. Builds the WASM contract: `cargo build -p formix-ao-contract --target wasm32-unknown-unknown --release`
3. Runs the E2E test: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture`

The actual deploy logic (WASM cache + process spawn) runs inside the Rust test
code, reusing the existing `HyperBEAMClient`, `signer.rs`, and `wallet.rs`
implementations. This avoids reimplementing RFC-9421 signing in shell.

```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. Check HyperBEAM health: curl -sf http://localhost:${HB_PORT:-10000}/~meta@1.0/info
# 2. Check WALLET_PATH is set and file exists
# 3. Build WASM: cargo build -p formix-ao-contract --target wasm32-unknown-unknown --release
# 4. Run E2E: cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture
```

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `WALLET_PATH` | (required) | Path to Arweave JWK wallet file |
| `WASM_PATH` | auto-detect from build output | Path to compiled WASM binary |

Note: `AOConfig::default()` already points to `http://localhost:10000`.
Override with `AO_MU_URL` / `AO_CU_URL` / `AO_GATEWAY_URL` if needed.
No separate `HB_URL` variable — reuse the existing `AOConfig::from_env()`.

### E2E Test (`client/tests/integration/test_hyperbeam_e2e.rs`)

#### Test Setup

```
1. Load ArweaveJWK from WALLET_PATH env var
   (skip test if not set)
     ↓
2. Build HyperBEAMClient with AOConfig::from_env() + wallet
   (skip test if HyperBEAM unreachable)
     ↓
3. Cache WASM binary: POST /~cache@1.0/write
   Body = raw WASM bytes, RFC-9421 signed
   Response: cache-id (SHA2-256 hash of binary)
     ↓
4. Spawn process: POST /schedule
   Signed process-definition with tags:
     device: "process@1.0"
     type: "Process"
     scheduler-device: "scheduler@1.0"
     scheduler-location: <wallet-address>
     execution-device: "stack@1.0"
     device-stack: ["WASI@1.0", "JSON-Iface@1.0", "WASM-64@1.0", "Multipass@1.0"]
     image: <cache-id>
     input-prefix: "process"
     output-prefix: "wasm"
     passes: 2
     stack-keys: ["init", "compute"]
     test-random-seed: <uuid>  (unique per test run)
   Process ID = hash of signed process-def message (client-derived)
     ↓
5. Send Init message: HyperBEAMClient.execute(process_id, AOExecuteMsg::init("Owner"))
   Role = "Owner" for single-process setup (Owner == Holder).
   Verify successful response before proceeding
     ↓
6. Wire workflow services manually:
   arweave = ArweaveStorageServiceImpl::default()  (in-memory)
   contract = ContractStorageImpl::new_single_process(hb_client)
   sharing_service + recovery_service wired with these
```

#### Test Flow

Threshold parameters: k=3, n=5 (matches existing test conventions).

```
[share] Phase 1
  generate_keypair() × 2 (owner, requester)
  sharing_service.execute_secret_sharing(SecretSharingRequest {
      secret: original_bytes,
      threshold: 3,
      total_shares: 5,
      owner_secret_key, owner_public_key,
      requester_public_key,
      owner_process_id: <spawned process_id>,
  })
  → Phase1Result { secret_id, kfrag_count: 5, ... }
    ↓
[recover] Phase 3
  recovery_service.execute_secret_recovery(SecretRecoveryRequest {
      secret_id: phase1.secret_id,
      requester_secret_key,
      owner_public_key,
      requester_process_id: <spawned process_id>,
  })
  → Phase3Result { recovered_secret }
    ↓
[assert]
  assert_eq!(recovered_secret, original_bytes)
```

#### Test Attributes

- `#[tokio::test]` — async test runtime
- `#[ignore]` — skipped by default; run manually when HyperBEAM is available
- `#[cfg(feature = "hyperbeam")]` — requires hyperbeam feature
- Execution: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture`

#### WASM Cache Helper

Caches the WASM binary on the local HyperBEAM node:

1. Read WASM binary from `WASM_PATH` env var or auto-detect from
   `ao/contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm`
2. `POST /~cache@1.0/write` with raw WASM bytes as body
   - RFC-9421 signed with the test wallet
   - Cache writers authorization: default HyperBEAM config auto-populates
     `cache_writers` with the node wallet address. If the test wallet differs
     from the node wallet, the node's config must include the test wallet address.
3. Parse response for `path` field = cache-id (deterministic SHA2-256 hash)
4. Return cache-id for use in spawn

#### Process Spawn Helper

Spawns a new FORMIX contract process:

1. Build process-definition message with all required tags (see Test Setup step 4)
2. Sign with RFC-9421 using the test wallet
3. `POST /schedule` — body is the signed process-definition
4. Compute process ID client-side: hash of the signed process-definition message
   (deterministic — the node does not assign IDs server-side)
5. Verify process exists: `GET /<ProcID>/compute` with slot=0 header
6. Return process_id

Note: `test-random-seed` field with a UUID ensures unique process IDs across
test runs even with the same WASM image and wallet.

#### Timeout Configuration

`AOConfig` default timeout is 30 seconds. WASM compilation and initial process
setup on HyperBEAM may require more time. The test should construct `AOConfig`
with a 60-second timeout via `AOConfig::new(url, url, url, 60_000)` and use a
single `HyperBEAMClient` instance for both setup and test execution.

### deploy.example.json Update

Add a `hyperbeam` section to the existing template:

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

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `ao/scripts/deploy-hyperbeam.sh` | Create | Shell wrapper: prereq check + WASM build + test run |
| `client/tests/integration/test_hyperbeam_e2e.rs` | Create | E2E test with WASM cache + spawn helpers |
| `deploy.example.json` | Modify | Add `hyperbeam` section |
| `client/tests/integration.rs` | Modify | Add `#[cfg(feature = "hyperbeam")] mod test_hyperbeam_e2e;` |

## Error Handling

- If `WALLET_PATH` is not set or file missing → skip test with clear message
- If HyperBEAM node is unreachable → skip test with connection error message
- If WASM binary not found → skip test with build instruction message
- If WASM cache fails (e.g., auth issue) → fail with actionable error
- If process spawn fails → fail with process-definition details
- If Init message fails → fail with HyperBEAM error response
- All skip conditions use `eprintln!` + `return` (not panic) for clean output

## Security Considerations

- Wallet JWK file is loaded via `WALLET_PATH` environment variable, never
  committed to repository
- Test wallet should be a dedicated test-only wallet with no mainnet funds
- Error messages must not expose wallet key material
- Cache writer authorization: the test wallet must be in the HyperBEAM node's
  `cache_writers` list (default: node's own wallet address)

## Done Criteria Mapping

| Criterion | Verification |
|-----------|-------------|
| `share()` + `recover()` succeeds against local HyperBEAM | `test_hyperbeam_e2e_share_and_recover` passes |
| Recovered secret matches original bytes | `assert_eq!(recovered_secret, original_bytes)` |
| Test is reproducible from clean state | Each test run spawns a fresh process via `test-random-seed` |
