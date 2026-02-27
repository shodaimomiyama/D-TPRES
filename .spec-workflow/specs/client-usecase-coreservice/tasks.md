# Tasks Document: Client UseCase CoreService

## Overview

CryptoService（Shamir + Umbral TPRE + AES-GCM）、StorageService（Arweave操作）、ServiceError（構造化エラー）の実装タスク。

---

- [x] 1. Add cryptographic dependencies to Cargo.toml
  - File: client/Cargo.toml
  - Add `umbral-pre = "0.11"` for Umbral TPRE
  - Add `shamirsecretsharing = "0.1"` for Shamir Secret Sharing
  - Add `aes-gcm = "0.10"` for AES-GCM symmetric encryption
  - Add `zeroize = { version = "1.8", features = ["derive"] }` for memory safety
  - Add `subtle = "2.6"` for constant-time operations
  - Add `thiserror = "1"` for error handling
  - Add `bincode = "1.3"` for binary serialization
  - Add `rand = "0.8"` for random number generation
  - Purpose: Provide cryptographic libraries required for CoreService implementation
  - _Leverage: existing Cargo.toml structure_
  - _Requirements: 0 (Dependencies)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer | Task: Add all required cryptographic dependencies to client/Cargo.toml following the design document specifications | Restrictions: Only add dependencies, do not modify existing dependencies, ensure versions match design spec | Success: `cargo check` passes with all new dependencies resolved. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 2. Create ServiceError in usecase/error.rs
  - File: client/src/usecase/error.rs
  - Define `ServiceError` enum with `Business` and `System` variants
  - Define `BusinessException` enum with `ValidationError`, `ResourceNotFound`, `ThresholdNotMet`, `InvalidOperation`
  - Define `SystemException` enum with `CryptoError`, `StorageError`, `SerializationError`, `InternalError`
  - Implement `is_recoverable()` method on ServiceError
  - Implement `From<DomainError>` for ServiceError conversion
  - Use `thiserror` for derive macros
  - Purpose: Provide structured error hierarchy for UseCase layer
  - _Leverage: client/src/domain/errors.rs (DomainError, DomainResult)_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in error handling | Task: Create ServiceError types in client/src/usecase/error.rs following design document, implementing BusinessException (recoverable) and SystemException (non-recoverable) variants with thiserror derive macros | Restrictions: Must use thiserror crate, implement From<DomainError>, all error messages in English, must implement is_recoverable() method | Success: All 6 acceptance criteria pass: ServiceError wraps Business/System, BusinessException has 4 variants, SystemException has 4 variants, is_recoverable() works correctly, From<DomainError> implemented, Display trait auto-derived. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 3. Create usecase module structure
  - File: client/src/usecase/mod.rs
  - Create `mod error;` declaration
  - Create `pub mod core;` declaration
  - Re-export `ServiceError` and core service types
  - Purpose: Establish UseCase layer module structure
  - _Leverage: client/src/lib.rs module patterns_
  - _Requirements: 0 (Module Structure)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer | Task: Create usecase module structure in client/src/usecase/mod.rs with proper module declarations and re-exports for error.rs and core/ submodule | Restrictions: Follow existing module patterns in the crate, ensure proper visibility modifiers | Success: Module structure compiles, ServiceError and core services are accessible via `usecase::` path. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 4. Create CryptoService trait definition
  - File: client/src/usecase/core/crypto.rs
  - Define `CryptoService` trait with `Send + Sync` bounds
  - Add Shamir methods: `split_secret_shamir`, `reconstruct_secret_shamir`
  - Add Umbral TPRE methods: `generate_keypair`, `create_pre_capsule`, `generate_kfrags`, `verify_cfrag`, `decrypt_original`, `decrypt_reencrypted`
  - Add AES-GCM methods: `encrypt_share`, `decrypt_share`
  - Define `ShamirShare` and `EncryptedShare` data types
  - All methods return `Result<T, ServiceError>`
  - Purpose: Define cryptographic operation contract for CoreService
  - _Leverage: design.md CryptoService interface specification, umbral-pre types_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in cryptographic API design | Task: Create CryptoService trait in client/src/usecase/core/crypto.rs with all method signatures from design.md, including Shamir, Umbral TPRE, and AES-GCM operations, with Send + Sync bounds | Restrictions: Trait only, no implementation in this task, all methods must be synchronous (no async), must require Send + Sync, return ServiceError on failure | Success: Trait compiles with all 10 method signatures, ShamirShare and EncryptedShare types defined, proper Send + Sync bounds enforced. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 5. Implement CryptoService Shamir operations
  - File: client/src/usecase/core/crypto.rs
  - Create `CryptoServiceImpl` struct
  - Implement `split_secret_shamir` using `shamirsecretsharing` crate
  - Implement `reconstruct_secret_shamir` using `shamirsecretsharing` crate
  - Validate threshold parameters (1 <= k <= n <= 255)
  - Return `BusinessException::ValidationError` for invalid parameters
  - Return `BusinessException::ThresholdNotMet` when insufficient shares
  - Purpose: Provide Shamir Secret Sharing functionality
  - _Leverage: shamirsecretsharing crate API, ServiceError types_
  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer with cryptographic implementation experience | Task: Implement CryptoServiceImpl struct with split_secret_shamir and reconstruct_secret_shamir methods using shamirsecretsharing crate, including proper parameter validation and error handling | Restrictions: Must validate 1 <= k <= n <= 255, return BusinessException::ValidationError for invalid params, return BusinessException::ThresholdNotMet for insufficient shares, wrap library errors in SystemException::CryptoError | Success: Shamir split with valid k,n returns n shares, reconstruct with >= k shares returns original secret, invalid params return ValidationError, insufficient shares return ThresholdNotMet. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 6. Implement CryptoService Umbral TPRE operations
  - File: client/src/usecase/core/crypto.rs
  - Implement `generate_keypair` using `umbral_pre::SecretKey::random()`
  - Implement `create_pre_capsule` using `umbral_pre::encrypt()`
  - Implement `generate_kfrags` using `umbral_pre::generate_kfrags()`
  - Implement `verify_cfrag` for cfrag verification
  - Implement `decrypt_original` for owner decryption
  - Implement `decrypt_reencrypted` for delegatee decryption with cfrags
  - Validate threshold parameters for kfrag generation
  - Purpose: Provide Umbral Proxy Re-Encryption functionality
  - _Leverage: umbral-pre 0.11 crate API, ServiceError types_
  - _Requirements: 1.5, 1.6, 1.7, 1.8, 1.9, 1.10_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer with Umbral PRE experience | Task: Implement Umbral TPRE methods in CryptoServiceImpl using umbral-pre 0.11 crate, including keypair generation, capsule creation, kfrag generation, cfrag verification, and both original and re-encrypted decryption | Restrictions: Must use umbral-pre 0.11 API correctly, validate threshold parameters for kfrags, wrap all umbral errors in SystemException::CryptoError, never expose secret keys in error messages | Success: Keypair generation works, capsule encryption/decryption works, kfrag generation with valid k,n succeeds, cfrag verification returns bool, re-encrypted decryption with threshold cfrags succeeds. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 7. Implement CryptoService AES-GCM operations
  - File: client/src/usecase/core/crypto.rs
  - Implement `encrypt_share` using `aes_gcm` crate
  - Implement `decrypt_share` using `aes_gcm` crate
  - Generate random 12-byte nonce for each encryption
  - Return ciphertext with prepended nonce
  - Validate 32-byte key length
  - Purpose: Provide symmetric encryption for share protection
  - _Leverage: aes-gcm crate API, rand crate for nonce generation_
  - _Requirements: 1.11, 1.12_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer with symmetric encryption experience | Task: Implement AES-GCM encrypt_share and decrypt_share methods in CryptoServiceImpl using aes-gcm crate, with proper nonce generation and ciphertext format | Restrictions: Must use AES-256-GCM, generate random 12-byte nonce per encryption, prepend nonce to ciphertext, validate 32-byte key input, wrap decryption failures in SystemException::CryptoError | Success: Encryption produces ciphertext with nonce, decryption recovers original plaintext, wrong key fails with CryptoError, key validation rejects non-32-byte keys. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 8. Add memory safety to CryptoService
  - File: client/src/usecase/core/crypto.rs
  - Add `#[derive(Zeroize, ZeroizeOnDrop)]` to `ShamirShare.data`
  - Create `SecretKeyWrapper` with Zeroize for secret key handling
  - Create `SymmetricKey` wrapper with Zeroize for AES keys
  - Ensure no Clone on secret-containing types
  - Add `#[zeroize(skip)]` for non-secret metadata fields
  - Purpose: Ensure cryptographic secrets are cleared from memory on drop
  - _Leverage: zeroize crate derive macros_
  - _Requirements: 1.13, 1.14_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer with security-sensitive data handling experience | Task: Add Zeroize and ZeroizeOnDrop derives to all secret-containing types in crypto.rs, create wrapper types for secret keys, ensure no Clone on secret types | Restrictions: Must use zeroize derive macros, add #[zeroize(skip)] only to non-secret fields, do NOT implement Clone for secret-containing types, use subtle crate for any secret comparisons | Success: All secret types derive Zeroize + ZeroizeOnDrop, SecretKeyWrapper and SymmetricKey wrappers created, Clone not available on secret types, memory is zeroed on drop. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 9. Create StorageService trait definition
  - File: client/src/usecase/core/storage.rs
  - Define `StorageService` trait with `#[async_trait]` and `Send + Sync` bounds
  - Add store methods: `store_capsule`, `store_encrypted_share`, `store_kfrag`, `store_cfrag`
  - Add retrieve methods: `retrieve_capsule`, `retrieve_encrypted_shares`, `retrieve_kfrags`, `retrieve_cfrags`
  - Add query methods: `query_by_tags`, `exists`
  - Define `TransactionId`, `Tag`, `TransactionInfo` types
  - All methods return `Result<T, ServiceError>`
  - Purpose: Define Arweave storage operation contract for CoreService
  - _Leverage: design.md StorageService interface specification, async_trait crate_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in async trait design | Task: Create StorageService trait in client/src/usecase/core/storage.rs with async_trait, implementing all store, retrieve, and query method signatures from design.md | Restrictions: Must use #[async_trait], require Send + Sync bounds, all methods async, return ServiceError on failure, define TransactionId/Tag/TransactionInfo types | Success: Trait compiles with all 10 method signatures as async, helper types defined, proper Send + Sync bounds enforced. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 10. Implement StorageService with Repository integration
  - File: client/src/usecase/core/storage.rs
  - Create `StorageServiceImpl` struct with Repository dependencies
  - Inject `CapsuleRepository`, `KFragRepository`, `CFragRepository` via constructor
  - Implement all store methods delegating to repositories
  - Implement all retrieve methods delegating to repositories
  - Convert DomainError to ServiceError for all operations
  - Return `BusinessException::ResourceNotFound` for missing entities
  - Purpose: Provide Arweave storage functionality via Repository pattern
  - _Leverage: client/src/repositories/*.rs, ServiceError conversion_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.11, 2.12_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer with Repository pattern experience | Task: Implement StorageServiceImpl struct in client/src/usecase/core/storage.rs with constructor injection of Repository traits, implementing all async methods by delegating to repositories | Restrictions: Must inject repositories via Box<dyn Trait>, convert all DomainError to ServiceError, return BusinessException::ResourceNotFound for missing entities, wrap storage failures in SystemException::StorageError | Success: All store operations delegate to correct repository, all retrieve operations return correct entity types, missing entities return ResourceNotFound, DomainError properly converted to ServiceError. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 11. Create core module exports
  - File: client/src/usecase/core/mod.rs
  - Create `mod crypto;` declaration
  - Create `mod storage;` declaration
  - Re-export `CryptoService`, `CryptoServiceImpl`, `ShamirShare`, `EncryptedShare`
  - Re-export `StorageService`, `StorageServiceImpl`, `TransactionId`, `Tag`, `TransactionInfo`
  - Purpose: Consolidate CoreService exports for UseCase layer
  - _Leverage: existing module patterns_
  - _Requirements: 0 (Module Structure)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer | Task: Create core module structure in client/src/usecase/core/mod.rs with proper module declarations and re-exports for crypto.rs and storage.rs | Restrictions: Follow existing module patterns, ensure proper visibility modifiers, re-export all public types | Success: Module structure compiles, all CoreService types accessible via `usecase::core::` path. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 12. Update lib.rs to export usecase module
  - File: client/src/lib.rs
  - Add `pub mod usecase;` declaration
  - Ensure usecase module is publicly accessible
  - Purpose: Make UseCase CoreService available for external use
  - _Leverage: client/src/lib.rs existing module structure_
  - _Requirements: 0 (Module Export)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer | Task: Update client/src/lib.rs to export the new usecase module by adding pub mod usecase declaration | Restrictions: Only add the module declaration, maintain existing module structure | Success: lib.rs compiles, usecase module is accessible from outside the crate. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 13. Write CryptoService unit tests
  - File: client/src/usecase/core/crypto.rs (tests module)
  - Test Shamir: `test_shamir_split_valid_params`, `test_shamir_reconstruct_with_threshold`, `test_shamir_invalid_threshold`, `test_shamir_insufficient_shares`
  - Test Umbral: `test_keypair_generation`, `test_pre_encrypt_decrypt`, `test_kfrag_generation_valid`, `test_decrypt_reencrypted`
  - Test AES-GCM: `test_aes_gcm_encrypt_decrypt`, `test_aes_gcm_wrong_key`, `test_aes_gcm_invalid_key_length`
  - Test Memory: `test_zeroize_on_drop`
  - Purpose: Verify all CryptoService operations work correctly
  - _Leverage: existing test patterns, assert macros_
  - _Requirements: 1.1-1.14 (Test Coverage)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Test Engineer | Task: Write comprehensive unit tests for CryptoService in client/src/usecase/core/crypto.rs covering all Shamir, Umbral TPRE, AES-GCM, and memory safety operations as specified in requirements test coverage | Restrictions: Use #[cfg(test)] module, test both success and failure cases, verify error types match expected BusinessException/SystemException variants | Success: All 20 test functions pass, coverage includes all acceptance criteria, error scenarios properly tested. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 14. Write StorageService unit tests
  - File: client/src/usecase/core/storage.rs (tests module)
  - Create mock Repository implementations for testing
  - Test Store: `test_store_capsule`, `test_store_encrypted_share`, `test_store_kfrag`, `test_store_cfrag`
  - Test Retrieve: `test_retrieve_capsule`, `test_retrieve_not_found`, `test_retrieve_kfrags_multiple`
  - Test Query: `test_query_by_tags`, `test_exists_true`, `test_exists_false`
  - Test Error: `test_domain_error_conversion`, `test_storage_error_propagation`
  - Purpose: Verify all StorageService operations work correctly
  - _Leverage: mockall or manual mock implementations, tokio::test_
  - _Requirements: 2.1-2.12 (Test Coverage)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Test Engineer with async testing experience | Task: Write comprehensive unit tests for StorageService in client/src/usecase/core/storage.rs with mock Repository implementations, covering all store, retrieve, query, and error handling operations | Restrictions: Use #[tokio::test] for async tests, create mock repositories, test both success and failure cases, verify error types match expected ServiceError variants | Success: All 15 test functions pass, mock repositories properly simulate behavior, coverage includes all acceptance criteria. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 15. Write ServiceError unit tests
  - File: client/src/usecase/error.rs (tests module)
  - Test: `test_is_recoverable_business`, `test_is_recoverable_system`
  - Test: `test_domain_error_not_found_conversion`, `test_domain_error_validation_conversion`
  - Test: `test_display_business_exception`, `test_display_system_exception`
  - Test: `test_error_from_crypto`, `test_error_from_storage`
  - Purpose: Verify ServiceError hierarchy and conversions work correctly
  - _Leverage: existing test patterns_
  - _Requirements: 3.1-3.6 (Test Coverage)_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Test Engineer | Task: Write comprehensive unit tests for ServiceError in client/src/usecase/error.rs covering is_recoverable(), From<DomainError> conversion, and Display formatting | Restrictions: Use #[cfg(test)] module, test all error variants, verify proper Display output | Success: All 11 test functions pass, is_recoverable() correctly identifies business vs system errors, From conversions work correctly. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._

- [x] 16. Verify compilation and run all tests
  - File: client/ (entire crate)
  - Run `cargo check` to verify all code compiles
  - Run `cargo clippy` to verify no lint warnings
  - Run `cargo test` to ensure all tests pass
  - Verify Send + Sync bounds are enforced at compile time
  - Purpose: Final verification of all CoreService implementations
  - _Leverage: Makefile commands (make check, make clippy, make test)_
  - _Requirements: All_
  - _Prompt: Implement the task for spec client-usecase-coreservice, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer | Task: Verify the entire client crate compiles successfully with cargo check, passes clippy with no warnings, and all tests pass with cargo test | Restrictions: Do not modify any code, only verify compilation and test results, report any failures | Success: cargo check passes, cargo clippy passes with no warnings, cargo test shows all 46 tests passing, no compilation warnings related to new CoreService code. Before starting, edit tasks.md to change `[ ]` to `[-]` for this task. After completion, use log-implementation tool to record details, then change `[-]` to `[x]`._
