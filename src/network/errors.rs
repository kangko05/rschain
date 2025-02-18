#![allow(dead_code)]

use std::fmt::Display;

#[derive(Debug)]
pub enum NetError {
    String(String),
    IoError(tokio::io::Error),
    AddrParseError(std::net::AddrParseError),
    SerdeError(serde_json::Error),
}

impl NetError {
    pub fn str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

impl From<tokio::io::Error> for NetError {
    fn from(value: tokio::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<std::net::AddrParseError> for NetError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::AddrParseError(value)
    }
}

impl From<serde_json::Error> for NetError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(value)
    }
}

impl Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(v) => write!(f, "{}", v),
            Self::IoError(v) => write!(f, "{}", v),
            Self::AddrParseError(v) => write!(f, "{}", v),
            Self::SerdeError(v) => write!(f, "{}", v),
        }
    }
}

pub type NetResult<T> = Result<T, NetError>;
