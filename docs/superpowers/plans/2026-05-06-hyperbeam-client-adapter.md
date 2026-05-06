# HyperBEAM Client Adapter Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the CWAO-compatible `ProductionAOClient` with `HyperBEAMClient` that speaks HyperBEAM's native HTTP protocol (RFC-9421 signed requests, TABM-encoded multipart bodies).

**Architecture:** Modular port of sandbox `hb-client` into `client/src/adapter/external/ao/`. Three independent modules (`wallet.rs`, `signer.rs`, `tabm.rs`) compose into `hyperbeam_client.rs` which implements the existing `AOClient` trait. Old CWAO-specific files (`production_client.rs`, `data_item.rs`) are deleted; `production-ao` feature flag replaced with `hyperbeam`.

**Tech Stack:** `rsa 0.9` (BlindedSigningKey, RSA-PSS-SHA512), `sha2 0.10` (SHA-256/512), `base64 0.22`, `reqwest 0.12` (async), `zeroize 1.8`

**Spec:** `docs/superpowers/specs/2026-05-06-hyperbeam-client-adapter-design.md`

**Sandbox source (porting from):** `hyperbeam-sandbox/crates/hb-client/src/`

---

## File Structure

```
client/src/adapter/external/ao/
├── mod.rs                    # MODIFY: update exports
├── client.rs                 # UNCHANGED
├── config.rs                 # MODIFY: update default URLs
├── message.rs                # UNCHANGED
├── wallet.rs                 # CREATE: ArweaveJWK moved from data_item.rs + new methods
├── signer.rs                 # CREATE: RFC-9421 signer
├── tabm.rs                   # CREATE: TABM multipart encoder
├── hyperbeam_client.rs       # CREATE: HyperBEAMClient
├── production_client.rs      # DELETE
└── data_item.rs              # DELETE

client/src/adapter/errors.rs  # MODIFY: add 4 new error variants
client/src/actions/di.rs      # MODIFY: replace production-ao with hyperbeam
client/Cargo.toml             # MODIFY: feature flag rename

client/tests/unit/adapter/external/ao/
├── mod.rs                    # MODIFY: update test module refs
├── wallet.rs                 # CREATE: wallet unit tests
├── signer.rs                 # CREATE: signer unit tests
├── tabm.rs                   # CREATE: TABM encoder unit tests
├── data_item.rs              # DELETE
└── production_client.rs      # DELETE
```

---

## Chunk 1: Foundation — Wallet, Errors, Config

### Task 1: Add error variants to AOCommunicationError

**Files:**
- Modify: `client/src/adapter/errors.rs:203-247` (AOCommunicationError enum)
- Modify: `client/src/adapter/errors.rs:249-304` (From<AOCommunicationError> for AdapterError)
- Modify: `client/src/adapter/errors.rs:308-387` (helper methods)

- [ ] **Step 1: Add 4 new error variants**

Add after `ExecutionError` (line 246) in `AOCommunicationError`:

```rust
    #[error("RFC-9421 signing failed: {details}")]
    SigningError { details: String },

    #[error("TABM encoding failed: {details}")]
    TabmEncodingError { details: String },

    #[error("Wallet error: {details}")]
    WalletError { details: String },

    #[error("Multipart response parsing failed: {details}")]
    ResponseParsingError { details: String },
```

- [ ] **Step 2: Add From conversions for new variants**

Add after `ExecutionError` match arm (line 302) in `From<AOCommunicationError> for AdapterError`:

```rust
            AOCommunicationError::SigningError { details } => Self::StorageError {
                operation: "rfc9421_signing".to_string(),
                details,
            },
            AOCommunicationError::TabmEncodingError { details } => Self::SerializationError {
                operation: "tabm_encode".to_string(),
                details,
            },
            AOCommunicationError::WalletError { details } => Self::StorageError {
                operation: "wallet".to_string(),
                details,
            },
            AOCommunicationError::ResponseParsingError { details } => {
                Self::SerializationError {
                    operation: "response_parse".to_string(),
                    details,
                }
            }
```

- [ ] **Step 3: Add helper methods**

Add after `execution_error` (line 386):

```rust
    pub fn signing_error(details: impl Into<String>) -> Self {
        Self::SigningError { details: details.into() }
    }

    pub fn tabm_encoding_error(details: impl Into<String>) -> Self {
        Self::TabmEncodingError { details: details.into() }
    }

    pub fn wallet_error(details: impl Into<String>) -> Self {
        Self::WalletError { details: details.into() }
    }

    pub fn response_parsing_error(details: impl Into<String>) -> Self {
        Self::ResponseParsingError { details: details.into() }
    }
```

- [ ] **Step 4: Run compilation check**

Run: `cd client && cargo check 2>&1 | head -20`
Expected: compiles successfully (no existing code uses the new variants yet)

- [ ] **Step 5: Commit**

```bash
git add client/src/adapter/errors.rs
git commit -m "feat(adapter): add HyperBEAM error variants to AOCommunicationError"
```

---

