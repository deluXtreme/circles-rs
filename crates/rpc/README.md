# Circles RPC (Rust)

Async JSON-RPC client for the Circles protocol, mirroring the TS `@rpc` package while leaning on Alloy transports and shared Circles types.

## Features
- Thin `CirclesRpc` facade with method groups (`balance`, `token`, `trust`, `avatar`, `profile`, `query`, `events`, `invitation`, `pathfinder`, `group`, `tables`, `health`, `network`, `search`).
- HTTP constructor helpers (`try_from_http`, `TryFrom<&str>`); WS subscriptions behind the `ws` feature with best-effort `eth_unsubscribe` on drop.
- `circles_query` helpers with cursor extraction plus `PagedQuery`/`paged_stream` convenience; `paged_query` is validated against live `circles_query`.
- Normalized token holder balances (`TokenHolderNormalized`) and invitation batching with bounded concurrency.
- WS parsing tolerates heartbeats (`[]`), flattens batch frames, and maps unknown event types to `CrcUnknownEvent`.

## Quickstart
```rust
use circles_rpc::CirclesRpc;
use circles_types::Address;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CirclesRpc::try_from("https://rpc.helsinki.aboutcircles.com/")?;

    // Total balance (v2)
    let addr: Address = "0xde374ece6fa50e781e81aac78e811b33d16912c7".parse()?;
    let total = rpc.balance().get_total_balance(addr, false, true).await?;
    println!("total v2 balance: {}", total.0);

    // Token holders (normalized U256 balances)
    let holders = rpc.token().get_token_holders(addr).await?;
    println!("holders count: {}", holders.len());
    Ok(())
}
```

## Paged queries
```rust
use circles_rpc::CirclesRpc;
use circles_types::{PagedQueryParams, SortOrder, Address};

#[derive(Debug, serde::Deserialize)]
struct AvatarRow { avatar: Address, timestamp: u64 }

async fn first_page(rpc: &CirclesRpc) -> circles_rpc::Result<()> {
    let params = PagedQueryParams {
        namespace: "V_Crc".into(),
        table: "Avatars".into(),
        sort_order: SortOrder::DESC,
        columns: vec![],
        filter: None,
        limit: 50,
    };

    // Pull one page
    let mut pager = rpc.paged_query::<AvatarRow>(params.clone());
    if let Some(page) = pager.next_page().await? {
        println!("fetched {} rows, has_more={}", page.items.len(), page.has_more);
    }

    // Or stream rows
    let mut stream = rpc.paged_stream::<AvatarRow>(params);
    while let Some(row) = stream.next().await.transpose()? {
        println!("row: {:?}", row);
    }
    Ok(())
}
```

## Events
HTTP fetch:
```rust
let events = rpc
    .events()
    .circles_events(Some(addr), 0, None, None)
    .await?;
```

WebSocket subscription (enable the `ws` feature):
```rust
#[cfg(feature = "ws")]
{
    let sub = rpc.events().subscribe_parsed_events(serde_json::json!({ "address": addr })).await?;
    tokio::pin!(sub);
    while let Some(evt) = sub.next().await.transpose()? {
        println!("event: {:?}", evt.event_type);
    }
}
```

### WS behavior notes
- Public endpoints (`wss://rpc.aboutcircles.com/ws`, `wss://rpc.helsinki.aboutcircles.com/ws`) emit periodic empty arrays; these are dropped.
- Event frames can arrive batched (array-of-arrays); they are flattened before parsing.
- Unknown event types are surfaced as `CrcUnknownEvent`. We have observed `CrcV2_TransferSingle` batches and an unknown `CrcV2_TransferSummary` type; schema validation on a busier node is still pending.
- Reconnect/backoff is not automatic; see the SDK crate for retry/catch-up helpers if you need them.

## Examples
- `paged_and_ws`: fetch one page of avatars via `circles_query` and, with the `ws` feature, subscribe to Circles events. Tolerates heartbeats/batches and logs unknown events without panicking.
```bash
CIRCLES_RPC_URL=https://rpc.aboutcircles.com/ \
  CIRCLES_RPC_WS_URL=wss://rpc.aboutcircles.com/ws \
  cargo run -p circles-rpc --example paged_and_ws --features ws
```

## Status / TODO
- Subscription resilience (reconnect/backoff) is best-effort only.
- Pagination helpers could gain table-aware defaults and richer ergonomics.
- Transaction sending is intentionally omitted; transfers are handled by a separate crate in this workspace.
