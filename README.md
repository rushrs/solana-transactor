# Solana Transaction Client

Solana TX client which I created when learning how to work with rust.


## Features

- Connect to Solana devnet
- Submit transactions with retry logic
- Comprehensive error handling
- Transaction monitoring with Prometheus metrics
- Command-line interface for configuration

## Prerequisites

- Rust and Cargo installed
- A Solana keypair (or the application can generate one for you)
- SOL in your devnet wallet for transaction fees

## Setup

1. Clone the repository
2. Install dependencies:

```bash
cargo build
```

## Usage

Run the application with the following command:

```bash
cargo run -- --rpc-url https://api.devnet.solana.com --keypair-path /path/to/your/keypair.json --num-transactions 10 --max-retries 3
```

### Command-line Arguments

- `--rpc-url`: Solana RPC URL (default: https://api.devnet.solana.com)
- `--keypair-path`: Path to your Solana keypair file (optional; if not provided, a new keypair will be generated)
- `--num-transactions`: Number of sample transactions to send (default: 10)
- `--max-retries`: Maximum number of retries for each transaction (default: 3)
- `--metrics-port`: Port for Prometheus metrics server (default: 9000)

## Monitoring

The application exposes Prometheus metrics on the specified port (default: 9000). The following metrics are available:

- `solana.wallet.balance`: Current wallet balance in SOL
- `solana.transactions.total`: Total number of transactions attempted
- `solana.transactions.success`: Number of successful transactions
- `solana.transactions.failed`: Number of failed transactions
- `solana.transaction.latency`: Transaction latency in milliseconds

You can configure Prometheus to scrape these metrics and visualize them in Grafana.

## Error Handling

The application handles various types of errors:

- RPC connection errors
- Transaction submission errors
- Confirmation timeout errors
- Insufficient funds errors

The retry logic automatically retries failed transactions with exponential backoff, up to the specified maximum number of retries.
