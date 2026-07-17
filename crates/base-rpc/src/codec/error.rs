use connectrpc::{error::ErrorCode, ConnectError};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("RLP decoding error: {0}")]
    RlpDecode(String),
    #[error("Block not found: {0}")]
    BlockNotFound(u64),
    #[error("Receipt not found for block: {0}")]
    ReceiptNotFound(u64),
    #[error("Invalid address length")]
    InvalidAddressLength,
    #[error("Invalid bloom length")]
    InvalidBloomLength,
    #[error("Invalid B256 length")]
    InvalidB256Length,
    #[error("Invalid B64 length")]
    InvalidB64Length,
    #[error("Provider error: {0}")]
    Provider(String),
}

impl From<CodecError> for ConnectError {
    fn from(err: CodecError) -> Self {
        ConnectError::new(ErrorCode::Internal, err.to_string())
    }
}
