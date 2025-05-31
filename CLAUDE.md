# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

D-TPRES (Deterministic Threshold Proxy Re-Encryption System) is a decentralized key management layer that implements threshold proxy re-encryption for Arweave storage. The system combines:

- **Arweave**: Immutable storage for encrypted data and capsules
- **AO Network**: WebAssembly-based distributed execution environment
- **EVM Smart Contracts**: Deterministic access control verification
- **Threshold Proxy Re-Encryption (TPRE)**: k-of-n distributed key management using Umbral

## Development Commands

### Build and Development
```bash
# Check code compilation
make check

# Format code
make fmt

# Run linter with specific clippy rules
make clippy

# Run both formatting and linting
make lint

# Run tests
make test

# Run all checks (check, lint, test)
make all
```

### Individual Cargo Commands
```bash
cargo check
cargo fmt --all
cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings
cargo test
```

## Code Architecture

### Multi-Role Wasm Design
The core architecture implements a single Rust codebase (`dtpres_core`) that compiles to WebAssembly and runs on AO with different roles:

- **Owner-Process (Pᴼ)**: Manages secret key shares and re-encryption key generation
- **Holder-Process (Hⱼ)**: Stores key fragments (kFrag) and performs re-encryption
- **Requester-Process (R-Proc)**: Coordinates access requests and collects cipher fragments

All processes use the same Wasm binary deployed to Arweave, with role differentiation through message routing and process tags.

### Key Components
- **Threshold Proxy Re-Encryption**: Using `umbral-pre` library for cryptographic operations
- **Secret Sharing**: Shamir's Secret Sharing for k-of-n threshold schemes
- **EVM Bridge**: `elciao` integration for Ethereum event verification
- **Browser Integration**: WebCrypto API for client-side encryption/decryption

### Cryptographic Flow
1. **Phase 0**: Process spawning and key preparation
2. **Phase 1**: Secret splitting and public storage on Arweave
3. **Phase 2**: Access request and EVM verification via smart contracts
4. **Phase 3**: Re-encryption key fragmentation and holder assignment
5. **Phase 4**: k-of-n proxy re-encryption by holders
6. **Phase 5**: Client decryption and secret reconstruction

## Rust Configuration

- **Edition**: 2024 (Cargo.toml specifies edition = "2024")
- **Toolchain**: Fixed to Rust 1.86.0 (see rust-toolchain.toml)
- **Formatting**: 100 character line width, 4 spaces, Unix newlines
- **Linting**: Aggressive clippy configuration with specific allowances for development phase

## Development Targets

The system is designed for compilation to WebAssembly for deployment on AO compute units. The current codebase is in early development phase with placeholder implementations in main.rs and di.rs.
