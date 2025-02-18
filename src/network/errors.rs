use std::fmt::Display;

#[derive(Debug)]
pub enum NetError {
    Str(String),
    IoError(tokio::io::Error),
}

impl NetError {
    pub fn str(msg: &str) -> Self {
        Self::Str(msg.to_string())
    }
}

impl From<tokio::io::Error> for NetError {
    fn from(value: tokio::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(v) => write!(f, "{}", v),
            Self::IoError(v) => write!(f, "{}", v),
        }
    }
}

impl std::error::Error for NetError {}

pub type NetResult<T> = Result<T, NetError>;
