use std::{error::Error, fmt::Display, time::SystemTimeError};

// merkle errors
#[derive(Debug)]
pub enum MerkleError {
    Message(String),
}

impl MerkleError {
    pub fn err(msg: &str) -> MerkleError {
        Self::Message(msg.to_string())
    }
}

impl Error for MerkleError {}

impl Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::Message(s) => write!(f, "{}", s),
        }
    }
}

pub type MerkleResult<T> = Result<T, MerkleError>;

// block errors - this contains tx error inside
#[derive(Debug)]
pub enum BlockError {
    Message(String),
    InvalidTimeStamp,
    InvalidMerkleRoot,
    InvalidPOW,
    InvalidPreviousHash,
    EmptyBlock,
    SystemTimeError(SystemTimeError),
    MerkleError(MerkleError),
    SerializeError(bincode::Error),
    TxError(TxError),
}

impl BlockError {
    pub fn from_str(msg: &str) -> Self {
        Self::Message(msg.to_string())
    }
}

impl From<TxError> for BlockError {
    fn from(value: TxError) -> Self {
        Self::TxError(value)
    }
}

impl From<SystemTimeError> for BlockError {
    fn from(value: SystemTimeError) -> Self {
        Self::SystemTimeError(value)
    }
}

impl From<MerkleError> for BlockError {
    fn from(value: MerkleError) -> Self {
        Self::MerkleError(value)
    }
}

impl From<bincode::Error> for BlockError {
    fn from(value: bincode::Error) -> Self {
        Self::SerializeError(value)
    }
}

impl Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(s) => write!(f, "{}", s),
            Self::InvalidTimeStamp => write!(f, "invalid timestamp"),
            Self::InvalidMerkleRoot => write!(f, "invalid merkle root"),
            Self::InvalidPOW => write!(f, "invalid proof of work"),
            Self::InvalidPreviousHash => write!(f, "invalid previous hash"),
            Self::EmptyBlock => write!(f, "attempt to add empty block to the chain"),
            Self::SystemTimeError(err) => write!(f, "failed to get current time: {}", err),
            Self::MerkleError(merr) => write!(f, "{}", merr),
            Self::SerializeError(serr) => write!(f, "{}", serr),
            Self::TxError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for BlockError {}

pub type BlockResult<T> = Result<T, BlockError>;

// transaction error

#[derive(Debug)]
pub enum TxError {
    SerializeError(bincode::Error),
    InvalidHashLength { expected: usize, actual: usize },
    InvalidUtxo,
    UTXONotSufficient { utxo_value: u64, sending_value: u64 },
    SecpError(secp256k1::Error),
    SystemTimeError(SystemTimeError),
}

impl From<bincode::Error> for TxError {
    fn from(value: bincode::Error) -> Self {
        Self::SerializeError(value)
    }
}

impl From<Vec<u8>> for TxError {
    fn from(value: Vec<u8>) -> Self {
        Self::InvalidHashLength {
            expected: 32,
            actual: value.len(),
        }
    }
}

impl From<secp256k1::Error> for TxError {
    fn from(value: secp256k1::Error) -> Self {
        Self::SecpError(value)
    }
}

impl From<SystemTimeError> for TxError {
    fn from(value: SystemTimeError) -> Self {
        Self::SystemTimeError(value)
    }
}

impl Display for TxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializeError(err) => write!(f, "{}", err),
            Self::InvalidHashLength { expected, actual } => {
                write!(
                    f,
                    "invalid hash length: expected {} bytes but got {} bytes",
                    expected, actual
                )
            }
            Self::InvalidUtxo => write!(f, "utxo not found"),
            Self::UTXONotSufficient {
                utxo_value,
                sending_value,
            } => write!(f, "trying to send {} from {}", sending_value, utxo_value),
            Self::SecpError(err) => write!(f, "{}", err),
            Self::SystemTimeError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for TxError {}

pub type TxResult<T> = Result<T, TxError>;

// tx pool
#[derive(Debug)]
pub enum TxpoolError {
    IndexOutOfBound,
}

impl Display for TxpoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexOutOfBound => write!(f, "index out of bound"),
        }
    }
}

pub type TxpoolResult<T> = Result<T, TxpoolError>;
