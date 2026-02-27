use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use formix::service::core::crypto::{PublicKey, SecretKey};

const DEFAULT_DIR: &str = ".formix-demo";

#[derive(Serialize, Deserialize)]
struct KeyFile {
    role: String,
    secret_key_hex: String,
    public_key_hex: String,
}

#[derive(Serialize, Deserialize)]
pub struct ShareResult {
    pub secret_id: String,
    pub owner_public_key_hex: String,
    pub capsule_tx_id: String,
    pub kfrag_count: u8,
    pub share_tx_ids: Vec<String>,
}

fn ensure_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir).context("Failed to create .formix-demo directory")?;
    }
    Ok(())
}

fn default_dir() -> PathBuf {
    PathBuf::from(DEFAULT_DIR)
}

pub fn default_key_path(role: &str) -> PathBuf {
    default_dir().join(format!("{role}.json"))
}

pub fn default_share_result_path(secret_id: &str) -> PathBuf {
    default_dir().join(format!("{secret_id}.json"))
}

pub fn save_keypair(role: &str, sk: &SecretKey, pk: &PublicKey, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }

    let key_file = KeyFile {
        role: role.to_string(),
        secret_key_hex: hex::encode(sk.as_bytes()),
        public_key_hex: hex::encode(&pk.key_data),
    };

    let json = serde_json::to_string_pretty(&key_file)?;
    write_restricted(path, json.as_bytes()).context("Failed to write key file")?;
    Ok(())
}

fn write_restricted(path: &Path, data: &[u8]) -> Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(path)?;
    file.write_all(data)?;
    Ok(())
}

pub fn load_keypair(path: &Path) -> Result<(SecretKey, PublicKey)> {
    let json =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let key_file: KeyFile = serde_json::from_str(&json)?;

    let sk_bytes = hex::decode(&key_file.secret_key_hex).context("Invalid secret key hex")?;
    let pk_bytes = hex::decode(&key_file.public_key_hex).context("Invalid public key hex")?;

    let sk = SecretKey::from_bytes(sk_bytes).map_err(|e| anyhow::anyhow!(e))?;
    let pk = PublicKey::from_bytes(pk_bytes).map_err(|e| anyhow::anyhow!(e))?;
    Ok((sk, pk))
}

pub fn load_public_key(path: &Path) -> Result<PublicKey> {
    let json =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let key_file: KeyFile = serde_json::from_str(&json)?;

    let pk_bytes = hex::decode(&key_file.public_key_hex).context("Invalid public key hex")?;
    PublicKey::from_bytes(pk_bytes).map_err(|e| anyhow::anyhow!(e))
}

pub fn save_share_result(result: &ShareResult, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }

    let json = serde_json::to_string_pretty(result)?;
    fs::write(path, json).context("Failed to write share result file")?;
    Ok(())
}

pub fn load_share_result(path: &Path) -> Result<ShareResult> {
    let json =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let result: ShareResult = serde_json::from_str(&json)?;
    Ok(result)
}
