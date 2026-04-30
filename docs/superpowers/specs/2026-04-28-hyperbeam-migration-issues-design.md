# HyperBEAM Migration Issue Decomposition

## Goal

Decompose issue #86 into 4 independent, actionable sub-issues for migrating
FORMIX from CWAO to HyperBEAM. Based on empirical findings from the
`hyperbeam-sandbox` M1-M5 milestones.

## Context

### Sandbox findings (M1-M5)

- **M1**: genesis-wasm@1.0 is NOT viable (requires Node.js runtime).
  JSON-Iface stack `[WASI@1.0, JSON-Iface@1.0, WASM-64@1.0, Multipass@1.0]`
  is the correct path.
- **M2**: RFC-9421 `rsa-pss-sha512` signer and TABM multipart encoder
  implemented in Rust (`hb-client` crate).
- **M3**: Full HTTP round-trip confirmed: spawn → schedule → compute → /now.
  All signed with RFC-9421 using Arweave JWK wallet.
- **M4**: WASM memory snapshots are broken for cross-request persistence.
  Within-request replay works (counter=3 for 3 messages). Cross-request
  restore fails (counter=1 instead of 2). Root cause: WAMR
  serialize/deserialize for memory64.
- **M5**: Findings documented in `hyperbeam-sandbox/docs/hyperbeam-api-notes.md`.

### Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Existing ProductionAOClient | Rename to LegacyAOClient, keep implementation | Preserve CWAO compatibility for reference |
| New HyperBEAM adapter | Implement as new HyperBEAMClient | Port from sandbox hb-client crate |
| Contract state management | Keep static globals, rely on replay | Minimum change for PoC; replay is correct if slow |
| Contract ABI | Adapt to JSON-Iface `handle(msg_ptr, env_ptr)` | Thin wrapper change, handlers.rs unchanged |

---

## Issue 1: Phase B — HyperBEAM Client Adapter

**Title:** `[Phase B] HyperBEAM Client Adapter — RFC-9421 signed HTTP transport`

**Labels:** `enhancement`

**Scope:** `client/src/adapter/external/ao/`

### Changes

1. Rename `ProductionAOClient` → `LegacyAOClient` (keep implementation)
2. Create `HyperBEAMClient` implementing `AOClient` trait (async)
3. Port from sandbox `hb-client`:
   - RFC-9421 `rsa-pss-sha512` signer (`signer.rs` logic)
   - TABM multipart encoder (`tabm.rs` logic)
   - Arweave JWK wallet loader (`wallet.rs` logic)
4. Update `AOConfig` defaults: `http://localhost:10000` for local HyperBEAM
5. Wire into `ActionsBuilder` / DI for `HyperBEAMClient` selection

### Files to modify

- `client/src/adapter/external/ao/production_client.rs` → rename
- `client/src/adapter/external/ao/` — new `hyperbeam_client.rs`, `signer.rs`, `tabm.rs`
- `client/src/adapter/external/ao/config.rs` — HyperBEAM endpoint defaults
- `client/src/adapter/external/ao/mod.rs` — exports
- `client/src/actions/builder.rs` — DI wiring

### Dependencies

None (can be implemented independently)

### Security

- Wallet private key material must derive `Zeroize` / `ZeroizeOnDrop`
- Signing key bytes must not leak in error messages

### Crate dependencies (new)

- `rsa` + `sha2` — RFC-9421 rsa-pss-sha512 signing
- `base64` — structured field encoding
- `rand` — PSS padding

### Done criteria

- `HyperBEAMClient` completes spawn, schedule, compute HTTP round-trip
  against local HyperBEAM node
- Existing tests pass (LegacyAOClient still functional)
- Unit tests: signer produces valid RFC-9421 signatures, TABM encoder
  produces multipart with ao-types headers and nested sub-messages

---

## Issue 2: Phase C — Contract ABI Migration (JSON-Iface)

**Title:** `[Phase C] Contract ABI Migration — JSON-Iface compatible handle(msg_ptr, env_ptr)`

