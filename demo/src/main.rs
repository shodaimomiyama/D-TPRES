use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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

#[derive(Subcommand)]
enum Commands {
    /// Generate Owner and Requester key pairs
    Keygen,
    /// Split a secret using 2-of-3 threshold PRE and store on Arweave
    Share,
    /// Recover the secret from Arweave using threshold cFrags
    Recover {
        /// Secret ID returned by the share command
        #[arg(long)]
        secret_id: String,
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
    println!(
        "  deploy config: {}",
        deploy_path.cyan()
    );
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

fn run_keygen(client: &ProductionFormixClient) -> Result<()> {
    print_phase("KEYGEN", "Generating PRE key pairs");

    let (owner_sk, owner_pk) = client
        .generate_keypair()
        .context("Failed to generate Owner key pair")?;
    println!("  {} Owner key pair generated", "OK".green());
    print_result("  owner public key (hex)", &hex_short(&owner_pk.key_data));
    drop(owner_sk);

    let (_requester_sk, requester_pk) = client
        .generate_keypair()
        .context("Failed to generate Requester key pair")?;
    println!("  {} Requester key pair generated", "OK".green());
    print_result(
        "  requester public key (hex)",
        &hex_short(&requester_pk.key_data),
    );

    println!();
    println!(
        "{}",
        "Key pairs are ephemeral (in-memory only). Use `share` to run the full flow.".dimmed()
    );
    Ok(())
}

async fn run_share(client: &ProductionFormixClient) -> Result<()> {
    print_phase("Phase 1", "Secret Sharing (2-of-3 threshold)");

    let secret_data = b"Hello FORMIX - Threshold PRE Demo".to_vec();
    println!(
        "  secret: {:?} ({} bytes)",
        String::from_utf8_lossy(&secret_data),
        secret_data.len()
    );
    println!("  threshold: k=2, n=3");

    let (owner_sk, _owner_pk) = client
        .generate_keypair()
        .context("Failed to generate Owner key pair")?;
    let (_requester_sk, requester_pk) = client
        .generate_keypair()
        .context("Failed to generate Requester key pair")?;

    println!("  {} Key pairs generated", "OK".green());

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

    print_phase("Result", "Phase 1 completed successfully");
    print_result("secret_id", result.secret_id.as_str());
    print_result("capsule_tx_id", &result.capsule_tx_id);
    print_result("kfrag_count", &result.kfrag_count.to_string());
    print_result("share_tx_ids", &format!("{:?}", result.share_tx_ids));

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

async fn run_recover(client: &ProductionFormixClient, secret_id: &str) -> Result<()> {
    print_phase("Phase 3", "Secret Recovery");

    println!("  secret_id: {}", secret_id.cyan());

    let (requester_sk, _requester_pk) = client
        .generate_keypair()
        .context("Failed to generate Requester key pair")?;
    let (_owner_sk, owner_pk) = client
        .generate_keypair()
        .context("Failed to generate Owner key pair")?;

    println!("  {} Key pairs generated", "OK".green());
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
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    if hex.len() > 16 {
        format!("{}...{}", &hex[..8], &hex[hex.len() - 8..])
    } else {
        hex
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
        Commands::Keygen => run_keygen(&client)?,
        Commands::Share => run_share(&client).await?,
        Commands::Recover { secret_id } => run_recover(&client, &secret_id).await?,
    }

    println!();
    println!("{}", "Done.".green().bold());
    Ok(())
}
