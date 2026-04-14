# Capsule Serialization Alignment Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align Capsule serialization from bincode to Umbral's official rmp_serde API (`to_bytes()`/`from_bytes()`) so client and AO contract use the same format.

**Architecture:** 3 production code changes (crypto.rs serialize + deserialize, mock_ao deserialize) + 2 test fixture updates + MEMORY.md update. All changes are in `client/` — AO contract unchanged.

**Tech Stack:** Rust, umbral-pre (DefaultSerialize/DefaultDeserialize traits), rmp_serde

**Spec:** `docs/superpowers/specs/2026-04-14-capsule-serialization-alignment-design.md`

**Issue:** #86 Phase A

---

## Chunk 1: Production code + tests

### Task 1: Create feature branch

**Files:** None (git operation)

- [ ] **Step 1: Create branch from development**

```bash
git checkout development
git pull origin development
git checkout -b fix/86-capsule-serialization-alignment
```

- [ ] **Step 2: Verify branch**

Run: `git branch --show-current`
Expected: `fix/86-capsule-serialization-alignment`

---

### Task 2: Fix Capsule serialize in crypto.rs (Phase 1 path)

**Files:**
- Modify: `client/src/usecase/core/crypto.rs:513-514`

- [ ] **Step 1: Run existing tests to confirm baseline passes**

Run: `cd client && cargo test 2>&1 | tail -5`
Expected: All tests pass (271+ unit, 65+ integration)

- [ ] **Step 2: Change serialize from bincode to to_bytes()**

In `client/src/usecase/core/crypto.rs`, replace lines 513-514:

```rust
// BEFORE:
let serialized_capsule: Vec<u8> = bincode::serialize(&umbral_capsule)
    .map_err(|_| ServiceError::crypto_error("Failed to serialize capsule"))?;

// AFTER:
let serialized_capsule: Vec<u8> = umbral_capsule
    .to_bytes()
    .map_err(|_| ServiceError::crypto_error("Failed to serialize capsule"))?
    .to_vec();
```

Note: `to_bytes()` is from the `DefaultSerialize` trait (already imported at line 14). Returns `Result<Box<[u8]>, rmp_serde::encode::Error>`. The `.to_vec()` converts `Box<[u8]>` to `Vec<u8>`.

- [ ] **Step 3: Run tests — expect failures**

Run: `cd client && cargo test 2>&1 | grep -E "FAILED|failures" | head -10`
Expected: Tests that deserialize capsules with bincode will fail because bytes are now rmp_serde format.

---

### Task 3: Fix Capsule deserialize in crypto.rs (Phase 3 path)

**Files:**
- Modify: `client/src/usecase/core/crypto.rs:377-380`

- [ ] **Step 1: Change deserialize from bincode to from_bytes()**

In `client/src/usecase/core/crypto.rs`, replace the `deserialize_capsule` method (lines 377-380):

```rust
// BEFORE:
fn deserialize_capsule(&self, capsule: &Capsule) -> ServiceResult<umbral_pre::Capsule> {
    bincode::deserialize(&capsule.capsule_bytes)
        .map_err(|_| ServiceError::crypto_error("Failed to deserialize capsule"))
}

// AFTER:
fn deserialize_capsule(&self, capsule: &Capsule) -> ServiceResult<umbral_pre::Capsule> {
    umbral_pre::Capsule::from_bytes(&capsule.capsule_bytes)
        .map_err(|_| ServiceError::crypto_error("Failed to deserialize capsule"))
}
```

Note: `from_bytes()` is from the `DefaultDeserialize` trait (already imported at line 14). Returns `Result<Self, rmp_serde::decode::Error>`.

- [ ] **Step 2: Run tests — some should recover**

Run: `cd client && cargo test 2>&1 | grep -E "FAILED|test result" | head -10`
Expected: E2E roundtrip tests (serialize+deserialize both rmp_serde) should pass again. Tests using bincode in fixtures still fail.

---

### Task 4: Fix MockAOClient Capsule deserialize

**Files:**
- Modify: `client/src/adapter/external/mock_ao/client.rs:258-263`

- [ ] **Step 1: Change deserialize from bincode to from_bytes()**

In `client/src/adapter/external/mock_ao/client.rs`, replace lines 258-263:

```rust
// BEFORE:
// Client serializes capsules with bincode (see crypto.rs:create_pre_capsule)
let capsule: umbral_pre::Capsule =
    bincode::deserialize(capsule_bytes).map_err(|e| AOCommunicationError::ExecutionError {
        process_id: String::new(),
        details: format!("Capsule deserialize failed: {e:?}"),
    })?;

// AFTER:
let capsule: umbral_pre::Capsule =
    umbral_pre::Capsule::from_bytes(capsule_bytes).map_err(|e| {
        AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: format!("Capsule deserialize failed: {e:?}"),
        }
    })?;
```

