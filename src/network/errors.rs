use std::fmt::Display;

use tokio::io;

pub enum NetworkError {
    IoError(io::Error),
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
        }
    }
}

pub type NetworkResult<T> = Result<T, NetworkError>;
