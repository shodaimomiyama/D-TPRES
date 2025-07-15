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

### MCP Server Management
```bash
# Setup all MCP servers (initial setup only)
make mcp-setup

# Start context7 MCP server in background
make start-context7

# Stop context7 MCP server
make stop-context7

# View context7 MCP server logs
make logs-context7

# Check context7 MCP server status
make status-context7

# Show MCP help
make mcp-help
```

### Individual Cargo Commands
```bash
cargo check
cargo fmt --all
cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings
cargo test
```

## Code Architecture

### Layered Architecture Design
The codebase follows a clean layered architecture with clear separation of concerns:

```
src/
├── usecase/         # UseCase Layer - AO message handlers by role
│   └── handlers/    # Owner, Holder, Requester, Common handlers
├── controller/      # Controller Layer - Message processing & routing
├── service/         # Service Layer - Business logic (Workflow + Core services)
│   ├── workflow/    # Phase orchestration services
│   └── core/        # Basic operation services
├── domain/          # Domain Layer - Entities & repository interfaces
│   ├── entities/    # Pure data structures
│   ├── repositories/ # Repository interfaces (DIP)
│   └── value_objects/ # Domain value objects
├── infrastructure/ # Infrastructure Layer - Technical implementations
│   ├── repositories/ # Repository implementations
│   └── external/    # External system adapters
├── crypto/         # Cryptographic utilities
└── utils/          # Shared utilities
```

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

## Development Status

### Current Implementation
- **Documentation**: Comprehensive architectural design and specifications
- **Project Structure**: Well-defined layered architecture with clear separation of concerns
- **Build System**: Makefile with Rust commands and MCP server management
- **Toolchain**: Rust 1.86.0 with edition 2024 configuration

### Implementation Phase
The codebase is in **early development phase** with:
- Placeholder implementations in main.rs and di.rs
- Empty domain/, service/, and usecase/ directories ready for implementation
- Extensive documentation in docs/ directory covering all architectural aspects
- MCP server infrastructure for development tooling

### Development Targets
- **Primary**: WebAssembly compilation for AO Network deployment
- **Secondary**: Browser integration via WebCrypto API and WASM bindings
- **Future**: Smart contract integration via elciao bridge

## Documentation

Extensive project documentation is available in the `docs/` directory:
- `docs/development/architecture/` - System architecture and design philosophy
- `docs/development/domain/` - Domain entities and repository designs
- `docs/development/service/` - Service layer specifications
- `docs/development/usecase/` - UseCase handlers for each role
- `docs/development/lifecycle/` - Process and access lifecycles
- `docs/features/` - Feature specifications for each component
