//! Example: run pathfinding and (optionally) subscribe to Circles events over WS.
//!
//! Env vars:
//! - `CIRCLES_RPC_URL` (default: https://rpc.aboutcircles.com/)
//! - `CIRCLES_RPC_WS_URL` (default: wss://rpc.helsinki.aboutcircles.com/ws)
//!
//! Run with WS support:
//! `CIRCLES_RPC_URL=... CIRCLES_RPC_WS_URL=... cargo run -p circles-pathfinder --example path_and_events --features ws`

use alloy_primitives::{Address, aliases::U192};
use circles_pathfinder::find_path;
#[cfg(feature = "ws")]
use circles_rpc::CirclesRpc;
#[cfg(feature = "ws")]
use futures::StreamExt;

const DEFAULT_RPC_URL: &str = "https://rpc.aboutcircles.com/";
const DEFAULT_WS_URL: &str = "wss://rpc.helsinki.aboutcircles.com/ws";
// Sample addresses; replace with your own.
const SAMPLE_FROM: &str = "0xde374ece6fa50e781e81aac78e811b33d16912c7";
const SAMPLE_TO: &str = "0x6b69683c8897e3d18e74b1ba117b49f80423da5d";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_url = std::env::var("CIRCLES_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let ws_url = std::env::var("CIRCLES_RPC_WS_URL")
        .ok()
        .or_else(|| Some(DEFAULT_WS_URL.to_string()));

    let from: Address = SAMPLE_FROM.parse()?;
    let to: Address = SAMPLE_TO.parse()?;
    let amount = U192::from(1_000_000_000_000_000_000u64);

    // Pathfinding via HTTP
    println!("Finding path {from:?} -> {to:?}...");
    let transfers = find_path(&http_url, from, to, amount, true).await?;
    println!("Found {} transfers", transfers.len());

    // Optional WS subscription
    #[cfg(feature = "ws")]
    {
        if let Some(ws_url) = ws_url.as_deref() {
            match CirclesRpc::try_from_ws(ws_url).await {
                Ok(rpc_ws) => {
                    println!("Subscribing to Circles events for {to:#x}...");
                    let filter = serde_json::json!({ "address": to });
                    match rpc_ws.events().subscribe_parsed_events(filter).await {
                        Ok(mut sub) => {
                            let mut seen = 0u32;
                            while let Some(evt) =
                                tokio::time::timeout(std::time::Duration::from_secs(10), sub.next())
                                    .await
                                    .ok()
                                    .flatten()
                            {
                                match evt {
                                    Ok(e) => {
                                        println!(
                                            "event: {:?} @ block {}",
                                            e.event_type, e.base.block_number
                                        );
                                        seen += 1;
                                        if seen >= 3 {
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        println!("subscription error: {err}");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => println!("WS subscription failed: {err}"),
                    }
                }
                Err(err) => println!("WS connect failed; skipping subscription: {err}"),
            }
        } else {
            println!("WS URL not set; skipping subscription.");
        }
    }
    #[cfg(not(feature = "ws"))]
    println!("WS feature disabled; skipping subscription.");

    Ok(())
}
