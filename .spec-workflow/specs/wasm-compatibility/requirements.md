# WASM Compatibility Requirements

## Overview

This specification addresses WASM (WebAssembly) compatibility issues identified in PR #48 code review. The `client/` module is designed to be compiled to `wasm32-unknown-unknown` target for browser execution, but was using native-only dependencies.

## Background

### Problem Statement

PR #48 reviewer (Codex) identified that `reqwest` with `rustls-tls` feature in `client/Cargo.toml` cannot compile for `wasm32-unknown-unknown` target because native TLS is unavailable in browsers.

### Architecture Context

From steering documents:
- `client/` is designed for **browser WASM** as primary execution environment
- Target: `wasm32-unknown-unknown`
- Build artifact: `client/` → WASM + JS bindings → `client/pkg/`

## User Stories

### US-1: WASM Build Success

**As a** developer building the FORMIX client library
**I want** `cargo check --target wasm32-unknown-unknown` to succeed
**So that** the client can be compiled and used in browser environments

**Acceptance Criteria:**
- WASM build completes without errors
- All repository traits are WASM-compatible
- No native-only dependencies block compilation

### US-2: Native Build Compatibility

**As a** developer running tests
**I want** `cargo check` (native target) to also succeed
**So that** I can run unit tests and integration tests locally

**Acceptance Criteria:**
- Native build completes without errors
- All features available on native remain functional
- Tests can be executed on native target

### US-3: No Dead Code Warnings

**As a** developer maintaining code quality
**I want** dead_code warnings to be resolved
**So that** the codebase remains clean and maintainable

**Acceptance Criteria:**
- GraphQL response struct fields are properly annotated
- No compiler warnings related to unused code in modified files

## Functional Requirements

### FR-1: Platform-Specific Dependencies

The build system must support different dependencies for WASM and native targets:

| Dependency | WASM Target | Native Target |
|------------|-------------|---------------|
| reqwest | json feature only | json + rustls-tls |
| tokio | Not included | time feature |
| getrandom | js feature | default |

### FR-2: Conditional Compilation

Code that uses native-only features must be conditionally compiled:

- `tokio::time::sleep` - native only
- `reqwest::ClientBuilder::timeout()` - native only
- `#[async_trait]` with Send bounds - native only

### FR-3: Repository Trait Compatibility

Repository traits must not require `Send + Sync` bounds on WASM target:

- WASM uses single-threaded execution
- `Rc<RefCell>` patterns in WASM futures are not `Send`
- Conditional trait bounds required

## Non-Functional Requirements

### NFR-1: Build Performance

Both WASM and native builds should complete in reasonable time without additional complexity.

### NFR-2: Code Maintainability

Conditional compilation should be clear and well-documented to aid future maintenance.

## Dependencies

- `reqwest` v0.12 - HTTP client
- `async-trait` - Async trait support
- `getrandom` v0.2 - Random number generation (with js feature for WASM)
- `tokio` v1 - Async runtime (native only)

## References

- PR #48: https://github.com/shodaimomiyama/FORMIX/pull/48
- reqwest WASM support: https://docs.rs/reqwest/