### Task 2: Create wallet.rs — ArweaveJWK with RSA key conversion

**Files:**
- Create: `client/src/adapter/external/ao/wallet.rs`
- Reference: `client/src/adapter/external/ao/data_item.rs:177-207` (existing ArweaveJWK)
- Reference: `hyperbeam-sandbox/crates/hb-client/src/wallet.rs` (porting source)

- [ ] **Step 1: Write wallet test file**

Create `client/tests/unit/adapter/external/ao/wallet.rs`:

```rust
use formix::adapter::external::ao::wallet::ArweaveJWK;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::RsaPrivateKey;

fn generate_test_jwk() -> ArweaveJWK {
    let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let public_key = private_key.to_public_key();
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
    let d = URL_SAFE_NO_PAD.encode(private_key.d().to_bytes_be());
    let primes = private_key.primes();
    let p = URL_SAFE_NO_PAD.encode(primes[0].to_bytes_be());
    let q = URL_SAFE_NO_PAD.encode(primes[1].to_bytes_be());

    ArweaveJWK {
        kty: "RSA".to_string(),
        n,
        e,
        d,
        p,
        q,
        dp: String::new(),
        dq: String::new(),
        qi: String::new(),
    }
}

#[test]
fn test_to_rsa_private_key_roundtrip() {
    let jwk = generate_test_jwk();
    let rsa_key = jwk.to_rsa_private_key().expect("should convert to RsaPrivateKey");
    assert!(rsa_key.validate().is_ok());
}

#[test]
fn test_address_is_43_chars_base64url() {
    let jwk = generate_test_jwk();
    let address = jwk.address();
    assert_eq!(address.len(), 43, "Arweave address should be 43 chars base64url");
    assert!(address.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
}

#[test]
fn test_sig_name_format() {
    let jwk = generate_test_jwk();
    let sig_name = jwk.sig_name();
    assert!(sig_name.starts_with("http-sig-"), "sig_name should start with 'http-sig-'");
    assert_eq!(sig_name.len(), "http-sig-".len() + 16, "sig_name hex part should be 16 chars (8 bytes)");
}

#[test]
fn test_address_deterministic() {
    let jwk = generate_test_jwk();
    let addr1 = jwk.address();
    let addr2 = jwk.address();
    assert_eq!(addr1, addr2);
}
```

- [ ] **Step 2: Register wallet test module**

In `client/tests/unit/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
mod wallet;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd client && cargo test --features hyperbeam test_to_rsa_private_key_roundtrip 2>&1 | tail -5`
Expected: compilation error — `wallet` module not found

- [ ] **Step 4: Create wallet.rs implementation**

Create `client/src/adapter/external/ao/wallet.rs`:

```rust
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rsa::RsaPrivateKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::adapter::errors::AOCommunicationError;

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ArweaveJWK {
    #[zeroize(skip)]
    pub kty: String,
    #[zeroize(skip)]
    pub n: String,
    #[zeroize(skip)]
    pub e: String,
    #[serde(default)]
    pub d: String,
    #[serde(default)]
    pub p: String,
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub dp: String,
    #[serde(default)]
    pub dq: String,
    #[serde(default)]
    pub qi: String,
}

impl std::fmt::Debug for ArweaveJWK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArweaveJWK")
            .field("kty", &self.kty)
            .field("n", &format!("[{} chars]", self.n.len()))
            .field("e", &self.e)
            .field("d", &"[REDACTED]")
            .field("p", &"[REDACTED]")
            .field("q", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ArweaveJWK {
    pub fn from_file(path: &str) -> Result<Self, AOCommunicationError> {
        let data = std::fs::read_to_string(path).map_err(|e| {
            AOCommunicationError::wallet_error(format!("reading wallet file '{path}': {e}"))
        })?;
        serde_json::from_str(&data).map_err(|e| {
            AOCommunicationError::wallet_error(format!("parsing JWK: {e}"))
        })
    }

    pub fn to_rsa_private_key(&self) -> Result<RsaPrivateKey, AOCommunicationError> {
        let n = decode_biguint(&self.n).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding n: {e}"))
        })?;
        let e = decode_biguint(&self.e).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding e: {e}"))
        })?;
        let d = decode_biguint(&self.d).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding d: {e}"))
        })?;
        let p = decode_biguint(&self.p).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding p: {e}"))
        })?;
        let q = decode_biguint(&self.q).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding q: {e}"))
        })?;

        let primes = vec![p, q];
        RsaPrivateKey::from_components(n, e, d, primes).map_err(|e| {
            AOCommunicationError::wallet_error(format!("constructing RSA private key: {e}"))
        })
    }

    pub fn address(&self) -> String {
        let n_bytes = URL_SAFE_NO_PAD
            .decode(&self.n)
            .expect("n field is valid base64url");
        let hash = Sha256::digest(&n_bytes);
        URL_SAFE_NO_PAD.encode(hash)
    }

    pub fn sig_name(&self) -> String {
        let address = self.address();
        let raw = URL_SAFE_NO_PAD
            .decode(&address)
            .expect("address is valid base64url");
        let slice = &raw[1..9];
        let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
        format!("http-sig-{hex}")
    }

}

fn decode_biguint(b64: &str) -> Result<rsa::BigUint, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|e| format!("base64url decode: {e}"))?;
    Ok(rsa::BigUint::from_bytes_be(&bytes))
}
```

