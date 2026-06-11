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

// `duration_since(UNIX_EPOCH)` only fails when the system clock predates the
// epoch; surface that as a signing error instead of panicking.
fn unix_timestamp() -> Result<u64, AOCommunicationError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| AOCommunicationError::signing_error(format!("system clock before epoch: {e}")))
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

    let created = unix_timestamp()?;

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

    let created = unix_timestamp()?;

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
