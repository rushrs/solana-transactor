use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Transaction send error: {0}")]
    SendError(String),

    #[error("Transaction confirmation error: {0}")]
    ConfirmationError(String),

    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,

    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for TransactionError {
    fn from(error: anyhow::Error) -> Self {
        TransactionError::Other(error.to_string())
    }
}