- [ ] **Step 5: Register wallet module in mod.rs**

In `client/src/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
pub mod wallet;
```

And add the re-export:

```rust
#[cfg(feature = "hyperbeam")]
pub use wallet::ArweaveJWK;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd client && cargo test --features hyperbeam test_to_rsa_private_key_roundtrip test_address_is_43_chars test_sig_name_format test_address_deterministic -- --nocapture 2>&1 | tail -10`
Expected: all 4 tests PASS

- [ ] **Step 7: Commit**

```bash
git add client/src/adapter/external/ao/wallet.rs client/src/adapter/external/ao/mod.rs client/tests/unit/adapter/external/ao/wallet.rs client/tests/unit/adapter/external/ao/mod.rs
git commit -m "feat(adapter): add wallet.rs — ArweaveJWK with RSA key conversion and address derivation"
```

---

### Task 3: Update AOConfig defaults to HyperBEAM

**Files:**
- Modify: `client/src/adapter/external/ao/config.rs:14-16` (default constants)

- [ ] **Step 1: Update default URLs**

Change lines 14-16 from:

```rust
const DEFAULT_MU_URL: &str = "https://mu.ao-testnet.xyz";
const DEFAULT_CU_URL: &str = "https://cu.ao-testnet.xyz";
const DEFAULT_GATEWAY_URL: &str = "https://arweave.net";
```

To:

```rust
const DEFAULT_MU_URL: &str = "http://localhost:10000";
const DEFAULT_CU_URL: &str = "http://localhost:10000";
const DEFAULT_GATEWAY_URL: &str = "http://localhost:10000";
```

- [ ] **Step 2: Run existing config tests**

Run: `cd client && cargo test config 2>&1 | tail -10`
Expected: existing config tests pass (they test `new()` validation, not default values)

- [ ] **Step 3: Commit**

```bash
git add client/src/adapter/external/ao/config.rs
git commit -m "feat(config): update AOConfig defaults to local HyperBEAM endpoint"
```

---

## Chunk 2: Signer and TABM Encoder

### Task 4: Create signer.rs — RFC-9421 HTTP Message Signatures

**Files:**
- Create: `client/src/adapter/external/ao/signer.rs`
- Create: `client/tests/unit/adapter/external/ao/signer.rs`
- Reference: `hyperbeam-sandbox/crates/hb-client/src/signer.rs` (porting source)

- [ ] **Step 1: Write signer test file**

Create `client/tests/unit/adapter/external/ao/signer.rs`:

```rust
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use rand::rngs::OsRng;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use formix::adapter::external::ao::signer;

fn test_rsa_key() -> RsaPrivateKey {
    RsaPrivateKey::new(&mut OsRng, 2048).unwrap()
}

#[test]
fn test_sign_request_content_digest_is_sha256() {
    let key = test_rsa_key();
    let body = b"hello world";
    let signed = signer::sign_request(&key, body, "test-sig").unwrap();

    let expected_hash = Sha256::digest(body);
    let expected = format!("sha-256=:{}:", STANDARD.encode(expected_hash));
    assert_eq!(signed.content_digest_header, expected);
}

#[test]
fn test_sign_request_signature_header_format() {
    let key = test_rsa_key();
    let signed = signer::sign_request(&key, b"data", "my-sig").unwrap();

    assert!(signed.signature_header.starts_with("my-sig=:"));
    assert!(signed.signature_header.ends_with(':'));
}

#[test]
fn test_sign_request_signature_input_format() {
    let key = test_rsa_key();
    let signed = signer::sign_request(&key, b"data", "my-sig").unwrap();

    assert!(signed.signature_input_header.starts_with("my-sig="));
    assert!(signed.signature_input_header.contains("alg=\"rsa-pss-sha512\""));
    assert!(signed.signature_input_header.contains("created="));
    assert!(signed.signature_input_header.contains("keyid=\""));
    assert!(signed.signature_input_header.contains("\"content-digest\""));
}

#[test]
fn test_sign_message_covers_all_headers_sorted() {
    let key = test_rsa_key();
    let mut headers = BTreeMap::new();
    headers.insert("type".to_string(), "Message".to_string());
    headers.insert("action".to_string(), "Ping".to_string());

    let signed = signer::sign_message(&key, &headers, b"body", "test-sig").unwrap();

    let input = &signed.signature_input_header;
    let action_pos = input.find("\"action\"").expect("should contain action");
    let cd_pos = input.find("\"content-digest\"").expect("should contain content-digest");
    let type_pos = input.find("\"type\"").expect("should contain type");
    assert!(action_pos < cd_pos, "action before content-digest (alphabetical)");
    assert!(cd_pos < type_pos, "content-digest before type (alphabetical)");
}

#[test]
fn test_sign_message_empty_body_no_content_digest() {
    let key = test_rsa_key();
    let mut headers = BTreeMap::new();
    headers.insert("action".to_string(), "Ping".to_string());

    let signed = signer::sign_message(&key, &headers, b"", "test-sig").unwrap();

    assert!(signed.content_digest_header.is_empty());
    assert!(!signed.signature_input_header.contains("content-digest"));
}
```

