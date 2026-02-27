# Tasks Document: Client Domain Repository Interface

- [x] 1. Add async-trait dependency to Cargo.toml
  - File: client/Cargo.toml
  - Add `async-trait = "0.1"` to dependencies section
  - Purpose: Enable async fn in trait definitions for Repository interfaces
  - _Leverage: existing Cargo.toml structure_
  - _Requirements: 0_
  - _Prompt: Role: Rust Developer | Task: Add async-trait crate dependency to client/Cargo.toml for enabling async methods in trait definitions | Restrictions: Only add the dependency, do not modify other parts of Cargo.toml | Success: Cargo.toml compiles successfully with the new dependency_

- [x] 2. Create repositories module with base Repository trait
  - File: client/src/repositories/mod.rs
  - Define generic `Repository<T, ID>` trait with `async_trait`
  - Implement `save`, `find_by_id`, `delete`, `exists`, `find_by_ids` methods
  - Add `Send + Sync` bounds for thread safety
  - Export all repository interface modules
  - Purpose: Provide common CRUD operations abstracted for all entity repositories
  - _Leverage: client/src/domain/errors.rs (DomainResult, DomainError)_
  - _Requirements: 0_
  - _Prompt: Role: Rust Developer specializing in trait design and async patterns | Task: Create base Repository trait in client/src/repositories/mod.rs with async_trait, implementing generic CRUD operations (save, find_by_id, delete, exists, find_by_ids) with Send + Sync bounds, using DomainResult from domain/errors.rs | Restrictions: Must use async_trait crate, all methods must be async, must require Send + Sync for thread safety | Success: Trait compiles successfully, all methods are async, Send + Sync constraints are enforced at compile time_

- [x] 3. Create SecretRepository trait
  - File: client/src/repositories/secret_interface.rs
  - Define `SecretRepository` trait extending `Repository<Secret, SecretId>`
  - Secret is the aggregate root with no additional query methods needed
  - Purpose: Provide persistence abstraction for Secret entity (aggregate root)
  - _Leverage: client/src/repositories/mod.rs (Repository trait), client/src/domain/entities/secret.rs, client/src/domain/value_objects/ids.rs_
  - _Requirements: 1_
  - _Prompt: Role: Rust Developer with DDD experience | Task: Create SecretRepository trait in client/src/repositories/secret_interface.rs extending the base Repository trait for Secret entity, which is the aggregate root in FORMIX | Restrictions: Must extend Repository<Secret, SecretId>, use async_trait, no additional methods needed beyond base trait | Success: Trait compiles, properly extends base Repository, uses correct entity and ID types_

- [x] 4. Create ShareCollectionRepository trait
  - File: client/src/repositories/share_interface.rs
  - Define `ShareCollectionRepository` trait extending `Repository<ShareCollection, ShareCollectionId>`
  - Add `find_by_secret_id` method for querying by parent Secret
  - Purpose: Provide persistence abstraction for ShareCollection entity with parent reference query
  - _Leverage: client/src/repositories/mod.rs, client/src/domain/entities/share.rs, client/src/domain/value_objects/ids.rs_
  - _Requirements: 2_
  - _Prompt: Role: Rust Developer with repository pattern experience | Task: Create ShareCollectionRepository trait in client/src/repositories/share_interface.rs extending base Repository trait, adding find_by_secret_id method for querying ShareCollection by parent SecretId | Restrictions: Must extend Repository<ShareCollection, ShareCollectionId>, use async_trait, find_by_secret_id returns DomainResult<Option<ShareCollection>> | Success: Trait compiles, extends base Repository, includes find_by_secret_id method with correct signature_

- [x] 5. Create CapsuleRepository trait
  - File: client/src/repositories/capsule_interface.rs
  - Define `CapsuleRepository` trait extending `Repository<Capsule, CapsuleId>`
  - Add `find_by_secret_id` method for querying by parent Secret
  - Purpose: Provide persistence abstraction for Capsule entity (Umbral PRE capsule)
  - _Leverage: client/src/repositories/mod.rs, client/src/domain/entities/capsule.rs, client/src/domain/value_objects/ids.rs_
  - _Requirements: 3_
  - _Prompt: Role: Rust Developer with cryptographic domain experience | Task: Create CapsuleRepository trait in client/src/repositories/capsule_interface.rs extending base Repository trait, adding find_by_secret_id method for querying Capsule by parent SecretId | Restrictions: Must extend Repository<Capsule, CapsuleId>, use async_trait, find_by_secret_id returns DomainResult<Option<Capsule>> | Success: Trait compiles, extends base Repository, includes find_by_secret_id method_

