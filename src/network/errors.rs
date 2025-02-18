#![allow(dead_code)]

use std::fmt::Display;

use std::error::Error;

#[derive(Debug)]
pub enum NetworkError {
    Str(String),
    SerdeJsonError(serde_json::Error),
    IoError(tokio::io::Error),
    ConnectionClosed,
}

impl From<serde_json::Error> for NetworkError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJsonError(value)
    }
}

impl From<tokio::io::Error> for NetworkError {
    fn from(value: tokio::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl NetworkError {
    pub fn str(msg: &str) -> Self {
        Self::Str(msg.to_string())
    }
}

impl Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(v) => write!(f, "{}", v),
            Self::SerdeJsonError(v) => write!(f, "{}", v),
            Self::IoError(v) => write!(f, "{}", v),
            Self::ConnectionClosed => write!(f, "connection closed"),
        }
    }
}

impl Error for NetworkError {}

pub type NetworkResult<T> = Result<T, NetworkError>;
