# Capsule Serialization Alignment

## Problem

Client serializes Umbral Capsule with `bincode::serialize()` but AO contract deserializes with `Capsule::from_bytes()` (rmp_serde). MockAOClient masks this mismatch by using bincode, causing tests to pass while real AO E2E fails.

## Decision

Align Client to Umbral's official API (`capsule.to_bytes()` / `Capsule::from_bytes()`). Contract remains unchanged.

## Scope

### Changed (3 lines of production code)

| File | Line | Before | After |
|------|------|--------|-------|
| `client/src/usecase/core/crypto.rs` | 513 | `bincode::serialize(&umbral_capsule)` | `umbral_capsule.to_bytes().map_err(...)?.to_vec()` |
| `client/src/usecase/core/crypto.rs` | 378 | `bincode::deserialize::<Capsule>(&capsule.capsule_bytes)` | `Capsule::from_bytes(&capsule.capsule_bytes)?` |
| `client/src/adapter/external/mock_ao/client.rs` | 259 | `bincode::deserialize::<Capsule>(capsule_bytes)` | `Capsule::from_bytes(capsule_bytes)?` |

### Unchanged (and why)

- **`capsule_impl.rs`**: `StoredCapsule` wraps `capsule_bytes: Vec<u8>` as opaque blob. Outer bincode wrapper is unrelated to inner Capsule format.
- **`CapsulePayload`**: Same as above. `capsule_bytes` field is a pass-through byte vector.
- **`verification_data` (`crypto.rs:610,668`)**: `VerificationData { verifying_pk, delegating_pk, receiving_pk }` struct with three PublicKeys serialized/deserialized with bincode on both Client and Contract sides. Self-consistent pipeline, independent of the main Capsule serialization path.
- **`ao/contracts/src/handlers.rs:451`**: Already uses `Capsule::from_bytes()`. No change needed.

### Test / cfg(test) updates

| File | Line | Before | After |
|------|------|--------|-------|
| `client/src/usecase/core/contract_storage.rs` | 394 | `bincode::serialize(&capsule).unwrap()` | `capsule.to_bytes().unwrap().to_vec()` |
| `client/tests/unit/adapter/external/mock_ao/client.rs` | 52 | `bincode::serialize(&capsule).unwrap()` | `capsule.to_bytes().unwrap().to_vec()` |

- E2E roundtrip tests pass automatically since both serialize and deserialize switch to rmp_serde.

## Architecture

```
UseCase Core (crypto.rs)     ← 2 line changes
Adapter (mock_ao/client.rs)  ← 1 line change
Domain / Repository          ← unchanged (Vec<u8> pass-through)
AO Contract (handlers.rs)    ← unchanged (already from_bytes)
```

## Data Flow (post-fix)

### Phase 1: share()
1. `crypto.rs:513` — `capsule.to_bytes()` produces rmp_serde bytes
2. Bytes stored in `Capsule.capsule_bytes` field
3. `CapsulePayload { capsule_bytes }` bincode-serialized to Arweave
4. `delegate_capsule(capsule_bytes)` sent to AO contract
5. `handlers.rs:451` — `Capsule::from_bytes()` succeeds (format match)

### Phase 3: recover()
1. `CapsulePayload` bincode-deserialized from Arweave
2. `capsule_bytes` extracted (rmp_serde format)
3. `crypto.rs:378` — `Capsule::from_bytes(&capsule_bytes)` succeeds

## Error Handling

- `to_bytes()` returns `Result<Box<[u8]>, rmp_serde::encode::Error>` — mapped via existing `map_err(|_| ...)` pattern (error value discarded). Returns `Box<[u8]>`, convert to `Vec<u8>` with `.to_vec()`.
- `from_bytes()` returns `Result<Self, rmp_serde::decode::Error>` — mapped via existing `map_err(|_| ...)` pattern (error value discarded).

## Backward Compatibility

Arweave data created before this change uses bincode Capsule bytes and will not be readable. Acceptable for PoC (no production data exists).

## MEMORY.md Update

Change Capsule convention from:
```
- **Capsule**: Serialized with `bincode::serialize()` in `crypto.rs:create_pre_capsule`
```
To:
```
- **Capsule**: Serialized with `capsule.to_bytes()` / `Capsule::from_bytes()` (rmp_serde via DefaultSerialize)
```

## Acceptance Criteria

- All 271 unit tests pass
- All 65 integration tests pass
- `make lint` passes
- MockAOClient uses same format as real AO contract
