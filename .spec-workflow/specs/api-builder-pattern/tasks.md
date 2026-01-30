# Tasks Document

- [x] 1. Create Type-State marker types and ShareBuilder struct
  - Files: `client/src/actions/builder.rs`
  - Define `Set`/`NotSet` marker types, `ShareBuilder<S, T, N, O, R>` struct with PhantomData
  - Implement fluent setter methods (`secret`, `threshold`, `total_shares`, `owner_key`, `requester_key`, `metadata`)
  - Implement `execute()` on `ShareBuilder<Set, Set, Set, Set, Set>` delegating to existing `ActionsContainer::share()`
  - owner_public_key is derived internally from owner_secret_key via umbral_pre
  - Purpose: Type-safe compile-time enforcement of required fields for share operation
  - _Leverage: `client/src/actions/mod.rs` (existing share method), `client/src/usecase/core/crypto.rs` (CryptoService)_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 5.1, 5.2_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in type-state patterns and generics | Task: Create ShareBuilder with type-state pattern in client/src/actions/builder.rs. Define Set/NotSet marker types and ShareBuilder<Secret, Threshold, TotalShares, OwnerKey, RequesterKey> struct. Each setter method transitions the corresponding type parameter from NotSet to Set. execute() is only available when all type params are Set. Delegate to existing ActionsContainer::share() internally. Derive owner_public_key from owner_secret_key inside execute(). Reference client/src/actions/mod.rs for the existing share signature and client/src/usecase/core/crypto.rs for key derivation | Restrictions: Do not modify existing share implementation. Do not use runtime checks for required fields. Maintain Zeroize for SecretKey. Do not add async. Keep builder zero-cost (PhantomData only) | Success: ShareBuilder compiles with type-state enforcement. execute() only callable with all Set params. Existing tests still pass. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [x] 2. Create RecoverBuilder struct
  - Files: `client/src/actions/builder.rs`
  - Define `RecoverBuilder<S, R>` struct with PhantomData
  - Implement fluent setter methods (`secret_id`, `requester_key`)
  - Implement `execute()` on `RecoverBuilder<Set, Set>` delegating to existing `ActionsContainer::recover()`
  - Purpose: Type-safe compile-time enforcement of required fields for recover operation
  - _Leverage: `client/src/actions/mod.rs` (existing recover method), `client/src/actions/builder.rs` (Set/NotSet from task 1)_
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in type-state patterns | Task: Create RecoverBuilder<SecretIdState, RequesterKeyState> in client/src/actions/builder.rs, reusing Set/NotSet from task 1. Implement secret_id() and requester_key() setters. execute() only available when both are Set. Delegate to existing ActionsContainer::recover(). Reference client/src/actions/mod.rs for the existing recover signature | Restrictions: Do not modify existing recover implementation. Do not use runtime checks. Maintain Zeroize for SecretKey. Do not add async | Success: RecoverBuilder compiles with type-state enforcement. execute() only callable with all Set params. Existing tests still pass. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [x] 3. Create InitConfig and DTpresClient struct
  - Files: `client/src/actions/client.rs`
  - Define `InitConfig` struct with wallet_path, optional gateway URLs
  - Define `DTpresClient` struct wrapping existing `DefaultActionsContainer`
  - Implement `DTpresClient::init(config)` — load wallet, detect/spawn AO Process, return client
  - Implement accessor methods: `process_id()`, `wallet_address()`, `ao_gateway_url()`, `arweave_gateway_url()`
  - Implement `share()` returning `ShareBuilder<NotSet, NotSet, NotSet, NotSet, NotSet>`
  - Implement `recover()` returning `RecoverBuilder<NotSet, NotSet>`
  - Implement `generate_keypair()` delegating to CryptoService
  - Purpose: Main entry point for the D-TPRES client library
  - _Leverage: `client/src/actions/mod.rs` (DefaultActionsContainer), `client/src/actions/di.rs` (DI setup), `client/src/adapter/external/ao_client.rs` (AOClient)_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in API design and client libraries | Task: Create DTpresClient in client/src/actions/client.rs. InitConfig has wallet_path (String), ao_gateway_url (Option<String>), arweave_gateway_url (Option<String>). DTpresClient wraps DefaultActionsContainer and stores process_id, wallet_address, gateway URLs. init() loads JWK wallet, auto-detects existing process or spawns new one, returns DTpresClient. share() returns ShareBuilder, recover() returns RecoverBuilder. Reference client/src/actions/di.rs for container creation and client/src/adapter/external/ao_client.rs for AO interactions | Restrictions: Do not break existing DefaultActionsContainer usage. No async/await. JWK wallet loading must be secure. Do not expose internal types | Success: DTpresClient::init() works with valid wallet. share() and recover() return correct builder types. Accessor methods work. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [ ] 4. Create result and error types
  - Files: `client/src/actions/result.rs`, `client/src/actions/error.rs` (modify existing)
  - Define `ShareResult` struct (secret_id, capsule_info, encrypted_shares, holder_process_ids)
  - Define `ClientError`, `ShareError`, `RecoverError` enums using thiserror
  - Map existing ActionError variants to new error types
  - Purpose: Clean public API types for the new builder pattern
  - _Leverage: `client/src/actions/error.rs` (existing errors), `client/src/domain/value_objects/ids.rs` (SecretId)_
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in error handling and API design | Task: Create ShareResult in client/src/actions/result.rs and add ClientError, ShareError, RecoverError to client/src/actions/error.rs using thiserror. ShareResult contains secret_id (SecretId), capsule_info (CapsuleInfo), encrypted_shares (Vec<EncryptedShare>), holder_process_ids (Vec<String>). Error types: ClientError (WalletLoadFailed, ProcessSpawnFailed, ProcessConnectionFailed), ShareError (InvalidThreshold, ExecutionFailed), RecoverError (SecretNotFound, ExecutionFailed). Map from existing ActionError variants | Restrictions: Do not remove existing error types (backward compat). Do not expose internal error details in public error messages. Never include secret data in error messages | Success: All new types compile. Existing error handling still works. thiserror provides Display impl. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [x] 5. Wire up module exports and public API
  - Files: `client/src/actions/mod.rs` (modify), `client/src/lib.rs` (modify or create)
  - Add `mod builder; mod client; mod result;` to actions/mod.rs
  - Re-export public API at crate root: DTpresClient, InitConfig, ShareBuilder, RecoverBuilder, Set, NotSet, ShareResult, error types
  - Keep DefaultActionsContainer and internal types as pub(crate)
  - Purpose: Clean public API surface for the dtpres-client crate
  - _Leverage: `client/src/actions/mod.rs`, `client/src/lib.rs`_
  - _Requirements: 7.1, 7.2, 7.4_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer specializing in crate API design | Task: Wire up module exports in client/src/actions/mod.rs and client/src/lib.rs. Add mod declarations for builder, client, result modules. Re-export at crate root: DTpresClient, InitConfig, ShareBuilder, RecoverBuilder, Set, NotSet, ShareResult, ClientError, ShareError, RecoverError. Keep DefaultActionsContainer, AOClient, ArweaveClient as pub(crate) | Restrictions: Do not break existing imports. Do not expose internal implementation types. Maintain backward compatibility | Success: `use dtpres_client::DTpresClient` works. Internal types are not accessible from outside. Existing code still compiles. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [x] 6. Add #[deprecated] attributes to legacy API functions
  - Files: `client/src/actions/mod.rs` (modify)
  - Add `#[deprecated(since = "0.2.0", note = "use DTpresClient::share() builder instead")]` to existing share function
  - Add `#[deprecated(since = "0.2.0", note = "use DTpresClient::recover() builder instead")]` to existing recover function
  - Ensure deprecated functions internally delegate to builder implementation
  - Purpose: Smooth migration path for existing users
  - _Leverage: `client/src/actions/mod.rs`_
  - _Requirements: 6.1, 6.2, 6.3_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Developer | Task: Add #[deprecated] attributes to existing share() and recover() methods in client/src/actions/mod.rs. Use since="0.2.0" and note pointing to new DTpresClient builder API. Optionally refactor internals to delegate to builder execute() | Restrictions: Do not change function signatures. Do not remove functions. Existing tests must pass (with #[allow(deprecated)] if needed) | Success: Calling old share/recover produces deprecation warning. Functions still work correctly. All existing tests pass. `make check` and `make lint` pass | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_