- [ ] **Step 2: Register signer test module**

In `client/tests/unit/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
mod signer;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd client && cargo test --features hyperbeam test_sign_request_content_digest 2>&1 | tail -5`
Expected: compilation error — `signer` module not found

- [ ] **Step 4: Create signer.rs implementation**

Create `client/src/adapter/external/ao/signer.rs`:

```rust
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rsa::pss::BlindedSigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256, Sha512};
use std::collections::BTreeMap;

use crate::adapter::errors::AOCommunicationError;

pub struct SignedMessage {
    pub content_digest_header: String,
    pub signature_header: String,
    pub signature_input_header: String,
}

pub fn sign_request(
    key: &RsaPrivateKey,
    body: &[u8],
    sig_name: &str,
) -> Result<SignedMessage, AOCommunicationError> {
    let content_digest = {
        let hash = Sha256::digest(body);
        format!("sha-256=:{}:", STANDARD.encode(hash))
    };

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let keyid = STANDARD.encode(key.n().to_bytes_be());
    let covered_components = "\"content-digest\"";

    let signature_input = format!(
        "({covered_components});alg=\"rsa-pss-sha512\";created={created};keyid=\"{keyid}\""
    );

    let sig_base = format!(
        "\"content-digest\": {content_digest}\n\
         \"@signature-params\": {signature_input}"
    );

    // BlindedSigningKey for side-channel protection (CLAUDE.md rule 7.2)
    let signing_key = BlindedSigningKey::<Sha512>::new(key.clone());
    let mut rng = rand::thread_rng();
    let signature = signing_key.sign_with_rng(&mut rng, sig_base.as_bytes());
    let sig_b64 = STANDARD.encode(signature.to_vec());

    Ok(SignedMessage {
        content_digest_header: content_digest,
        signature_header: format!("{sig_name}=:{sig_b64}:"),
        signature_input_header: format!("{sig_name}={signature_input}"),
    })
}

pub fn sign_message(
    key: &RsaPrivateKey,
    headers: &BTreeMap<String, String>,
    body: &[u8],
    sig_name: &str,
) -> Result<SignedMessage, AOCommunicationError> {
    let has_body = !body.is_empty();

    let content_digest = if has_body {
        let hash = Sha256::digest(body);
        Some(format!("sha-256=:{}:", STANDARD.encode(hash)))
    } else {
        None
    };

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let keyid = STANDARD.encode(key.n().to_bytes_be());

    let mut components: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| (name.to_lowercase(), value.to_string()))
        .collect();
    if let Some(cd) = &content_digest {
        components.push(("content-digest".to_string(), cd.clone()));
    }
    components.sort_by(|a, b| a.0.cmp(&b.0));

    let covered_components = components
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(" ");

    let signature_input = format!(
        "({covered_components});alg=\"rsa-pss-sha512\";created={created};keyid=\"{keyid}\""
    );

    let mut sig_base_lines: Vec<String> = components
        .iter()
        .map(|(name, value)| format!("\"{name}\": {value}"))
        .collect();
    sig_base_lines.push(format!("\"@signature-params\": {signature_input}"));
    let sig_base = sig_base_lines.join("\n");

    let signing_key = BlindedSigningKey::<Sha512>::new(key.clone());
    let mut rng = rand::thread_rng();
    let signature = signing_key.sign_with_rng(&mut rng, sig_base.as_bytes());
    let sig_b64 = STANDARD.encode(signature.to_vec());

    Ok(SignedMessage {
        content_digest_header: content_digest.unwrap_or_default(),
        signature_header: format!("{sig_name}=:{sig_b64}:"),
        signature_input_header: format!("{sig_name}={signature_input}"),
    })
}
```

- [ ] **Step 5: Register signer module in mod.rs**

In `client/src/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
pub mod signer;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd client && cargo test --features hyperbeam signer 2>&1 | tail -15`
Expected: all 5 signer tests PASS

- [ ] **Step 7: Commit**

```bash
git add client/src/adapter/external/ao/signer.rs client/src/adapter/external/ao/mod.rs client/tests/unit/adapter/external/ao/signer.rs client/tests/unit/adapter/external/ao/mod.rs
git commit -m "feat(adapter): add signer.rs — RFC-9421 rsa-pss-sha512 HTTP message signatures"
```

---

### Task 5: Create tabm.rs — TABM Multipart Encoder

**Files:**
- Create: `client/src/adapter/external/ao/tabm.rs`
- Create: `client/tests/unit/adapter/external/ao/tabm.rs`
- Reference: `hyperbeam-sandbox/crates/hb-client/src/tabm.rs` (porting source)

