pub const CONTRACT_NAME: &str = "crates.io:stargaze-marketplace";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// 100% represented as basis points
pub const MAX_BASIS_POINTS: u64 = 10_000;

// Query limits
pub const DEFAULT_QUERY_LIMIT: u32 = 10;
pub const MAX_QUERY_LIMIT: u32 = 100;
