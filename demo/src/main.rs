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
    /// Path to deploy.json config file
    #[arg(long, default_value = "deploy.json")]
    deploy: String,

    /// Path to Arweave JWK wallet file (overrides ARWEAVE_WALLET_PATH env)
    #[arg(long)]
    wallet: Option<String>,

    #[command(subcommand)]
    command: Commands,
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

fn hex_short(data: &[u8]) -> String {
    let hex_str: String = data.iter().map(|b| format!("{b:02x}")).collect();
    if hex_str.len() > 16 {
        format!("{}...{}", &hex_str[..8], &hex_str[hex_str.len() - 8..])
    } else {
        hex_str
    }
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

    let wallet_path = resolve_wallet_path(&cli.wallet)?;
    let client = create_client(&cli.deploy, &wallet_path)?;

    println!(
        "  {} Client initialized (process: {})",
        "OK".green(),
        client.process_id().cyan()
    );

    match cli.command {
        Commands::Keygen { role, output } => run_keygen(&client, &role, &output)?,
        Commands::Share {
            owner_key_file,
            requester_pubkey_file,
        } => run_share(&client, &owner_key_file, &requester_pubkey_file).await?,
        Commands::Recover {
            secret_id,
            requester_key_file,
            share_result_file,
        } => run_recover(&client, &secret_id, &requester_key_file, &share_result_file).await?,
    }

    println!();
    println!("{}", "Done.".green().bold());
    Ok(())
}
