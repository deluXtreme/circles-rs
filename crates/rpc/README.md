# Circles RPC (Rust)

Async JSON-RPC client for the Circles protocol, mirroring the TypeScript SDK’s `@rpc` package while leaning on Alloy transports and shared Circles types.

## Features

- Thin `CirclesRpc` facade with method groups (`balance`, `token`, `trust`, `avatar`, `query`, `events`, `invitation`, `pathfinder`, `group`, `tables`, `health`, `network`, `search`).
- Uses `alloy-provider` for HTTP; WebSocket subscriptions are available behind the `ws` feature (best-effort `eth_unsubscribe` on drop).
- `TryFrom<&str>` / `From<reqwest::Url>` constructors instead of bespoke builders.
- `circles_query` helpers with cursor extraction and `PagedQuery`/`paged_stream` convenience.
- Normalized token holder balances (`TokenHolderNormalized`) and invitation batching with bounded concurrency.

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

### WS caveat (Helsinki)

The public WS endpoints we tested (`wss://rpc.aboutcircles.com/ws`, `wss://rpc.helsinki.aboutcircles.com/ws`) currently emit periodic empty-array frames (`[]`) and, so far, no actual event payloads. Our example logs and exits gracefully when parsing fails. If you have access to a busier node, try pointing `CIRCLES_RPC_WS_URL` there; we may need to filter heartbeats or adjust parsing once real payloads are observed.

## Status / TODO

- Subscription resilience (reconnect/backoff, pending cleanup) is best-effort today.
- Pagination helpers are stable but could gain table-aware defaults and richer ergonomics.
- Transaction/transfer RPCs are intentionally omitted; handled by a separate crate in this workspace.

## Examples

- `paged_and_ws`: fetch one page of avatars via `circles_query` and, with the `ws` feature, subscribe to Circles events.

```bash
CIRCLES_RPC_URL=https://rpc.aboutcircles.com/ \
  CIRCLES_RPC_WS_URL=wss://rpc.aboutcircles.com/ws \
  cargo run -p circles-rpc --example paged_and_ws --features ws
```

The WS example is best-effort: if the endpoint denies pubsub or no events arrive within a short timeout, it will log and exit gracefully.
