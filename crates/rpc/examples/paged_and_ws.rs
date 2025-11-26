//! Minimal example hitting the public Circles RPC:
//! - paged `circles_query` over V_Crc.Avatars
//! - optional websocket subscription for Circles events
//!
//! Run with:
//! `CIRCLES_RPC_URL=https://rpc.aboutcircles.com/ cargo run -p circles-rpc --example paged_and_ws --features ws`

use circles_rpc::CirclesRpc;
use circles_rpc::Result;
use circles_types::{Address, PagedQueryParams, SortOrder};
use futures::StreamExt;
use serde::Deserialize;
use std::time::Duration;

const DEFAULT_RPC_URL: &str = "https://rpc.aboutcircles.com/";
const DEFAULT_WS_URL: &str = "wss://rpc.aboutcircles.com/ws";
// Sample address from docs; replace with any avatar you care about.
const SAMPLE_AVATAR: &str = "0xde374ece6fa50e781e81aac78e811b33d16912c7";

#[derive(Debug, Deserialize, serde::Serialize, Clone)]
struct AvatarRow {
    avatar: Address,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = std::env::var("CIRCLES_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let rpc = CirclesRpc::try_from(rpc_url.as_str())?;

    let avatar: Address = SAMPLE_AVATAR.parse().expect("valid sample address");

    // --- Paged query example (first page) ---
    let params = PagedQueryParams {
        namespace: "V_Crc".to_string(),
        table: "Avatars".to_string(),
        sort_order: SortOrder::DESC,
        columns: vec!["avatar".into(), "timestamp".into()],
        filter: None,
        limit: 10,
    };

    let mut pager = rpc.paged_query::<AvatarRow>(params);
    if let Some(page) = pager.next_page().await? {
        println!(
            "Fetched {} avatar rows (has_more={})",
            page.items.len(),
            page.has_more
        );
        for row in page.items {
            println!("avatar {} @ {}", row.avatar, row.timestamp);
        }
    } else {
        println!("No rows returned");
    }

    // --- WebSocket events example (requires `ws` feature) ---
    #[cfg(feature = "ws")]
    {
        let ws_url =
            std::env::var("CIRCLES_RPC_WS_URL").unwrap_or_else(|_| DEFAULT_WS_URL.to_string());
        match CirclesRpc::try_from_ws(ws_url.as_str()).await {
            Ok(rpc_ws) => stream_events(&rpc_ws, avatar).await?,
            Err(e) => println!("WebSocket connect failed, skipping subscription: {e}"),
        }
    }
    #[cfg(not(feature = "ws"))]
    {
        println!("WebSocket example skipped; enable the `ws` feature to run it.");
    }

    Ok(())
}

#[cfg(feature = "ws")]
async fn stream_events(rpc: &CirclesRpc, address: Address) -> Result<()> {
    use tokio::time::timeout;

    println!("Subscribing to Circles events for address {address}");
    // Empty filter = firehose; add {"address": <addr>} to narrow. Some public WS
    // endpoints currently emit empty [] heartbeats and no payloads.
    let filter = serde_json::json!({});
    let mut sub = match rpc.events().subscribe_parsed_events(filter).await {
        Ok(sub) => sub,
        Err(e) => {
            eprintln!("subscription failed: {e}");
            return Ok(());
        }
    };

    let mut seen = 0u32;
    loop {
        match timeout(Duration::from_secs(15), sub.next()).await {
            Ok(Some(Ok(evt))) => {
                println!(
                    "event: {:?} @ block {}",
                    evt.event_type, evt.base.block_number
                );
                seen += 1;
                if seen >= 3 {
                    break;
                }
            }
            Ok(Some(Err(e))) => {
                println!("event stream error: {e}");
                break;
            }
            Ok(None) => {
                println!("subscription closed");
                break;
            }
            Err(_) => {
                println!("no events within timeout, stopping");
                break;
            }
        }
    }

    println!("Unsubscribing after {seen} events");
    // Drop will best-effort eth_unsubscribe, but explicit is fine too.
    let _ = sub.unsubscribe();

    // Debug helper: also try a raw Value subscription to inspect payloads when parsing fails.
    debug_raw_ws(address).await;
    Ok(())
}

#[cfg(feature = "ws")]
async fn debug_raw_ws(address: Address) {
    use alloy_provider::{Identity, Provider, ProviderBuilder};
    use alloy_transport_ws::WsConnect;
    use futures::StreamExt;
    use serde_json::Value;

    let ws_url = std::env::var("CIRCLES_RPC_WS_URL").unwrap_or_else(|_| DEFAULT_WS_URL.to_string());
    println!("Debugging raw WS frames from {ws_url}");

    let provider: alloy_provider::RootProvider =
        match ProviderBuilder::<Identity, Identity>::default()
            .connect_ws(WsConnect::new(ws_url.clone()))
            .await
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("raw ws connect failed: {e}");
                return;
            }
        };

    let filter = serde_json::json!({ "address": address });
    let sub = provider.subscribe::<_, Value>(("circles", filter));
    match circles_rpc::EventStream::from_subscription(sub).await {
        Ok((mut stream, _id)) => {
            let mut count = 0u8;
            while let Some(msg) = stream.next().await {
                println!("raw ws message: {msg:?}");
                count += 1;
                if count >= 3 {
                    break;
                }
            }
        }
        Err(e) => eprintln!("failed to build raw stream: {e}"),
    }
}
