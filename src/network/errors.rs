#![allow(dead_code)]

use std::fmt::Display;
use std::io;

#[derive(Debug)]
pub enum NetError {
    Str(String),
    IoErr(io::Error),
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

impl Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => write!(f, "{}", s),
            Self::IoErr(err) => write!(f, "{}", err),
        }
    }
}

pub type NetResult<T> = Result<T, NetError>;
