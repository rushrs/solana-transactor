use anyhow::Result;
use clap::Parser;
use log::{error, info};
use metrics::{counter, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;

mod error;
mod transaction_service;

use error::TransactionError;
use transaction_service::TransactionService;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Solana RPC URL (devnet by default)
    #[clap(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to keypair file
    #[clap(long)]
    keypair_path: Option<String>,

    /// Number of sample transactions to send
    #[clap(long, default_value = "10")]
    num_transactions: u32,

    /// Maximum number of retries for each transaction
    #[clap(long, default_value = "3")]
    max_retries: u32,

    /// Metrics port for Prometheus
    #[clap(long, default_value = "9000")]
    metrics_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Setup metrics exporter
    let builder = PrometheusBuilder::new();
    let handle = builder
        .with_http_listener(([0, 0, 0, 0], args.metrics_port))
        .install()?;

    info!("Metrics server running on port {}", args.metrics_port);

    // Initialize RPC client
    let rpc_client =
        RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());

    // Load or generate keypair
    let payer = match args.keypair_path {
        Some(path) => {
            let keypair_bytes = std::fs::read(path)?;
            Keypair::from_bytes(&keypair_bytes)?
        }
        None => {
            info!("No keypair provided, generating a new one");
            Keypair::new()
        }
    };

    info!("Using address: {}", payer.pubkey());

    // Create transaction service
    let transaction_service = Arc::new(TransactionService::new(
        Arc::new(rpc_client),
        args.max_retries,
    ));

    // Display wallet balance
    let balance = transaction_service.get_balance(&payer.pubkey()).await?;
    info!("Wallet balance: {} SOL", balance as f64 / 1_000_000_000.0);

    if balance < 1_000_000 {
        error!("Insufficient balance. Airdrop SOL to your wallet before proceeding.");
        return Ok(());
    }

    // Initialize metrics
    gauge!("solana.wallet.balance", balance as f64 / 1_000_000_000.0);
    counter!("solana.transactions.total", 0);
    counter!("solana.transactions.success", 0);
    counter!("solana.transactions.failed", 0);

    // Run sample transactions
    for i in 0..args.num_transactions {
        info!("Sending transaction {}/{}", i + 1, args.num_transactions);

        // Create a random recipient
        let recipient = Keypair::new().pubkey();

        // Create a transfer instruction (just sending a tiny amount)
        let instruction = system_instruction::transfer(
            &payer.pubkey(),
            &recipient,
            100, // 100 lamports, a very small amount
        );

        // Submit the transaction
        let start = Instant::now();
        let result = transaction_service
            .submit_transaction(&payer, vec![instruction])
            .await;

        // Update metrics
        counter!("solana.transactions.total", 1);

        match result {
            Ok(signature) => {
                let elapsed = start.elapsed();
                info!(
                    "Transaction succeeded after {}ms: {}",
                    elapsed.as_millis(),
                    signature
                );
                counter!("solana.transactions.success", 1);
                gauge!("solana.transaction.latency", elapsed.as_millis() as f64);
            }
            Err(e) => {
                error!("Transaction failed: {}", e);
                counter!("solana.transactions.failed", 1);
            }
        }

        // Add small delay between transactions
        sleep(Duration::from_millis(500)).await;
    }

    info!("All transactions completed.");

    // Let metrics be scraped
    sleep(Duration::from_secs(5)).await;

    Ok(())
}
