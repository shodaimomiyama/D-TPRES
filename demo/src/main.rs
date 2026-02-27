mod key_store;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use formix::actions::ProductionFormixClient;

#[derive(Parser)]
#[command(name = "formix-demo")]
#[command(about = "FORMIX Threshold Proxy Re-Encryption Demo CLI")]
struct Cli {
    /// Run full demo locally (backward compat for `--local`)
    #[arg(long)]
    local: bool,

    /// Path to deploy.json config file
    #[arg(long, default_value = "deploy.json")]
    deploy: String,

    /// Path to Arweave JWK wallet file (overrides ARWEAVE_WALLET_PATH env)
    #[arg(long)]
    wallet: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, ValueEnum)]
enum Role {
    Owner,
    Requester,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Owner => write!(f, "owner"),
            Role::Requester => write!(f, "requester"),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a key pair for a specific role and persist to file
    Keygen {
        /// Role to generate keys for
        #[arg(long)]
        role: Role,

        /// Output file path (default: .formix-demo/{role}.json)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Split a secret using 2-of-3 threshold PRE and store on Arweave
    Share {
        /// Path to owner key file
        #[arg(long, default_value = ".formix-demo/owner.json")]
        owner_key_file: PathBuf,

        /// Path to requester public key file
        #[arg(long, default_value = ".formix-demo/requester.json")]
        requester_pubkey_file: PathBuf,
    },
    /// Recover the secret from Arweave using threshold cFrags
    Recover {
        /// Secret ID returned by the share command
        #[arg(long)]
        secret_id: String,

        /// Path to requester key file
        #[arg(long, default_value = ".formix-demo/requester.json")]
        requester_key_file: PathBuf,

        /// Path to share result file (default: .formix-demo/{secret_id}.json)
        #[arg(long)]
        share_result_file: Option<PathBuf>,
    },
    /// Local mode: step-by-step execution with contract re-encryption
    Local {
        #[command(subcommand)]
        command: LocalCommands,
    },
}

#[derive(Subcommand)]
enum LocalCommands {
    /// Phase 1: split secret and save intermediate data
    Share {
        /// Path to owner key file
        #[arg(long, default_value = ".formix-demo/owner.json")]
        owner_key_file: PathBuf,

        /// Path to requester public key file
        #[arg(long, default_value = ".formix-demo/requester.json")]
        requester_pubkey_file: PathBuf,
    },
    /// Phase 2: re-encrypt using actual contract code
    Reencrypt {
        /// Secret ID from local share output
        #[arg(long)]
        secret_id: String,

        /// Path to local share result file
        #[arg(long)]
        share_result_file: Option<PathBuf>,
    },
    /// Phase 3: recover the secret from local re-encryption output
    Recover {
        /// Secret ID
        #[arg(long)]
        secret_id: String,

        /// Path to requester key file
        #[arg(long, default_value = ".formix-demo/requester.json")]
        requester_key_file: PathBuf,

        /// Path to local share result file
        #[arg(long)]
        share_result_file: Option<PathBuf>,

        /// Path to local reencrypt result file
        #[arg(long)]
        reencrypt_result_file: Option<PathBuf>,
    },
    /// Run all local steps: keygen -> share -> reencrypt -> recover
    All {
        /// Path to owner key file
        #[arg(long, default_value = ".formix-demo/owner.json")]
        owner_key_file: PathBuf,

        /// Path to requester key file
        #[arg(long, default_value = ".formix-demo/requester.json")]
        requester_key_file: PathBuf,
    },
}

fn resolve_wallet_path(cli_wallet: &Option<String>) -> Result<String> {
    if let Some(ref path) = cli_wallet {
        return Ok(path.clone());
    }
    std::env::var("ARWEAVE_WALLET_PATH")
        .context("Wallet path not provided. Set ARWEAVE_WALLET_PATH or use --wallet")
}

fn create_client(deploy_path: &str, wallet_path: &str) -> Result<ProductionFormixClient> {
    println!("{}", "Initializing FORMIX client...".dimmed());
    println!("  deploy config: {}", deploy_path.cyan());
    println!("  wallet:        {}", wallet_path.cyan());

    ProductionFormixClient::from_deploy_file(deploy_path, wallet_path)
        .context("Failed to initialize ProductionFormixClient")
}

fn print_phase(phase: &str, description: &str) {
    println!();
    println!(
        "{} {}",
        format!("[{phase}]").bold().green(),
        description.bold()
    );
    println!("{}", "─".repeat(60).dimmed());
}

fn print_result(label: &str, value: &str) {
    println!("  {}: {}", label.yellow(), value);
}

fn hex_short(data: &[u8]) -> String {
    let hex_str: String = data.iter().map(|b| format!("{b:02x}")).collect();
    if hex_str.len() > 16 {
        format!("{}...{}", &hex_str[..8], &hex_str[hex_str.len() - 8..])
    } else {
        hex_str
    }
}

// ─────────────────── Production commands ───────────────────

fn run_keygen(
    client: &ProductionFormixClient,
    role: &Role,
    output: &Option<PathBuf>,
) -> Result<()> {
    let role_str = role.to_string();
    print_phase("KEYGEN", &format!("Generating {role_str} key pair"));

    let (sk, pk) = client
        .generate_keypair()
        .context("Failed to generate key pair")?;

    let path = output
        .clone()
        .unwrap_or_else(|| key_store::default_key_path(&role_str));

    key_store::save_keypair(&role_str, &sk, &pk, &path)?;

    println!("  {} Key pair generated and saved", "OK".green());
    print_result("role", &role_str);
    print_result("public key (hex)", &hex_short(&pk.key_data));
    print_result("saved to", &path.display().to_string());

    Ok(())
}

async fn run_share(
    client: &ProductionFormixClient,
    owner_key_file: &Path,
    requester_pubkey_file: &Path,
) -> Result<()> {
    print_phase("Phase 1", "Secret Sharing (2-of-3 threshold)");

    let secret_data = b"Hello FORMIX - Threshold PRE Demo".to_vec();
    println!(
        "  secret: {:?} ({} bytes)",
        String::from_utf8_lossy(&secret_data),
        secret_data.len()
    );
    println!("  threshold: k=2, n=3");

    let (owner_sk, owner_pk) = key_store::load_keypair(owner_key_file)
        .with_context(|| format!("Failed to load owner key from {}", owner_key_file.display()))?;
    println!(
        "  {} Owner key loaded from {}",
        "OK".green(),
        owner_key_file.display()
    );

    let requester_pk = key_store::load_public_key(requester_pubkey_file).with_context(|| {
        format!(
            "Failed to load requester PK from {}",
            requester_pubkey_file.display()
        )
    })?;
    println!(
        "  {} Requester public key loaded from {}",
        "OK".green(),
        requester_pubkey_file.display()
    );

    println!();
    println!("  {} Executing share workflow...", ">>".blue());

    let result = client
        .share()
        .secret(secret_data)
        .threshold(2)
        .total_shares(3)
        .owner_key(owner_sk)
        .requester_key(requester_pk)
        .execute()
        .await
        .context("Share workflow failed")?;

    let share_result = key_store::ShareResult {
        secret_id: result.secret_id.as_str().to_string(),
        owner_public_key_hex: hex::encode(&owner_pk.key_data),
        capsule_tx_id: result.capsule_tx_id.clone(),
        kfrag_count: result.kfrag_count,
        share_tx_ids: result.share_tx_ids.clone(),
    };
    let result_path = key_store::default_share_result_path(result.secret_id.as_str());
    key_store::save_share_result(&share_result, &result_path)?;

    print_phase("Result", "Phase 1 completed successfully");
    print_result("secret_id", result.secret_id.as_str());
    print_result("capsule_tx_id", &result.capsule_tx_id);
    print_result("kfrag_count", &result.kfrag_count.to_string());
    print_result("share_tx_ids", &format!("{:?}", result.share_tx_ids));
    print_result("result saved to", &result_path.display().to_string());

    println!();
    println!(
        "{}",
        format!(
            "To recover, run: formix-demo recover --secret-id {}",
            result.secret_id.as_str()
        )
        .cyan()
    );

    Ok(())
}

async fn run_recover(
    client: &ProductionFormixClient,
    secret_id: &str,
    requester_key_file: &Path,
    share_result_file: &Option<PathBuf>,
) -> Result<()> {
    print_phase("Phase 3", "Secret Recovery");

    println!("  secret_id: {}", secret_id.cyan());

    let (requester_sk, _requester_pk) =
        key_store::load_keypair(requester_key_file).with_context(|| {
            format!(
                "Failed to load requester key from {}",
                requester_key_file.display()
            )
        })?;
    println!(
        "  {} Requester key loaded from {}",
        "OK".green(),
        requester_key_file.display()
    );

    let result_path = share_result_file
        .clone()
        .unwrap_or_else(|| key_store::default_share_result_path(secret_id));
    let share_result = key_store::load_share_result(&result_path)
        .with_context(|| format!("Failed to load share result from {}", result_path.display()))?;
    println!(
        "  {} Share result loaded from {}",
        "OK".green(),
        result_path.display()
    );

    let owner_pk_bytes =
        hex::decode(&share_result.owner_public_key_hex).context("Invalid owner public key hex")?;
    let owner_pk = formix::service::core::crypto::PublicKey::from_bytes(owner_pk_bytes)
        .map_err(|e| anyhow::anyhow!(e))?;

    println!();
    println!("  {} Executing recover workflow...", ">>".blue());

    let secret_id_val = formix::domain::SecretId::new(secret_id);
    let result = client
        .recover()
        .secret_id(&secret_id_val)
        .requester_key(requester_sk)
        .owner_key(owner_pk)
        .execute()
        .await
        .context("Recover workflow failed")?;

    print_phase("Result", "Phase 3 completed successfully");
    print_result(
        "recovered secret",
        &String::from_utf8_lossy(&result.recovered_secret),
    );
    print_result("audit_tx_id", &result.audit_tx_id);

    Ok(())
}

// ─────────────────── Local commands ───────────────────

fn run_local_share(owner_key_file: &Path, requester_pubkey_file: &Path) -> Result<String> {
    use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

    let crypto = CryptoServiceImpl::new();
    let threshold: u8 = 2;
    let total: u8 = 3;

    let (owner_sk, owner_pk) = key_store::load_keypair(owner_key_file)
        .with_context(|| format!("Failed to load owner key from {}", owner_key_file.display()))?;
    println!(
        "  {} Owner key loaded from {}",
        "OK".green(),
        owner_key_file.display()
    );

    let requester_pk = key_store::load_public_key(requester_pubkey_file).with_context(|| {
        format!(
            "Failed to load requester PK from {}",
            requester_pubkey_file.display()
        )
    })?;
    println!(
        "  {} Requester public key loaded from {}",
        "OK".green(),
        requester_pubkey_file.display()
    );

    let secret_data = b"Hello FORMIX - Threshold PRE Demo".to_vec();
    print_phase("Phase 1", "Secret Sharing (2-of-3 threshold)");
    println!(
        "  secret: {:?} ({} bytes)",
        String::from_utf8_lossy(&secret_data),
        secret_data.len()
    );
    println!("  threshold: k={threshold}, n={total}");

    let symmetric_key = crypto
        .generate_symmetric_key()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "  {} Symmetric key generated ({} bytes)",
        "OK".green(),
        symmetric_key.len()
    );