- [ ] **Step 1: Write TABM test file**

Create `client/tests/unit/adapter/external/ao/tabm.rs`:

```rust
use formix::adapter::external::ao::tabm;

#[test]
fn test_encode_multipart_simple_parts() {
    let result = tabm::encode_hb_multipart(&[("action", "Ping"), ("type", "Message")]);

    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("action"));
    assert!(body_str.contains("Ping"));
    assert!(body_str.contains("type"));
    assert!(body_str.contains("Message"));
    assert!(result.content_type.starts_with("multipart/form-data; boundary=\""));
}

#[test]
fn test_encode_multipart_parts_sorted_lexicographically() {
    let result = tabm::encode_hb_multipart(&[("zebra", "z"), ("alpha", "a")]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();

    let alpha_pos = body_str.find("alpha").unwrap();
    let zebra_pos = body_str.find("zebra").unwrap();
    assert!(alpha_pos < zebra_pos, "parts should be sorted lexicographically");
}

#[test]
fn test_encode_multipart_deterministic_boundary() {
    let parts = &[("key", "value")];
    let r1 = tabm::encode_hb_multipart(parts);
    let r2 = tabm::encode_hb_multipart(parts);
    assert_eq!(r1.boundary, r2.boundary, "boundary should be deterministic");
    assert_eq!(r1.body, r2.body, "body should be deterministic");
}

#[test]
fn test_encode_nested_body() {
    let result = tabm::encode_hb_nested_body(
        "device-stack",
        &[("1", "WASI@1.0"), ("2", "JSON-Iface@1.0")],
    );
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("device-stack"));
    assert!(body_str.contains("WASI@1.0"));
}

#[test]
fn test_encode_mixed_body() {
    let result = tabm::encode_hb_mixed_body(&[
        tabm::BodyPart::Nested {
            name: "device-stack",
            items: &[("1", "WASI@1.0")],
        },
        tabm::BodyPart::Binary {
            name: "data",
            data: b"hello",
        },
    ]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("data"));
    assert!(body_str.contains("device-stack"));
}

#[test]
fn test_multipart_boundary_format() {
    let result = tabm::encode_hb_multipart(&[("key", "value")]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.starts_with(&format!("--{}", result.boundary)));
    assert!(body_str.ends_with(&format!("--{}--", result.boundary)));
}
```

- [ ] **Step 2: Register tabm test module**

In `client/tests/unit/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
mod tabm;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd client && cargo test --features hyperbeam test_encode_multipart_simple 2>&1 | tail -5`
Expected: compilation error — `tabm` module not found

- [ ] **Step 4: Create tabm.rs implementation**

Create `client/src/adapter/external/ao/tabm.rs`:

```rust
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

pub struct HbMultipart {
    pub body: Vec<u8>,
    pub boundary: String,
    pub content_type: String,
}

pub fn encode_hb_nested_body(part_name: &str, items: &[(&str, &str)]) -> HbMultipart {
    let disposition = format!("form-data;name=\"{part_name}\"");

    let mut kv: Vec<(&str, String)> = items.iter().map(|(k, v)| (*k, v.to_string())).collect();
    kv.push(("content-disposition", disposition));
    kv.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let part_bytes: Vec<u8> = kv
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("\r\n")
        .into_bytes();

    assemble_multipart(&[(part_name.to_string(), part_bytes)])
}

pub fn encode_hb_multipart(parts: &[(&str, &str)]) -> HbMultipart {
    let mut encoded: Vec<(String, Vec<u8>)> = parts
        .iter()
        .map(|(name, value)| {
            let key = name.to_lowercase();
            let part = format!("content-disposition: form-data;name=\"{key}\"\r\n\r\n{value}");
            (key, part.into_bytes())
        })
        .collect();

    encoded.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    assemble_multipart(&encoded)
}

pub enum BodyPart<'a> {
    Nested {
        name: &'a str,
        items: &'a [(&'a str, &'a str)],
    },
    Binary {
        name: &'a str,
        data: &'a [u8],
    },
}

pub fn encode_hb_mixed_body(parts: &[BodyPart]) -> HbMultipart {
    let mut encoded: Vec<(String, Vec<u8>)> = parts
        .iter()
        .map(|part| match part {
            BodyPart::Nested { name, items } => {
                let disposition = format!("form-data;name=\"{name}\"");
                let mut kv: Vec<(&str, String)> =
                    items.iter().map(|(k, v)| (*k, v.to_string())).collect();
                kv.push(("content-disposition", disposition));
                kv.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                let bytes: Vec<u8> = kv
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join("\r\n")
                    .into_bytes();
                (name.to_string(), bytes)
            }
            BodyPart::Binary { name, data } => {
                let header = format!("content-disposition: form-data;name=\"{name}\"\r\n\r\n");
                let mut bytes = header.into_bytes();
                bytes.extend_from_slice(data);
                (name.to_string(), bytes)
            }
        })
        .collect();

    encoded.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    assemble_multipart(&encoded)
}

fn assemble_multipart(sorted_parts: &[(String, Vec<u8>)]) -> HbMultipart {
    let part_bytes: Vec<&[u8]> = sorted_parts.iter().map(|(_, p)| p.as_slice()).collect();

    let mut hasher = Sha256::new();
    for (i, part) in part_bytes.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\r\n");
        }
        hasher.update(part);
    }
    let boundary = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut body = Vec::new();
    for (i, part) in part_bytes.iter().enumerate() {
        if i == 0 {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        } else {
            body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
        }
        body.extend_from_slice(part);
    }
    body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());

    let content_type = format!("multipart/form-data; boundary=\"{boundary}\"");

    HbMultipart {
        body,
        boundary,
        content_type,
    }
}
```

