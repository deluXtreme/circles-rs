use std::str::FromStr;

use alloy_primitives::Address;
use circles_sdk::config;
use circles_types::CirclesConfig;

fn live_enabled() -> bool {
    std::env::var("RUN_LIVE").as_deref() == Ok("1")
}

/// Returns a live config if `RUN_LIVE=1`, applying env overrides when set.
pub fn maybe_live_config() -> Option<CirclesConfig> {
    if !live_enabled() {
        return None;
    }
    let mut cfg = config::gnosis_mainnet();
    if let Ok(url) = std::env::var("CIRCLES_RPC_URL") {
        cfg.circles_rpc_url = url;
    }
    if let Ok(url) = std::env::var("CIRCLES_PATHFINDER_URL") {
        cfg.pathfinder_url = url;
    }
    if let Ok(url) = std::env::var("CIRCLES_PROFILE_URL") {
        cfg.profile_service_url = url;
    }
    Some(cfg)
}

/// Reads `LIVE_AVATAR` as a hex address when `RUN_LIVE=1`.
pub fn maybe_live_avatar() -> Option<Address> {
    if !live_enabled() {
        return None;
    }
    let addr = std::env::var("LIVE_AVATAR").ok()?;
    Address::from_str(addr.as_str()).ok()
}
