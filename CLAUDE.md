# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FORMIX (Deterministic Threshold Proxy Re-Encryption System) is a decentralized key management layer that implements threshold proxy re-encryption for Arweave storage. The system combines:

- **Arweave**: Immutable storage for encrypted data and capsules
- **AO Network**: WebAssembly-based distributed execution environment
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

### Run Single Test
```bash
# Run specific test
cargo test test_name

# Run tests for specific module
cargo test module_name

# Run tests with output
cargo test -- --nocapture
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
client/src/
├── actions/            # Actions Layer - Client API entry points
│   ├── client.rs       # share, recover, generateKeyPair actions
│   ├── builder.rs      # ActionsBuilder for DI setup
│   ├── di.rs           # Type aliases and DI container
│   └── options.rs      # Action options/parameters
├── controller/         # Controller Layer - Validation & DTO extraction
│   ├── validator.rs    # ShareValidator, RecoverValidator
│   └── extractor.rs    # ShareExtractor, RecoverExtractor
├── usecase/            # UseCase Layer - Business logic
│   ├── core/           # Core services (basic operations)
│   │   ├── crypto.rs   # CoreCryptoService (TPRE, Shamir)
│   │   ├── storage.rs  # ArweaveStorageService
│   │   └── contract_storage.rs  # ContractStorage (AO state)
│   ├── service/        # Service layer (wrapping core)
│   │   ├── crypto_service.rs    # ServiceCryptoService
│   │   └── storage_service.rs   # ServiceStorageService
│   └── workflow/       # Workflow services (phase orchestration)
│       ├── secret_sharing_service.rs   # Phase 1: split & distribute
│       └── secret_recovery_service.rs  # Phase 3: recover
├── domain/             # Domain Layer - Entities & repository interfaces
│   ├── entities/       # Secret, Capsule, KFrag, CFrag, ShareCollection
│   └── value_objects/  # IDs, KeyPair, SecretData, SymmetricKey
├── repositories/       # Repository interfaces (DIP)
├── adapter/            # Infrastructure Layer
│   ├── repository_impl/  # Arweave-backed repository implementations
│   └── external/         # ArweaveClient, AOClient, MockAOClient
└── lib.rs              # Library exports
```

### Multi-Role Wasm Design
The core architecture implements a single Rust codebase (`formix`) that compiles to WebAssembly and runs on AO with different roles:

- **Owner-Process (Pᴼ)**: Manages secret key shares and re-encryption key generation
- **Holder-Process (Hⱼ)**: Stores key fragments (kFrag) and performs re-encryption
- **Requester-Process (R-Proc)**: Coordinates access requests and collects cipher fragments

All processes use the same Wasm binary deployed to Arweave, with role differentiation through message routing and process tags.

### Key Components
- **Threshold Proxy Re-Encryption**: Using `umbral-pre` library for cryptographic operations
- **Secret Sharing**: Shamir's Secret Sharing for k-of-n threshold schemes
- **Browser Integration**: WebCrypto API for client-side encryption/decryption

### Cryptographic Flow
1. **Phase 1**: Secret splitting and initial distribution (client-side)
   - 1.1: Shamir secret sharing (k-of-n)
   - 1.2: Umbral capsule creation and kFrag generation
   - 1.3: Encrypted data storage on Arweave
2. **Phase 2**: Key fragment distributed management (AO Network)
   - 2.1: Owner-Process selects Holders and distributes kFrags
   - 2.2: Holder-Process stores kFrags and performs re-encryption
   - 2.3: cFrag generation and storage
3. **Phase 3**: Secret recovery (client-side)
   - 3.1: Requester-Process collects cFrags
   - 3.2: Capsule decryption and fragment combination
   - 3.3: Shamir interpolation for secret recovery

## Rust Configuration

- **Edition**: 2024 (Cargo.toml specifies edition = "2024")
- **Toolchain**: Fixed to Rust 1.86.0 (see rust-toolchain.toml)
- **Formatting**: 100 character line width, 4 spaces, Unix newlines
- **Linting**: Aggressive clippy configuration with specific allowances for development phase

## AO Execution Constraints

FORMIX targets two AO runtimes with different state models:

### HyperBEAM-native (`ao/contracts/src/`) — WASM memory snapshots

HyperBEAM's `~wasm64@1.0` device snapshots the entire WASM linear memory after
each message and restores it before the next. This means **global Rust statics
persist across invocations** — no explicit load/save to Arweave is needed.

- State is kept in `static` globals using `UnsafeCell` (safe because WASM is single-threaded)
- No CosmWasm host functions (`db_read`/`db_write`)
- Entry point: `handle(msg_ptr, msg_len) -> i32`

### CWAO (`ao_cwao/contracts/src/`) — CosmWasm-style persistence

The legacy CWAO runtime is stateless between messages:

1. **Memory Non-Persistence Between Messages**
   - Each message execution starts with clean memory
   - All state must be explicitly loaded from storage (Arweave)
   - Process state cannot rely on in-memory variables between messages

