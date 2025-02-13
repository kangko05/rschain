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

// block errors
#[derive(Debug)]
pub enum BlockError {
    InvalidTimeStamp,
    InvalidMerkleRoot,
    InvalidPOW,
    InvalidPreviousHash,
    SystemTimeError(SystemTimeError),
    MerkleError(MerkleError),
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

impl Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTimeStamp => write!(f, "invalid timestamp"),
            Self::InvalidMerkleRoot => write!(f, "invalid merkle root"),
            Self::InvalidPOW => write!(f, "invalid proof of work"),
            Self::InvalidPreviousHash => write!(f, "invalid previous hash"),
            Self::SystemTimeError(err) => write!(f, "failed to get current time: {}", err),
            Self::MerkleError(merr) => write!(f, "{}", merr),
        }
    }
}

impl Error for BlockError {}

pub type BlockResult<T> = Result<T, BlockError>;