    let shares = crypto
        .split_secret_shamir(&secret_data, threshold, total)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("  {} Shamir split: {} shares", "OK".green(), shares.len());

    let encrypted_shares: Vec<(u8, Vec<u8>)> = shares
        .iter()
        .map(|s| {
            let ct = crypto
                .aes_gcm_encrypt(&symmetric_key, &s.share_data)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok((s.index, ct))
        })
        .collect::<Result<Vec<_>>>()?;
    println!(
        "  {} AES-encrypted {} shares",
        "OK".green(),
        encrypted_shares.len()
    );

    let (capsule, capsule_ciphertext) = crypto
        .create_pre_capsule(&owner_pk, &symmetric_key)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "  {} PRE capsule created ({} bytes)",
        "OK".green(),
        capsule.capsule_bytes.len()
    );

    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &requester_pk)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let kfrags = crypto
        .create_kfrags(&reenc_key, threshold, total)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("  {} Generated {} kFrags", "OK".green(), kfrags.len());

    let verifying_pk = crypto
        .verifying_key_bytes()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let secret_id = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>();

    let local_kfrags: Vec<key_store::LocalKFragInfo> = kfrags
        .iter()
        .map(|kf| {
            let stored = contract::StoredKeyFrag {
                id: kf.id,
                key_data: kf.key_data.clone(),
                verification_data: kf.verification_data.clone(),
                precursor: kf.precursor.clone(),
            };
            let serialized = bincode::serialize(&stored).expect("kFrag serialization");
            key_store::LocalKFragInfo {
                id: kf.id,
                serialized_hex: hex::encode(&serialized),
            }
        })
        .collect();

    let result = key_store::LocalShareResult {
        secret_id: secret_id.clone(),
        owner_public_key_hex: hex::encode(&owner_pk.key_data),
        capsule_bytes_hex: hex::encode(&capsule.capsule_bytes),
        capsule_ciphertext_hex: hex::encode(&capsule_ciphertext),
        encrypted_shares: encrypted_shares
            .iter()
            .map(|(idx, ct)| key_store::LocalEncryptedShare {
                index: *idx,
                ciphertext_hex: hex::encode(ct),
            })
            .collect(),
        kfrags: local_kfrags,
        verifying_pk_hex: hex::encode(&verifying_pk),
        threshold_k: threshold,
        total_n: total,
    };

    let result_path = key_store::default_local_share_path(&secret_id);
    key_store::save_local_share_result(&result, &result_path)?;

    print_phase("Result", "Phase 1 completed successfully");
    print_result("secret_id", &secret_id);
    print_result("kfrag_count", &kfrags.len().to_string());
    print_result("result saved to", &result_path.display().to_string());

    println!();
    println!(
        "{}",
        format!("Next: formix-demo local reencrypt --secret-id {secret_id}").cyan()
    );

    Ok(secret_id)
}