- [ ] **Step 5: Register tabm module in mod.rs**

In `client/src/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
pub mod tabm;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd client && cargo test --features hyperbeam tabm 2>&1 | tail -15`
Expected: all 6 TABM tests PASS

- [ ] **Step 7: Commit**

```bash
git add client/src/adapter/external/ao/tabm.rs client/src/adapter/external/ao/mod.rs client/tests/unit/adapter/external/ao/tabm.rs client/tests/unit/adapter/external/ao/mod.rs
git commit -m "feat(adapter): add tabm.rs — TABM multipart encoder for HyperBEAM wire format"
```

---

## Chunk 3: HyperBEAMClient and Cleanup

### Task 6: Create hyperbeam_client.rs — AOClient implementation

**Files:**
- Create: `client/src/adapter/external/ao/hyperbeam_client.rs`
- Reference: `hyperbeam-sandbox/crates/hb-client/src/hb.rs` (porting source)
- Reference: `client/src/adapter/external/ao/client.rs` (AOClient trait)

- [ ] **Step 1: Create hyperbeam_client.rs implementation**

Create `client/src/adapter/external/ao/hyperbeam_client.rs`:

```rust
use async_trait::async_trait;
use std::collections::BTreeMap;

use super::client::AOClient;
use super::config::AOConfig;
use super::message::{AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary};
use super::signer;
use super::wallet::ArweaveJWK;
// tabm is available for spawn/multipart operations (Phase D) but not used
// in schedule messages which send JSON body via inline-body-key
use crate::adapter::errors::AOCommunicationError;

pub struct HyperBEAMClient {
    http: reqwest::Client,
    config: AOConfig,
    wallet: ArweaveJWK,
}

impl HyperBEAMClient {
    pub fn new(config: AOConfig, wallet: ArweaveJWK) -> Result<Self, AOCommunicationError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms()))
            .build()
            .map_err(|e| AOCommunicationError::connection_error(format!("HTTP client: {e}")))?;

        Ok(Self {
            http,
            config,
            wallet,
        })
    }

    fn base_url(&self) -> &str {
        self.config.mu_url()
    }

    fn rsa_key(&self) -> Result<rsa::RsaPrivateKey, AOCommunicationError> {
        self.wallet.to_rsa_private_key()
    }

    async fn schedule(
        &self,
        process_id: &str,
        msg: &AOExecuteMsg,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/schedule", self.base_url(), process_id);
        let rsa_key = self.rsa_key()?;
        let sig_name = self.wallet.sig_name();

        let action = msg.action();
        let data = serde_json::to_string(msg.data()).map_err(|e| {
            AOCommunicationError::serialization_error(format!("serialize msg data: {e}"))
        })?;
        let body = data.as_bytes();

        let header_fields: BTreeMap<String, String> = BTreeMap::from([
            ("action".to_string(), action.to_string()),
            ("data-protocol".to_string(), "ao".to_string()),
            ("inline-body-key".to_string(), "data".to_string()),
            ("target".to_string(), process_id.to_string()),
            ("type".to_string(), "Message".to_string()),
            ("variant".to_string(), "ao.N.1".to_string()),
        ]);

        let signed = signer::sign_message(&rsa_key, &header_fields, body, &sig_name)
            .map_err(|e| AOCommunicationError::signing_error(format!("{e}")))?;

        let mut req = self.http.post(&url);
        for (name, value) in &header_fields {
            req = req.header(name.as_str(), value.as_str());
        }
        if !signed.content_digest_header.is_empty() {
            req = req.header("content-digest", &signed.content_digest_header);
        }
        let resp = req
            .header("signature", &signed.signature_header)
            .header("signature-input", &signed.signature_input_header)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| {
                AOCommunicationError::connection_error(format!("schedule POST: {e}"))
            })?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body_text = resp.text().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read schedule body: {e}"))
        })?;

        Ok((status, body_text, headers))
    }

    async fn compute(
        &self,
        process_id: &str,
        slot: u64,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/compute", self.base_url(), process_id);
        let resp = self
            .http
            .get(&url)
            .header("slot", slot.to_string())
            .send()
            .await
            .map_err(|e| {
                AOCommunicationError::connection_error(format!("compute GET: {e}"))
            })?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.text().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read compute body: {e}"))
        })?;

        Ok((status, body, headers))
    }

    async fn now(
        &self,
        process_id: &str,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/now", self.base_url(), process_id);
        let resp = self.http.get(&url).send().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("now GET: {e}"))
        })?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.text().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read now body: {e}"))
        })?;

        Ok((status, body, headers))
    }

    fn extract_slot(headers: &[(String, String)], body: &str) -> Option<u64> {
        for (key, val) in headers {
            if key.to_lowercase() == "slot" {
                return val.trim().parse().ok();
            }
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(s) = v.get("slot").and_then(|s| s.as_u64()) {
                return Some(s);
            }
        }
        None
    }

    fn parse_response(
        status: u16,
        body: &str,
        _headers: &[(String, String)],
    ) -> AONativeResponse {
        if status >= 400 {
            return AONativeResponse::error_response(
                format!("HTTP {status}: {}", &body[..body.len().min(200)]),
            );
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            let ok = v.get("ok").and_then(|o| o.as_bool()).unwrap_or(true);
            let error = v
                .get("error")
                .and_then(|e| e.as_str())
                .map(|s| s.to_string());
            if let Some(err) = error {
                return AONativeResponse::error_response(err);
            }
            if !ok {
                return AONativeResponse::error_response("Process returned ok=false");
            }
            AONativeResponse::success(v)
        } else {
            AONativeResponse::success(serde_json::Value::String(body.to_string()))
        }
    }
}

#[async_trait]
impl AOClient for HyperBEAMClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        let (sched_status, sched_body, sched_headers) =
            self.schedule(process_id, &msg).await?;

        if sched_status >= 400 {
            return Err(AOCommunicationError::execution_error(
                process_id,
                format!(
                    "schedule failed (HTTP {}): {}",
                    sched_status,
                    &sched_body[..sched_body.len().min(200)]
                ),
            ));
        }

        let slot = Self::extract_slot(&sched_headers, &sched_body).unwrap_or(1);

        let (comp_status, comp_body, comp_headers) =
            self.compute(process_id, slot).await?;

        Ok(Self::parse_response(comp_status, &comp_body, &comp_headers))
    }

    async fn query(
        &self,
        process_id: &str,
        _msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError> {
        let (_status, body, _headers) = self.now(process_id).await?;
        Ok(Binary(body.into_bytes()))
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        self.execute(process_id, msg).await
    }
}
```