- [x] 6. Create KFragRepository trait
  - File: client/src/repositories/kfrag_interface.rs
  - Define `KFragRepository` trait extending `Repository<KFrag, KFragId>`
  - Add `find_by_secret_id` method returning Vec<KFrag>
  - Add `find_by_holder_index` method for specific holder lookup
  - Add `delete_by_secret_id` method for bulk deletion
  - Purpose: Provide persistence abstraction for KFrag entity (key fragment with Zeroize)
  - _Leverage: client/src/repositories/mod.rs, client/src/domain/entities/kfrag.rs, client/src/domain/value_objects/ids.rs_
  - _Requirements: 4_
  - _Prompt: Role: Rust Developer with security-sensitive data handling experience | Task: Create KFragRepository trait in client/src/repositories/kfrag_interface.rs extending base Repository trait, adding find_by_secret_id (returns Vec), find_by_holder_index, and delete_by_secret_id methods for KFrag management | Restrictions: Must extend Repository<KFrag, KFragId>, use async_trait, KFrag implements Zeroize so handle with care, find_by_secret_id returns DomainResult<Vec<KFrag>>, find_by_holder_index takes SecretId and holder_index | Success: Trait compiles, extends base Repository, includes all three additional methods with correct signatures_

- [x] 7. Create CFragRepository trait
  - File: client/src/repositories/cfrag_interface.rs
  - Define `CFragRepository` trait extending `Repository<CFrag, CFragId>`
  - Add `find_by_secret_id` method returning Vec<CFrag>
  - Add `find_by_kfrag_id` method for KFrag relationship lookup
  - Add `delete_by_secret_id` method for bulk deletion
  - Add `count_by_secret_id` method for counting related CFrags
  - Purpose: Provide persistence abstraction for CFrag entity (re-encryption fragment with Zeroize)
  - _Leverage: client/src/repositories/mod.rs, client/src/domain/entities/cfrag.rs, client/src/domain/value_objects/ids.rs_
  - _Requirements: 5_
  - _Prompt: Role: Rust Developer with security-sensitive data handling experience | Task: Create CFragRepository trait in client/src/repositories/cfrag_interface.rs extending base Repository trait, adding find_by_secret_id (returns Vec), find_by_kfrag_id, delete_by_secret_id, and count_by_secret_id methods for CFrag management | Restrictions: Must extend Repository<CFrag, CFragId>, use async_trait, CFrag implements Zeroize so handle with care, count_by_secret_id returns DomainResult<usize> | Success: Trait compiles, extends base Repository, includes all four additional methods with correct signatures_

- [x] 8. Update lib.rs to export repositories module
  - File: client/src/lib.rs
  - Add `pub mod repositories;` declaration
  - Ensure repositories module is publicly accessible
  - Purpose: Make Repository interfaces available for external use
  - _Leverage: client/src/lib.rs existing module structure_
  - _Requirements: 0, 1, 2, 3, 4, 5_
  - _Prompt: Role: Rust Developer | Task: Update client/src/lib.rs to export the new repositories module by adding pub mod repositories declaration | Restrictions: Only add the module declaration, maintain existing module structure | Success: lib.rs compiles, repositories module is accessible from outside the crate_

- [x] 9. Verify compilation and run tests
  - File: client/ (entire crate)
  - Run `cargo check` to verify all trait definitions compile
  - Run `cargo test` to ensure no regressions
  - Verify Send + Sync bounds are enforced at compile time
  - Purpose: Ensure all Repository interfaces are correctly defined and compilable
  - _Leverage: Makefile commands (make check, make test)_
  - _Requirements: All_
  - _Prompt: Role: QA Engineer | Task: Verify the entire client crate compiles successfully with cargo check and all existing tests pass with cargo test | Restrictions: Do not modify any code, only verify compilation and test results | Success: cargo check passes with no errors, cargo test shows all tests passing, no compilation warnings related to new repository code_