fn run_local_reencrypt(
    secret_id: &str,
    share_result_file: &Option<PathBuf>,
) -> Result<()> {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Binary;

    let share_path = share_result_file
        .clone()
        .unwrap_or_else(|| key_store::default_local_share_path(secret_id));
    let share_result = key_store::load_local_share_result(&share_path)
        .with_context(|| format!("Failed to load local share from {}", share_path.display()))?;
    println!(
        "  {} Local share result loaded from {}",
        "OK".green(),
        share_path.display()
    );

    let capsule_bytes =
        hex::decode(&share_result.capsule_bytes_hex).context("Invalid capsule hex")?;
    let capsule_binary = Binary::from(capsule_bytes);

    print_phase(
        "Phase 2",
        "Proxy Re-Encryption (contract code, local execution)",
    );
    println!(
        "  Using {} of {} kFrags (threshold)",
        share_result.threshold_k, share_result.total_n
    );

    let mut cfrags: Vec<key_store::LocalCFragInfo> = Vec::new();

    for (i, kfrag_info) in share_result
        .kfrags
        .iter()
        .take(share_result.threshold_k as usize)
        .enumerate()
    {
        let holder_id = format!("holder-{i}");
        let process_id = format!("holder_process_{i}");
        let kfrag_id = format!("kfrag-{}", kfrag_info.id);
        let capsule_id = format!("capsule-{secret_id}");

        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);

        contract::contract::instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            contract::InstantiateMsg {
                process_id: process_id.clone(),
            },
        )
        .map_err(|e| anyhow::anyhow!("Holder {i} instantiate failed: {e}"))?;

        let kfrag_bytes =
            hex::decode(&kfrag_info.serialized_hex).context("Invalid kFrag hex")?;
        let kfrag_binary = Binary::from(kfrag_bytes);

        contract::contract::execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            contract::ExecuteMsg::SubmitKFrag {
                kfrag_id: kfrag_id.clone(),
                kfrag: kfrag_binary,
            },
        )
        .map_err(|e| anyhow::anyhow!("Holder {i} SubmitKFrag failed: {e}"))?;

        contract::contract::execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            contract::ExecuteMsg::SubmitCapsule {
                kfrag_id: kfrag_id.clone(),
                capsule_id: capsule_id.clone(),
                capsule: capsule_binary.clone(),
            },
        )
        .map_err(|e| anyhow::anyhow!("Holder {i} SubmitCapsule failed: {e}"))?;

        let query_result = contract::contract::query(
            deps.as_ref(),
            env,
            contract::QueryMsg::GetCFrag {
                kfrag_id,
                capsule_id,
            },
        )
        .map_err(|e| anyhow::anyhow!("Holder {i} GetCFrag query failed: {e}"))?;

        let cfrag_response: contract::GetCFragResponse =
            cosmwasm_std::from_json(query_result)
                .map_err(|e| anyhow::anyhow!("Holder {i} cFrag response parse failed: {e}"))?;

        let stored_cfrag: contract::StoredCFrag =
            bincode::deserialize(cfrag_response.cfrag.as_slice())
                .context("Failed to deserialize StoredCFrag from contract")?;

        println!(
            "  {} Holder {} re-encrypted kFrag -> cFrag ({} bytes) [contract]",
            "OK".green(),
            i,
            stored_cfrag.capsule_fragment.len()
        );

        cfrags.push(key_store::LocalCFragInfo {
            fragment_id: stored_cfrag.fragment_id,
            capsule_fragment_hex: hex::encode(&stored_cfrag.capsule_fragment),
            holder_id,
        });
    }

    println!(
        "  {} Collected {}/{} cFrags (threshold met)",
        "OK".green(),
        cfrags.len(),
        share_result.threshold_k
    );

    let reencrypt_result = key_store::LocalReencryptResult {
        secret_id: secret_id.to_string(),
        cfrags,
    };
    let result_path = key_store::default_local_reencrypt_path(secret_id);
    key_store::save_local_reencrypt_result(&reencrypt_result, &result_path)?;

    print_phase("Result", "Phase 2 completed successfully");
    print_result("result saved to", &result_path.display().to_string());

    println!();
    println!(
        "{}",
        format!("Next: formix-demo local recover --secret-id {secret_id}").cyan()
    );

    Ok(())
}