- [ ] **Step 2: Register hyperbeam_client module in mod.rs**

In `client/src/adapter/external/ao/mod.rs`, add:

```rust
#[cfg(feature = "hyperbeam")]
mod hyperbeam_client;
```

And add the re-export:

```rust
#[cfg(feature = "hyperbeam")]
pub use hyperbeam_client::HyperBEAMClient;
```

- [ ] **Step 3: Run compilation check**

Run: `cd client && cargo check --features hyperbeam 2>&1 | head -20`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add client/src/adapter/external/ao/hyperbeam_client.rs client/src/adapter/external/ao/mod.rs
git commit -m "feat(adapter): add HyperBEAMClient — AOClient impl for HyperBEAM HTTP transport"
```

---

### Task 7: Update feature flag and DI wiring

**Files:**
- Modify: `client/Cargo.toml:41-43` (feature flags)
- Modify: `client/src/actions/di.rs:126-162` (production-ao → hyperbeam)
- Modify: `client/src/adapter/external/mod.rs` (re-exports)

- [ ] **Step 1: Replace feature flag in Cargo.toml**

Change line 42 from:

```toml
production-ao = ["dep:reqwest", "dep:rsa", "dep:sha2"]
```

To:

```toml
hyperbeam = ["dep:reqwest", "dep:rsa", "dep:sha2"]
```

- [ ] **Step 2: Update DI container in actions/di.rs**

Replace lines 126-162 (the entire `#[cfg(feature = "production-ao")]` block) with:

```rust
#[cfg(feature = "hyperbeam")]
use crate::adapter::external::ao::HyperBEAMClient;
#[cfg(feature = "hyperbeam")]
use crate::adapter::external::ao::wallet::ArweaveJWK;
#[cfg(feature = "hyperbeam")]
use crate::adapter::external::arweave::{ArweaveClientImpl, ProductionArweaveStorageService};

#[cfg(feature = "hyperbeam")]
pub type HyperBEAMStorageService = ServiceStorageServiceImpl<
    ProductionArweaveStorageService<ArweaveClientImpl>,
    ContractStorageImpl<HyperBEAMClient>,
>;

#[cfg(feature = "hyperbeam")]
pub type HyperBEAMActionsContainer =
    ActionsContainer<CoreCryptoServiceImpl, HyperBEAMStorageService>;

#[cfg(feature = "hyperbeam")]
impl HyperBEAMActionsContainer {
    pub fn with_hyperbeam(
        ao_client: std::sync::Arc<HyperBEAMClient>,
        arweave_client: std::sync::Arc<ArweaveClientImpl>,
    ) -> Self {
        let crypto_service = std::sync::Arc::new(CoreCryptoServiceImpl::new());
        let service_crypto =
            std::sync::Arc::new(ServiceCryptoServiceImpl::new(std::sync::Arc::clone(&crypto_service)));
        let arweave = std::sync::Arc::new(ProductionArweaveStorageService::new(arweave_client));
        let contract =
            std::sync::Arc::new(ContractStorageImpl::new_single_process(ao_client));
        let storage_service = std::sync::Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
        let controller = ControllerContainer::new(std::sync::Arc::clone(&crypto_service));
        let workflow_services = WorkflowServiceContainer::new(service_crypto, storage_service);
        Self {
            controller,
            workflow_services,
            crypto_service,
        }
    }
}
```

