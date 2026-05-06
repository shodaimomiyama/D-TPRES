# HyperBEAM Client Adapter Design

**Issue:** [#88 — Phase B: HyperBEAM Client Adapter](https://github.com/shodaimomiyama/FORMIX/issues/88)

**Goal:** Implement an HTTP client adapter for HyperBEAM in `client/src/adapter/external/ao/`, porting the sandbox `hb-client` crate's RFC-9421 signer, TABM encoder, and wallet loader into FORMIX's async adapter layer.

## Context

FORMIX currently has two AO client implementations:

- `ProductionAOClient` (in `ao/production_client.rs`) — CWAO-compatible, ANS-104 DataItem signing, MU/CU endpoints
- `MockAOClient` (in `mock_ao/`) — In-memory mock for testing

The CWAO-compatible client is redundant because `ao_cwao/` already maintains that code path. This phase replaces `ProductionAOClient` with `HyperBEAMClient` that speaks HyperBEAM's native HTTP protocol: RFC-9421 signed requests with TABM-encoded multipart bodies.

The sandbox `hb-client` crate (`hyperbeam-sandbox/crates/hb-client/src/`) validates HyperBEAM's HTTP API surface and serves as the porting source.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| HTTP mode | async reqwest | Matches existing `AOClient` trait (async). Consistent with FORMIX's async-first client architecture. |
| Wallet/JWK | Reuse existing `ArweaveJWK`, move to `wallet.rs` | Avoids duplicate JWK loading code. Add `to_rsa_private_key()` for signer. |
| Legacy CWAO client | Delete `production_client.rs` and `data_item.rs` | CWAO compatibility maintained in `ao_cwao/`. No rename needed. |
| Feature flag | New `hyperbeam` flag (replaces `production-ao`) | Clean separation. `production-ao` becomes obsolete. |
| API mapping | execute→schedule+compute, query/dry_run→compute | Maps to HyperBEAM endpoints. spawn is out of AOClient scope. |
| Test scope | Unit tests for signer/TABM/wallet. E2E in Phase D. | Phase B focuses on module correctness. Integration testing in #90. |
| Signing key type | `BlindedSigningKey<Sha512>` | Side-channel protection via blinding, consistent with project's constant-time security requirements (CLAUDE.md rule 7.2). |
| Content-digest algorithm | SHA-256 (not SHA-512) | HyperBEAM wire format uses `sha-256` for content-digest. The signing algorithm (rsa-pss-sha512) is separate. |
| TABM body type | `Vec<u8>` | Multipart bodies may contain binary data. `String` would break on non-UTF-8 content. |
| Variant string | `"ao.N.1"` | HyperBEAM mainnet variant, not `"ao.TN.1"` (legacy testnet). |

## File Structure

```
client/src/adapter/external/ao/
├── mod.rs                    # Updated exports
├── client.rs                 # AOClient trait (unchanged)
├── config.rs                 # AOConfig (HyperBEAM defaults)
├── message.rs                # AOExecuteMsg/AOQueryMsg (unchanged)
├── wallet.rs                 # NEW: ArweaveJWK moved from data_item.rs
├── signer.rs                 # NEW: RFC-9421 rsa-pss-sha512 signer
├── tabm.rs                   # NEW: TABM multipart encoder
└── hyperbeam_client.rs       # NEW: HyperBEAMClient (AOClient impl)
```

**Deleted files:**
- `production_client.rs` — CWAO client, redundant with `ao_cwao/`
- `data_item.rs` — ANS-104 DataItem builder/signer, not used by HyperBEAM

## Module Specifications

### `wallet.rs` — Arweave JWK Loader

Moved from `data_item.rs`. Provides RSA key material for signing.

```rust
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ArweaveJWK {
    // Existing fields (n, e, d, p, q, dp, dq, qi)
    // All secret fields zeroized on drop
}

impl ArweaveJWK {
    pub fn from_file(path: &str) -> Result<Self, WalletError>;
    pub fn to_rsa_private_key(&self) -> Result<RsaPrivateKey, WalletError>;
    pub fn address(&self) -> String;
    pub fn sig_name(&self) -> String;
}
```

- `to_rsa_private_key()` converts JWK fields to `rsa::RsaPrivateKey` for use by signer
- `address()` computes `base64url(SHA-256(base64url_decode(n)))` where `n` is the raw big-endian bytes of the RSA modulus decoded from the JWK `n` field. Returns a 43-character base64url string.
- `sig_name()` generates the label used in RFC-9421 `signature-input` header: `"http-sig-{hex(native_id(address)[1:9])}"`

### `signer.rs` — RFC-9421 HTTP Message Signatures

Ports `hb-client/src/signer.rs`. Algorithm: `rsa-pss-sha512` (only algorithm HyperBEAM accepts).

Uses `BlindedSigningKey<Sha512>` from the `rsa` crate for timing-attack resistance, consistent with the project's constant-time security requirements.

```rust
pub struct SignedMessage {
    pub content_digest_header: String,   // "sha-256=:base64:="
    pub signature_header: String,        // "<sig_name>=:base64:="
    pub signature_input_header: String,  // "<sig_name>=(components);alg=\"rsa-pss-sha512\";created=<ts>;keyid=\"<keyid>\""
}

/// Sign a body-bearing request (covers content-digest only).
/// Used for binary data uploads (e.g., cache_write).
pub fn sign_request(
    key: &RsaPrivateKey,
    body: &[u8],
    sig_name: &str,
) -> Result<SignedMessage, SignerError>;

/// Sign a committed message (covers all headers + content-digest).
/// Components sorted alphabetically to match HyperBEAM's expectation.
/// Used for spawn, schedule operations.
pub fn sign_message(
    key: &RsaPrivateKey,
    headers: &BTreeMap<String, String>,
    body: &[u8],
    sig_name: &str,
) -> Result<SignedMessage, SignerError>;
```

**Signing process:**
1. Compute `content-digest` = `sha-256=:base64(SHA-256(body)):=` (structured field format)
2. Build `signature-input`: sorted component identifiers with parameters `alg="rsa-pss-sha512"`, `created=<unix_timestamp>`, `keyid="<wallet_address>"`
3. Build signature base: for each covered component, `"<name>": <value>\n`, then `"@signature-params": <signature-input-value>`
4. Sign the signature base bytes with `BlindedSigningKey<Sha512>` (RSA-PSS-SHA512)
5. Encode signature as structured field byte sequence: `<sig_name>=:base64(signature_bytes):=`

**Note:** `sign_request` covers only `"content-digest"` as the component. `sign_message` covers all message headers plus `"content-digest"`, with components sorted alphabetically. The `content-type` header is set separately by the caller in `hyperbeam_client.rs`.

**Deviation from sandbox:** The sandbox uses `&[(&str, &str)]` for headers; this spec uses `BTreeMap<String, String>` for guaranteed sorted iteration, eliminating the need for manual sorting.

### `tabm.rs` — TABM Multipart Encoder

Ports `hb-client/src/tabm.rs`. Builds `multipart/form-data` bodies matching HyperBEAM's wire format.

```rust
pub struct TabmEncoded {
    pub content_type: String,  // "multipart/form-data; boundary=<B>"
    pub body: Vec<u8>,         // Binary-safe multipart body
}

/// Encode nested sub-message (map type, for device-stack etc.)
pub fn encode_nested_body(
    parts: &BTreeMap<String, String>,
) -> TabmEncoded;

/// Encode simple binary parts
pub fn encode_multipart(
    parts: &BTreeMap<String, Vec<u8>>,
) -> TabmEncoded;

/// Encode mixed parts (nested maps + binaries)
pub fn encode_mixed_body(
    nested: &BTreeMap<String, BTreeMap<String, String>>,
    binary: &BTreeMap<String, Vec<u8>>,
) -> TabmEncoded;
```

**Encoding rules:**
- Parts sorted lexicographically by name
- Boundary = `base64url(SHA-256(parts joined with "\r\n"))`
- Wire format: `--B\r\n<Part1>\r\n--B\r\n<Part2>...\r\n--B--`
- Nested sub-messages use recursive multipart encoding

### `hyperbeam_client.rs` — HyperBEAMClient

The main adapter implementing `AOClient` trait for HyperBEAM communication.

```rust
pub struct HyperBEAMClient {
    http: reqwest::Client,
    config: AOConfig,
    wallet: ArweaveJWK,
}

impl HyperBEAMClient {
    pub fn new(config: AOConfig, wallet: ArweaveJWK) -> Result<Self, AOCommunicationError>;
}

#[async_trait]
impl AOClient for HyperBEAMClient {
    async fn execute(&self, process_id: &str, msg: AOExecuteMsg)
        -> Result<AONativeResponse, AOCommunicationError>;
    async fn query(&self, process_id: &str, msg: AOQueryMsg)
        -> Result<Binary, AOCommunicationError>;
    async fn dry_run(&self, process_id: &str, msg: AOExecuteMsg)
        -> Result<AONativeResponse, AOCommunicationError>;
}
```

**API mapping to HyperBEAM endpoints:**

| AOClient method | HyperBEAM operation | HTTP |
|-----------------|---------------------|------|
| `execute(pid, msg)` | schedule + compute | `POST /<pid>/schedule` → `GET /<pid>/compute/<slot>` |
| `query(pid, msg)` | compute (read) | `GET /<pid>/now` or subpath compute |
| `dry_run(pid, msg)` | compute (simulate) | Same as compute flow |

**`execute` flow detail:**
1. Convert `AOExecuteMsg` fields to header map (fields < 4KB → HTTP headers)
2. Set `inline-body-key: data` header when the HTTP body carries the `data` field
3. Encode body via TABM if multipart content exists
4. Sign with RFC-9421 signer (`sign_message` for committed messages)
5. Attach `signature`, `signature-input`, `content-digest` headers
6. `POST /<process_id>/schedule` via async reqwest
7. Extract `slot` from the schedule response (from the `slot` response header or body field — the exact mechanism is determined by HyperBEAM's response format for the schedule endpoint)
8. `GET /<process_id>/compute/<slot>` to retrieve computation result
9. Parse response into `AONativeResponse`

**Response parsing:**
HyperBEAM returns `multipart/form-data` responses with `ao-types` headers that describe the type of each part. Response parsing strategy:

1. Read `content-type` header to extract boundary
2. Split response body on boundary markers
3. For each part, read the part's `content-disposition` (field name) and `ao-type` header
4. Map known fields to `AONativeResponse`:
   - Result data fields → `data` (serialized as `serde_json::Value`)
   - Error fields → `error` (String)
   - Success/failure → `ok` (bool, derived from presence of error fields or HTTP status)
   - Message ID → `message_id` (from schedule response, if available)
5. Hand-parse multipart boundaries (no external multipart parsing library needed — the format is simple and deterministic)

## AOConfig Changes

Update defaults from CWAO testnet to local HyperBEAM:

```rust
impl Default for AOConfig {
    fn default() -> Self {
        Self {
            mu_url: "http://localhost:10000".to_string(),
            cu_url: "http://localhost:10000".to_string(),
            gateway_url: "http://localhost:10000".to_string(),
            timeout_ms: 30_000,
        }
    }
}
```

HyperBEAM runs as a single node (MU/CU/Gateway are the same endpoint). Field names are preserved for compatibility with existing code that references them (MockAOClient, tests). Environment variable overrides (`AO_MU_URL`, `AO_CU_URL`, `AO_GATEWAY_URL`, `AO_TIMEOUT_MS`) remain functional.

## Error Handling

Extend existing `AOCommunicationError` with HyperBEAM-specific variants:

```rust
pub enum AOCommunicationError {
    // Existing variants preserved...

    #[error("RFC-9421 signing failed: {0}")]
    SigningError(String),

    #[error("TABM encoding failed: {0}")]
    TabmEncodingError(String),

    #[error("Wallet error: {0}")]
    WalletError(String),

    #[error("Multipart response parsing failed: {0}")]
    ResponseParsingError(String),
}
```

Signing key bytes and secret material never appear in error messages.

## Feature Flag and DI Wiring

### Cargo.toml

```toml
[features]
hyperbeam = ["dep:reqwest", "dep:rsa", "dep:sha2"]
# production-ao is removed
```

The `hyperbeam` feature gates the same dependencies that `production-ao` previously gated. On non-wasm32 targets, `rsa`, `sha2`, and `reqwest` are currently unconditional dependencies; the feature flag controls whether `HyperBEAMClient` is compiled. `HyperBEAMClient` is not intended to compile for wasm32 (it requires network I/O via reqwest).

### DI Container (`actions/di.rs`)

```rust
#[cfg(feature = "hyperbeam")]
pub type HyperBEAMActionsContainer = ActionsContainer<
    CoreCryptoServiceImpl,
    ServiceStorageServiceImpl<
        ArweaveStorageServiceImpl,
        ContractStorageImpl<HyperBEAMClient>,
    >,
>;
```

### Builder (`actions/builder.rs`)

Add `HyperBEAMClient`-based assembly under `#[cfg(feature = "hyperbeam")]`. Remove existing `#[cfg(feature = "production-ao")]` code blocks.

## Security

- `ArweaveJWK`: All secret fields derive `Zeroize` and `ZeroizeOnDrop`
- `RsaPrivateKey` from `rsa` crate handles its own zeroization
- `BlindedSigningKey<Sha512>` provides timing-attack resistance for RSA-PSS signing
- Error messages never expose key material, signature bytes, or JWK fields
- `to_rsa_private_key()` conversion happens in-memory; intermediate bytes are zeroized

## Testing Strategy

**Phase B unit tests:**

1. **signer tests** — Verify RFC-9421 signature format: correct `signature-input` structure with `alg`, `created`, `keyid` parameters; valid `content-digest` computation (SHA-256); proper base64 encoding of signature bytes; alphabetical component sorting
2. **TABM encoder tests** — Verify lexicographic part sorting, deterministic boundary computation, correct multipart wire format with nested sub-messages, binary safety of `Vec<u8>` body
3. **wallet tests** — JWK file parsing, `RsaPrivateKey` conversion round-trip, address derivation from raw modulus bytes, `sig_name()` format

**Existing test maintenance:**
- `MockAOClient` tests are unaffected (AOClient trait unchanged)
- Tests gated behind `production-ao` are migrated to `hyperbeam` or removed if CWAO-specific

**Out of scope (Phase D #90):**
- E2E integration tests against local HyperBEAM node
- spawn + schedule + compute HTTP round-trip verification

## Dependencies

All required crates are already in `client/Cargo.toml`:
- `rsa = "0.9"` (with `sha2` feature) — RSA-PSS-SHA512 signing via `BlindedSigningKey`
- `sha2 = "0.10"` — SHA-256 for content-digest, SHA-512 for signing
- `base64 = "0.22"` — Structured field encoding
- `reqwest = "0.12"` — Async HTTP client
- `rand = "0.8"` — Required by `BlindedSigningKey` for randomized blinding (already in Cargo.toml)

No new dependencies needed.

## Reference

- `hyperbeam-sandbox/crates/hb-client/src/` — Porting source code
- `hyperbeam-sandbox/docs/hyperbeam-api-notes.md` — M1-M5 API findings
- HyperBEAM uses `wasm-64@1.0` device (hyphenated, not `wasm64`)
- JSON-Iface provides the JSON-to-WASM memory bridge: `handle(msg_ptr: i32, env_ptr: i32)`
- HyperBEAM variant string: `"ao.N.1"` (not `"ao.TN.1"`)