2. **Distributed Compute Units**
   - Messages may be processed by different Compute Units
   - No shared memory between executions
   - State consistency must be maintained through persistence

### Shared Constraints (both runtimes)

1. **Synchronous-Only Execution**
   - async/await is not available in AO environment
   - All operations must be blocking
   - Error handling must be synchronous

2. **Message-Driven Architecture**
   - All processing is triggered by messages
   - UseCase handlers are entry points
   - State transitions must be atomic per message

## Directory-Specific Async Policy

FORMIX has two Rust codebases with different async constraints:

| Directory | async/await | tokio | Reason |
|-----------|------------|-------|--------|
| `ao/contracts/src/` | Prohibited | Not available | AO WASM single-threaded constraint |
| `ao_cwao/contracts/src/` | Prohibited | Not available | AO WASM single-threaded constraint |
| `client/` | Required for I/O | Available (non-wasm32) | Client-side library with network operations |

### `ao/contracts/src/` and `ao_cwao/contracts/src/` - Sync Only
The "AO Execution Constraints" above apply exclusively to these directories.

### `client/` - Async for Network I/O
- `AOClient` trait uses `#[async_trait]` - all AO Network calls are async
- Network operations (Arweave, AO) must use async/await, not nested runtimes
- **Anti-pattern**: `tokio::runtime::Builder::new_current_thread().block_on()` inside sync methods — causes "runtime inside runtime" panic
- Propagate async up the call chain instead

## Security and Cryptographic Requirements

### Memory Management for Secrets
- **Always use Zeroize**: All structs containing secrets must derive `Zeroize` and `ZeroizeOnDrop`
- **No Clone for Secrets**: Secret-containing types should not implement `Clone`
- **Explicit Secret Types**: Use `secrecy::Secret<T>` or similar wrappers for clarity

### Constant-Time Operations
- **Use subtle crate**: For comparisons that must be constant-time
- **Avoid secret-dependent branching**: No if statements based on secret values
- **Use crypto libraries**: Don't implement cryptographic primitives yourself

### Process Role Separation
- **Single Role Per Process**: Each AO process instance has exactly one role
- **Role-Specific Handlers**: UseCase handlers are separated by role
- **No Cross-Role Access**: Handlers cannot access other roles' functionality

## Development Status

### Current Implementation
- **Documentation**: Comprehensive architectural design and specifications
- **Project Structure**: Well-defined layered architecture with clear separation of concerns
- **Build System**: Makefile with Rust commands and MCP server management
- **Toolchain**: Rust 1.86.0 with edition 2024 configuration

### Implementation Phase
The codebase is in **active development phase** with:
- Domain layer complete: entities (Secret, Capsule, KFrag, CFrag, ShareCollection), value objects, repository interfaces
- Service layer implemented: CryptoService (Umbral + Shamir), StorageService (Arweave + AO Contract), Workflow services
- Application layer: Actions (share, recover, generateKeyPair), Controller (validators, extractors)
- Infrastructure: ArweaveClient, AOClient (Production + Mock), repository implementations
- 60 tests passing (unit + integration)

### Development Targets
- **Primary**: WebAssembly compilation for AO Network deployment
- **Secondary**: Browser integration via WebCrypto API and WASM bindings

## Claude Rules and Modes

This project uses Claude-specific rules and modes for AI-assisted development:

### Available Modes
- **Default Mode**: General Rust development with AO constraints
- **rust-test Mode**: For implementing or modifying tests
- **pr Mode**: For creating Pull Requests

### Key Rules from `.claude/rules/`
- **Comment Convention**: Only write comments explaining "why", not "what"
- **Import Resolution**: Use absolute imports with blank lines between standard/third-party and internal modules
- **Entity Encapsulation**: Domain entities must have private fields with constructor validation
- **Role Separation**: Strict separation between Owner/Holder/Requester process roles
- **Stateless Patterns**: All handlers must follow load-process-save pattern for AO compatibility

### Security Restrictions
The project has specific security restrictions in `.claude/settings.json`:
- Limited bash command permissions
- No access to secrets, tokens, or key files
- Restricted file access patterns
- No database access

## Documentation

Project documentation is available in the `docs/` directory:
- `docs/PRD.md` - Product Requirements Document with system specifications
- `docs/status.md` - Current implementation status
- `docs/architecture/` - System architecture and design philosophy
- `docs/operations/` - Testing and workflow documentation
- `docs/client/` - クライアントライブラリのレイヤー別リファレンス
  - `domain.md` - エンティティ、値オブジェクト、エラー型
  - `repositories.md` - リポジトリインターフェース（DIP）
  - `usecase.md` - Core / Service / Workflow サービス
  - `controller.md` - バリデーター、エクストラクター
  - `actions.md` - ActionsContainer、ビルダー、DTpresClient
  - `adapter.md` - Arweaveリポジトリ、AOクライアント、外部連携
- `docs/contracts/` - AOスマートコントラクトリファレンス
  - `contracts_overview.md` - メッセージ、状態、ハンドラー、再暗号化フロー
