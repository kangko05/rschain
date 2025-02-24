mod bootstrap;
mod full;
mod light;
mod mining;

pub use bootstrap::BootstrapNode;
pub use full::FullNode;
pub use mining::MiningNode;

pub const MAX_RETRY: usize = 3;
pub const NUM_CLOSE_NODES: usize = 20;