- [ ] **Step 3: Update adapter/external/mod.rs re-exports**

Replace the `production-ao` re-exports with `hyperbeam`:

```rust
#[cfg(feature = "hyperbeam")]
pub use ao::hyperbeam_client::HyperBEAMClient;
#[cfg(feature = "hyperbeam")]
pub use ao::wallet::ArweaveJWK;
```

Remove the old `production-ao` re-exports for `ProductionAOClient`, `DataItemBuilder`, `DataItemSigner`.

- [ ] **Step 4: Run compilation check**

Run: `cd client && cargo check --features hyperbeam 2>&1 | head -20`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add client/Cargo.toml client/src/actions/di.rs client/src/adapter/external/mod.rs
git commit -m "feat(di): replace production-ao feature flag with hyperbeam, update DI wiring"
```

---

### Task 8: Delete legacy CWAO files

**Files:**
- Delete: `client/src/adapter/external/ao/production_client.rs`
- Delete: `client/src/adapter/external/ao/data_item.rs`
- Delete: `client/tests/unit/adapter/external/ao/production_client.rs`
- Delete: `client/tests/unit/adapter/external/ao/data_item.rs`
- Modify: `client/src/adapter/external/ao/mod.rs` (remove old module declarations)
- Modify: `client/tests/unit/adapter/external/ao/mod.rs` (remove old test modules)

- [ ] **Step 1: Remove old module declarations from ao/mod.rs**

Remove these lines from `client/src/adapter/external/ao/mod.rs`:

```rust
#[cfg(feature = "production-ao")]
mod data_item;
#[cfg(feature = "production-ao")]
mod production_client;
```

And remove these re-exports:

```rust
#[cfg(feature = "production-ao")]
pub use data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
#[cfg(feature = "production-ao")]
pub use production_client::ProductionAOClient;
```

- [ ] **Step 2: Remove old test module declarations**

Remove these lines from `client/tests/unit/adapter/external/ao/mod.rs`:

```rust
#[cfg(feature = "production-ao")]
mod data_item;
#[cfg(feature = "production-ao")]
mod production_client;
```

- [ ] **Step 3: Delete the source files**

```bash
rm client/src/adapter/external/ao/production_client.rs
rm client/src/adapter/external/ao/data_item.rs
rm client/tests/unit/adapter/external/ao/production_client.rs
rm client/tests/unit/adapter/external/ao/data_item.rs
```

- [ ] **Step 4: Run full test suite**

Run: `cd client && cargo test 2>&1 | tail -15`
Expected: all existing tests pass (MockAOClient tests unaffected)

- [ ] **Step 5: Run with hyperbeam feature**

Run: `cd client && cargo test --features hyperbeam 2>&1 | tail -15`
Expected: all tests pass including new wallet/signer/tabm tests

- [ ] **Step 6: Run lint**

Run: `cd client && cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings 2>&1 | tail -10`
Expected: no warnings

- [ ] **Step 7: Commit**

```bash
git add -A client/src/adapter/external/ao/ client/tests/unit/adapter/external/ao/
git commit -m "chore(adapter): delete legacy CWAO files (production_client.rs, data_item.rs)"
```

---

### Task 9: Final verification

- [ ] **Step 1: Run make all**

Run: `make all 2>&1 | tail -20`
Expected: check, lint, and test all pass

- [ ] **Step 2: Run tests with hyperbeam feature**

Run: `cd client && cargo test --features hyperbeam 2>&1 | tail -20`
Expected: all tests pass

- [ ] **Step 3: Verify git status is clean**

Run: `git status`
Expected: clean working tree, all changes committed

---

## Summary

| Task | Description | Files | Tests |
|------|-------------|-------|-------|
| 1 | Error variants | errors.rs | - |
| 2 | wallet.rs | wallet.rs + test | 4 unit tests |
| 3 | AOConfig defaults | config.rs | existing tests |
| 4 | signer.rs | signer.rs + test | 5 unit tests |
| 5 | tabm.rs | tabm.rs + test | 6 unit tests |
| 6 | hyperbeam_client.rs | hyperbeam_client.rs | compile check |
| 7 | Feature flag + DI | Cargo.toml, di.rs, mod.rs | compile check |
| 8 | Delete legacy files | 4 files deleted | full suite |
| 9 | Final verification | - | make all |

**Total new tests:** 15 unit tests (wallet: 4, signer: 5, tabm: 6)
**Total commits:** 9