Note: `DefaultDeserialize` trait is already imported at line 17.

- [ ] **Step 2: Run tests — more should recover**

Run: `cd client && cargo test 2>&1 | grep -E "FAILED|test result" | head -10`
Expected: Most tests pass. Only test fixtures using `bincode::serialize(&capsule)` still fail.

---

### Task 5: Fix test fixture in contract_storage.rs

**Files:**
- Modify: `client/src/usecase/core/contract_storage.rs:394`

- [ ] **Step 1: Change test helper capsule serialization**

In `client/src/usecase/core/contract_storage.rs`, replace line 394:

```rust
// BEFORE:
let capsule_bytes = bincode::serialize(&capsule).unwrap();

// AFTER:
let capsule_bytes = capsule.to_bytes().unwrap().to_vec();
```

Note: This is inside `#[cfg(test)] fn create_test_crypto_data()`. The `DefaultSerialize` trait needs to be in scope. Check imports at top of test module — add `use umbral_pre::DefaultSerialize;` if not present.

- [ ] **Step 2: Verify import exists**

Check test module imports. If `DefaultSerialize` is not imported, add it:
```rust
use umbral_pre::DefaultSerialize;
```

---

### Task 6: Fix test fixture in mock_ao test

**Files:**
- Modify: `client/tests/unit/adapter/external/mock_ao/client.rs:52`

- [ ] **Step 1: Change test helper capsule serialization**

In `client/tests/unit/adapter/external/mock_ao/client.rs`, replace line 52:

```rust
// BEFORE:
let capsule_bytes = bincode::serialize(&capsule).unwrap();

// AFTER:
let capsule_bytes = capsule.to_bytes().unwrap().to_vec();
```

Note: Check imports at top of file — add `use umbral_pre::DefaultSerialize;` if not present.

- [ ] **Step 2: Verify import exists**

Check test file imports. If `DefaultSerialize` is not imported, add it.

- [ ] **Step 3: Run ALL tests — expect all pass**

Run: `cd client && cargo test 2>&1 | tail -5`
Expected: ALL tests pass (271+ unit, 65+ integration). Zero failures.

- [ ] **Step 4: Run lint**

Run: `cd client && cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings 2>&1 | tail -5`
Expected: No warnings (warnings are errors via `-D warnings`).

- [ ] **Step 5: Run format**

Run: `cd client && cargo fmt --all 2>&1`
Expected: No output (already formatted) or auto-formatted.

- [ ] **Step 6: Commit all changes**

```bash
git add client/src/usecase/core/crypto.rs client/src/adapter/external/mock_ao/client.rs client/src/usecase/core/contract_storage.rs client/tests/unit/adapter/external/mock_ao/client.rs
git commit -m "fix(client): align Capsule serialization to Umbral official API (rmp_serde)

Replace bincode serialize/deserialize of umbral_pre::Capsule with
official to_bytes()/from_bytes() API (rmp_serde via DefaultSerialize).

Fixes serialization mismatch between client (bincode) and AO contract
(Capsule::from_bytes / rmp_serde), which caused E2E failure on real
AO HyperBEAM while MockAOClient masked the bug.

Changed:
- crypto.rs: create_pre_capsule() and deserialize_capsule()
- mock_ao/client.rs: perform_reencryption()
- Test fixtures: contract_storage.rs and mock_ao test helper

Refs: #86 Phase A

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Chunk 2: MEMORY.md update

### Task 7: Update MEMORY.md

**Files:**
- Modify: `/Users/momiyamashodai/.claude/projects/-Users-momiyamashodai-Develop-MyProject-D-TPRES-FORMIX/memory/MEMORY.md`

- [ ] **Step 1: Update Capsule serialization convention**

Change the first item under "## Serialization Format Conventions":

```markdown
## Serialization Format Conventions
- **Capsule**: Serialized with `capsule.to_bytes()` / `Capsule::from_bytes()` (rmp_serde via DefaultSerialize)
```

- [ ] **Step 2: Update Mock vs Production Format Alignment section**

Remove the first bullet point about Mock using bincode for Capsule (no longer relevant):

```markdown
## Mock vs Production Format Alignment
- MockAOClient and AO contract both use `Capsule::from_bytes()` (rmp_serde) — formats now aligned
- Test helpers must use `capsule.to_bytes().unwrap().to_vec()` not `bincode::serialize(&capsule)`
```