**Labels:** `enhancement`

**Scope:** `ao/contracts/src/`

### Changes

1. `contract.rs`: Change entry point signature
   - From: `handle(msg_ptr: *const u8, msg_len: i32) -> i32`
   - To: `handle(msg_ptr: *const u8, env_ptr: *const u8) -> *const u8`
2. Add `malloc(size) -> *mut u8` and `free(ptr)` exports
3. Response format: AOS-compatible JSON
   ```json
   {"ok":true,"response":{"Output":{"data":"..."},"Messages":[],"Spawns":[]}}
   ```
4. Parse incoming message as null-terminated JSON string (JSON-Iface writes
   null-terminated JSON to WASM memory via `malloc`)
5. `state.rs`: Keep static globals as-is. Each `compute` replays all
   messages from genesis within a single WASM instance, so statics
   accumulate correctly (O(n) cost per request, acceptable for PoC).
   Cross-request snapshots are broken (M4) but not needed with replay.
6. `handlers.rs`: No changes to business logic

### Files to modify

- `ao/contracts/src/contract.rs` — entry point ABI change
- `ao/contracts/src/message.rs` — JSON message parsing adaptation
- `ao/contracts/src/lib.rs` — malloc/free exports

### Dependencies

None (can be implemented in parallel with Issue 1)

### Done criteria

- WASM builds successfully with `cargo build --target wasm32-unknown-unknown --release`
- Deployed to local HyperBEAM via cache + spawn
- Basic message (e.g. `ListCapsules`) returns valid JSON response

---

## Issue 3: Phase D — E2E Integration Test

**Title:** `[Phase D] E2E Integration Test — share() + recover() on real HyperBEAM`

**Labels:** `enhancement`

**Scope:** `client/tests/` + deploy scripts

### Changes

1. HyperBEAM deploy script:
   - Cache WASM binary on local node
   - Spawn process with JSON-Iface stack
   - Record `process_id` to `deploy.example.json`
2. Integration test `test_hyperbeam_e2e.rs`:
   - share: secret → Arweave + Contract stores kFrag/Capsule
   - recover: collect cFrag → decrypt → original bytes match
   - Default `#[ignore]` — manual run with local HyperBEAM
3. Example: `cargo run --example basic_usage --features production-ao`

### Files to create/modify

- `ao/scripts/deploy-hyperbeam.sh` — deploy script (replaces legacy `deploy.js`)
- `client/tests/integration/test_hyperbeam_e2e.rs` — E2E test
- `ao/deploy.example.json` — process_id template

### Dependencies

- Issue 1 (HyperBEAMClient adapter)
- Issue 2 (Contract ABI migration)

### Done criteria

- `share()` + `recover()` succeeds against local HyperBEAM
- Recovered secret matches original bytes
- Test is reproducible from clean state

---

## Issue 4: Phase E — Documentation & Cleanup

**Title:** `[Phase E] Documentation & Cleanup — HyperBEAM production setup`

**Labels:** `documentation`

**Scope:** docs + cleanup

### Changes

1. README.md: Add "Production Setup (HyperBEAM)" section
2. `docs/status.md`: Update implementation status
3. `docs/contracts/`: Update HyperBEAM contract guide
4. Clean up `ao/scripts/deploy.js` (aoconnect-based, superseded)
5. Close open questions in issue #86

### Dependencies

- Issue 3 (E2E verified, so docs reflect tested procedure)

### Done criteria

- README procedure reproduces E2E from scratch
- `docs/status.md` reflects current state
- Issue #86 closeable

---

## Dependency graph

```
Issue 1 (Phase B: Adapter) ──┐
                              ├──→ Issue 3 (Phase D: E2E) ──→ Issue 4 (Phase E: Docs)
Issue 2 (Phase C: Contract) ──┘
```

Issues 1 and 2 are independent and can be worked on in parallel.
Issue 3 depends on both. Issue 4 depends on Issue 3.
