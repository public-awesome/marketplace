pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// 100% represented as basis points
pub const MAX_BASIS_POINTS: u64 = 10_000;

// Address authorized to manage the token blacklist
pub const BLACKLIST_MANAGER: &str =
    "stars14ay8uhnnacg79dvygpf5rh9wz2m7tj8jnw60w233xja4akax4mrs54770s";
