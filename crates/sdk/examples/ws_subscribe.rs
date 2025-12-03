#![cfg_attr(not(feature = "ws"), allow(dead_code, unused_imports))]

use alloy_primitives::address;
#[cfg(feature = "ws")]
use circles_sdk::ws;
use circles_sdk::{Sdk, config};
use circles_types::CirclesConfig;

#[cfg(feature = "ws")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: CirclesConfig = config::gnosis_mainnet();

    let sdk = Sdk::new(config, None)?;
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let ws_url = "wss://rpc.aboutcircles.com/ws";

    // Default filter: address of the avatar.
    let filter = serde_json::json!({ "address": format!("{:#x}", avatar) });
    let (catch_up, sub) = sdk
        .subscribe_events_ws_with_catchup(ws_url, filter, Some(3), None, None)
        .await?;

    println!("Catch-up events: {}", catch_up.len());

    let handle = ws::spawn_event_handler(sub, move |evt| {
        println!("Event: {:?}", evt.event_type);
    });

    // Run for a short time then exit.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    handle.abort();
    Ok(())
}

#[cfg(not(feature = "ws"))]
fn main() {
    println!("ws_subscribe example requires --features ws");
}