fn run_local_recover(
    secret_id: &str,
    requester_key_file: &Path,
    share_result_file: &Option<PathBuf>,
    reencrypt_result_file: &Option<PathBuf>,
) -> Result<()> {
    use formix::usecase::core::crypto::{
        CFragData, Capsule, CryptoService, CryptoServiceImpl, ShamirShare,
    };

    let crypto = CryptoServiceImpl::new();

    print_phase("Phase 3", "Secret Recovery");
    println!("  secret_id: {}", secret_id.cyan());

    let share_path = share_result_file
        .clone()
        .unwrap_or_else(|| key_store::default_local_share_path(secret_id));
    let share_result = key_store::load_local_share_result(&share_path)
        .with_context(|| format!("Failed to load local share from {}", share_path.display()))?;
    println!(
        "  {} Local share result loaded",
        "OK".green()
    );

    let reencrypt_path = reencrypt_result_file
        .clone()
        .unwrap_or_else(|| key_store::default_local_reencrypt_path(secret_id));
    let reencrypt_result = key_store::load_local_reencrypt_result(&reencrypt_path).with_context(
        || {
            format!(
                "Failed to load local reencrypt from {}",
                reencrypt_path.display()
            )
        },
    )?;
    println!(
        "  {} Local reencrypt result loaded",
        "OK".green()
    );

    let (requester_sk, _) = key_store::load_keypair(requester_key_file).with_context(|| {
        format!(
            "Failed to load requester key from {}",
            requester_key_file.display()
        )
    })?;
    println!(
        "  {} Requester key loaded",
        "OK".green()
    );

    let owner_pk_bytes =
        hex::decode(&share_result.owner_public_key_hex).context("Invalid owner PK hex")?;
    let owner_pk = formix::service::core::crypto::PublicKey::from_bytes(owner_pk_bytes)
        .map_err(|e| anyhow::anyhow!(e))?;

    let capsule_bytes =
        hex::decode(&share_result.capsule_bytes_hex).context("Invalid capsule hex")?;
    let capsule = Capsule::from_bytes(capsule_bytes);
    let capsule_ciphertext =
        hex::decode(&share_result.capsule_ciphertext_hex).context("Invalid ciphertext hex")?;
    let verifying_pk =
        hex::decode(&share_result.verifying_pk_hex).context("Invalid verifying PK hex")?;

    let cfrag_data_list: Vec<CFragData> = reencrypt_result
        .cfrags
        .iter()
        .map(|cf| {
            let cfrag_bytes =
                hex::decode(&cf.capsule_fragment_hex).expect("Invalid cFrag hex");
            CFragData::new(cfrag_bytes, cf.holder_id.clone())
        })
        .collect();

    println!();
    println!("  {} Executing recover workflow...", ">>".blue());

    let recovered_key = crypto
        .decrypt_pre_capsule(
            &capsule,
            &cfrag_data_list,
            &requester_sk,
            &owner_pk,
            &capsule_ciphertext,
            &verifying_pk,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("  {} Symmetric key recovered via PRE", "OK".green());

    let threshold = share_result.threshold_k as usize;

    let mut decrypted_shares = Vec::new();
    for enc_share in share_result.encrypted_shares.iter().take(threshold) {
        let ciphertext =
            hex::decode(&enc_share.ciphertext_hex).context("Invalid encrypted share hex")?;
        let decrypted = crypto
            .aes_gcm_decrypt(&recovered_key, &ciphertext)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        decrypted_shares.push(ShamirShare::new(enc_share.index, decrypted));
    }
    println!(
        "  {} Decrypted {} shares with recovered key",
        "OK".green(),
        decrypted_shares.len()
    );

    let recovered_secret = crypto
        .reconstruct_secret_shamir(&decrypted_shares, share_result.threshold_k)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("  {} Secret reconstructed via Shamir", "OK".green());

    print_phase("Result", "Phase 3 completed successfully");
    print_result(
        "recovered secret",
        &String::from_utf8_lossy(&recovered_secret),
    );

    let original_matches = recovered_secret == b"Hello FORMIX - Threshold PRE Demo";
    if original_matches {
        println!(
            "\n  {}",
            "SUCCESS: Secret matches original!".green().bold()
        );
    }

    Ok(())
}

fn run_local_all(owner_key_file: &Path, requester_key_file: &Path) -> Result<()> {
    use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

    let crypto = CryptoServiceImpl::new();

    // Keygen if key files don't exist
    if !owner_key_file.exists() {
        print_phase("KEYGEN", "Generating owner key pair");
        let (sk, pk) = crypto
            .generate_keypair()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        key_store::save_keypair("owner", &sk, &pk, owner_key_file)?;
        println!("  {} Key pair generated and saved", "OK".green());
        print_result("saved to", &owner_key_file.display().to_string());
    }

    if !requester_key_file.exists() {
        print_phase("KEYGEN", "Generating requester key pair");
        let (sk, pk) = crypto
            .generate_keypair()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        key_store::save_keypair("requester", &sk, &pk, requester_key_file)?;
        println!("  {} Key pair generated and saved", "OK".green());
        print_result("saved to", &requester_key_file.display().to_string());
    }

    let secret_id = run_local_share(owner_key_file, requester_key_file)?;

    run_local_reencrypt(&secret_id, &None)?;

    run_local_recover(&secret_id, requester_key_file, &None, &None)?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    println!();
    println!(
        "{}",
        "FORMIX - Threshold Proxy Re-Encryption Demo"
            .bold()
            .magenta()
    );
    println!("{}", "=".repeat(60).dimmed());

    if cli.local {
        let owner_key_file = PathBuf::from(".formix-demo/owner.json");
        let requester_key_file = PathBuf::from(".formix-demo/requester.json");
        run_local_all(&owner_key_file, &requester_key_file)?;
    } else {
        let command = cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Subcommand required (or use --local)"))?;

        match command {
            Commands::Local { command: local_cmd } => match local_cmd {
                LocalCommands::Share {
                    owner_key_file,
                    requester_pubkey_file,
                } => {
                    run_local_share(&owner_key_file, &requester_pubkey_file)?;
                }
                LocalCommands::Reencrypt {
                    secret_id,
                    share_result_file,
                } => {
                    run_local_reencrypt(&secret_id, &share_result_file)?;
                }
                LocalCommands::Recover {
                    secret_id,
                    requester_key_file,
                    share_result_file,
                    reencrypt_result_file,
                } => {
                    run_local_recover(
                        &secret_id,
                        &requester_key_file,
                        &share_result_file,
                        &reencrypt_result_file,
                    )?;
                }
                LocalCommands::All {
                    owner_key_file,
                    requester_key_file,
                } => {
                    run_local_all(&owner_key_file, &requester_key_file)?;
                }
            },
            _ => {
                let wallet_path = resolve_wallet_path(&cli.wallet)?;
                let client = create_client(&cli.deploy, &wallet_path)?;

                println!(
                    "  {} Client initialized (process: {})",
                    "OK".green(),
                    client.process_id().cyan()
                );

                match command {
                    Commands::Keygen { role, output } => run_keygen(&client, &role, &output)?,
                    Commands::Share {
                        owner_key_file,
                        requester_pubkey_file,
                    } => run_share(&client, &owner_key_file, &requester_pubkey_file).await?,
                    Commands::Recover {
                        secret_id,
                        requester_key_file,
                        share_result_file,
                    } => {
                        run_recover(
                            &client,
                            &secret_id,
                            &requester_key_file,
                            &share_result_file,
                        )
                        .await?
                    }
                    Commands::Local { .. } => unreachable!(),
                }
            }
        }
    }

    println!();
    println!("{}", "Done.".green().bold());
    Ok(())
}
