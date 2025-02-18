use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    Str(String),
    GetPeers,
    Ok,
}

impl NetMessage {
    pub fn str(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}
