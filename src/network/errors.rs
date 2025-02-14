#![allow(dead_code)]

use std::{error::Error, fmt::Display, net::AddrParseError};

use tokio::{io, task::JoinError};

#[derive(Debug)]
pub enum NetworkError {
    String(String),
    IoError(io::Error),
    AddrParseError(AddrParseError),
    JoinHandleError(JoinError),
    SerdeJsonError(serde_json::Error),
}

impl NetworkError {
    pub fn str(s: &str) -> Self {
        Self::String(s.to_string())
    }
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

impl From<serde_json::Error> for NetworkError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJsonError(value)
    }
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::IoError(err) => write!(f, "{}", err),
            Self::AddrParseError(err) => write!(f, "{}", err),
            Self::JoinHandleError(err) => write!(f, "{}", err),
            Self::SerdeJsonError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for NetworkError {}

pub type NetworkResult<T> = Result<T, NetworkError>;
