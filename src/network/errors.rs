#![allow(dead_code)]

use std::error::Error;
use std::fmt::Display;
use std::io;

use crate::blockchain::BlockError;

#[derive(Debug)]
pub enum NetError {
    Str(String),
    IoErr(io::Error),
    SerdeJsonErr(serde_json::Error),
    InvalidBlocks(BlockError),
}

impl NetError {
    pub fn str(msg: &str) -> Self {
        Self::Str(msg.to_string())
    }
}

impl From<io::Error> for NetError {
    fn from(value: io::Error) -> Self {
        Self::IoErr(value)
    }
}

impl From<serde_json::Error> for NetError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJsonErr(value)
    }
}

impl From<BlockError> for NetError {
    fn from(value: BlockError) -> Self {
        Self::InvalidBlocks(value)
    }
}

impl Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => write!(f, "{}", s),
            Self::IoErr(err) => write!(f, "{}", err),
            Self::SerdeJsonErr(err) => write!(f, "{}", err),
            Self::InvalidBlocks(err) => write!(f, "{}", err),
        }
    }
}

impl Error for NetError {}

pub type NetResult<T> = Result<T, NetError>;
