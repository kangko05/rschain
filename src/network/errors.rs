use std::{fmt::Display, net::AddrParseError};

use tokio::{io, task::JoinError};

#[derive(Debug)]
pub enum NetworkError {
    IoError(io::Error),
    AddrParseError(AddrParseError),
    JoinHandleError(JoinError),
}

impl From<JoinError> for NetworkError {
    fn from(value: JoinError) -> Self {
        Self::JoinHandleError(value)
    }
}

impl From<AddrParseError> for NetworkError {
    fn from(value: AddrParseError) -> Self {
        Self::AddrParseError(value)
    }
}

impl From<io::Error> for NetworkError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "{}", err),
            Self::AddrParseError(err) => write!(f, "{}", err),
            Self::JoinHandleError(err) => write!(f, "{}", err),
        }
    }
}

pub type NetworkResult<T> = Result<T, NetworkError>;