- [ ] 7. Create unit tests for builders and DTpresClient
  - Files: `client/tests/builder_test.rs` (new), `client/tests/client_test.rs` (new)
  - Test ShareBuilder type-state transitions and execute
  - Test RecoverBuilder type-state transitions and execute
  - Test DTpresClient::init, share(), recover(), generate_keypair()
  - Test error scenarios (invalid threshold, missing wallet, etc.)
  - Compile-fail tests for missing required fields (if trybuild available)
  - Purpose: Verify builder pattern correctness and client initialization
  - _Leverage: `client/tests/actions_integration_test.rs` (existing test patterns), `client/src/adapter/external/ao_client.rs` (MockAOClient)_
  - _Requirements: 2.1, 2.2, 2.3, 4.1, 4.2, 4.3, 1.1, 1.6_
  - _Prompt: Implement the task for spec api-builder-pattern, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Rust Test Engineer | Task: Create unit and integration tests for ShareBuilder, RecoverBuilder, and DTpresClient. Test builder fluent API with all fields set (execute succeeds), test error cases (invalid threshold), test DTpresClient::init with MockAOClient. Reference client/tests/actions_integration_test.rs for test patterns and MockAOClient usage | Restrictions: Use MockAOClient, not real AO Network. Do not modify existing tests. Test both success and error paths | Success: All new tests pass. Full share→recover roundtrip works via builders. Error cases are covered. `make test` passes | After completing: mark task [-] to [x] in tasks.md, log implementation with log-implementation tool_
