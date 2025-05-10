use crate::error::TransactionError;
use log::{debug, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// Service for managing Solana transactions
pub struct TransactionService {
    client: Arc<RpcClient>,
    max_retries: u32,
}

impl TransactionService {
    /// Create a new TransactionService
    pub fn new(client: Arc<RpcClient>, max_retries: u32) -> Self {
        Self {
            client,
            max_retries,
        }
    }

    /// Get the balance of a Solana account
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, TransactionError> {
        match self.client.get_balance(pubkey) {
            Ok(balance) => Ok(balance),
            Err(err) => Err(TransactionError::RpcError(err.to_string())),
        }
    }

    /// Submit a transaction with retry logic
    pub async fn submit_transaction(
        &self,
        payer: &Keypair,
        instructions: Vec<Instruction>,
    ) -> Result<Signature, TransactionError> {
        let mut attempt = 0;
        let blockhash_query_interval = Duration::from_millis(1000);

        // Retry logic
        loop {
            attempt += 1;

            // Check if we've exceeded max retries
            if attempt > self.max_retries + 1 {
                return Err(TransactionError::MaxRetriesExceeded);
            }

            if attempt > 1 {
                debug!(
                    "Retrying transaction (attempt {}/{})",
                    attempt - 1,
                    self.max_retries
                );
                // Add exponential backoff
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 2));
                sleep(backoff).await;
            }

            // Get a recent blockhash
            let recent_blockhash = match self.client.get_latest_blockhash() {
                Ok(blockhash) => blockhash,
                Err(err) => {
                    warn!("Failed to get recent blockhash: {}", err);
                    sleep(blockhash_query_interval).await;
                    continue;
                }
            };

            // Create the transaction
            let message = Message::new(&instructions, Some(&payer.pubkey()));
            let mut tx = Transaction::new_unsigned(message);
            tx.sign(&[payer], recent_blockhash);

            // Send the transaction
            match self.send_and_confirm_transaction(&tx).await {
                Ok(signature) => return Ok(signature),
                Err(err) => {
                    warn!("Transaction failed: {}", err);

                    // If the error is non-retriable, fail immediately
                    if !Self::is_retriable_error(&err) {
                        return Err(err);
                    }
                }
            }
        }
    }

    /// Send and confirm a transaction
    async fn send_and_confirm_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<Signature, TransactionError> {
        let signature = match self.client.send_transaction(tx) {
            Ok(sig) => sig,
            Err(err) => return Err(TransactionError::SendError(err.to_string())),
        };

        // Wait for confirmation
        match self
            .client
            .confirm_transaction_with_commitment(&signature, CommitmentConfig::confirmed())
        {
            Ok(result) => {
                if !result.value {
                    return Err(TransactionError::ConfirmationError(
                        "Transaction was not confirmed".to_string(),
                    ));
                }
            }
            Err(err) => return Err(TransactionError::ConfirmationError(err.to_string())),
        }

        Ok(signature)
    }

    /// Determine if an error is retriable
    fn is_retriable_error(err: &TransactionError) -> bool {
        match err {
            TransactionError::RpcError(_) => true,
            TransactionError::SendError(msg) => {
                // Common retriable errors
                msg.contains("blockhash not found")
                    || msg.contains("timeout")
                    || msg.contains("socket closed")
                    || msg.contains("retry rate limit")
                    || msg.contains("too many requests")
            }
            TransactionError::ConfirmationError(msg) => {
                msg.contains("timeout") || msg.contains("connection closed")
            }
            _ => false,
        }
    }
}
