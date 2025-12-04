use crate::SdkError;
use circles_rpc::{CirclesRpc, events::subscription::CirclesSubscription};
use circles_types::{CirclesEvent, Filter};
use futures::StreamExt;
use serde_json::Value;
use tokio::time::sleep;
use tracing::warn;

/// Subscribe with retry/backoff on websocket disconnects.
pub async fn subscribe_with_retries(
    ws_url: &str,
    filter: Value,
    max_attempts: Option<usize>,
) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
    let mut attempt = 0usize;
    let cap = max_attempts.unwrap_or(5);
    loop {
        attempt += 1;
        let ws = match CirclesRpc::try_from_ws(ws_url).await {
            Ok(ws) => ws,
            Err(_err) if attempt < cap => {
                let delay_ms = (1u64 << attempt).saturating_mul(1000).min(10_000);
                sleep(std::time::Duration::from_millis(delay_ms)).await;
                continue;
            }
            Err(err) => {
                return Err(SdkError::WsSubscribeFailed {
                    attempts: attempt,
                    reason: err.to_string(),
                });
            }
        };
        let sub_res = ws.events().subscribe_parsed_events(filter.clone()).await;
        match sub_res {
            Ok(sub) => return Ok(sub),
            Err(_err) if attempt < cap => {
                let delay_ms = (1u64 << attempt).saturating_mul(1000).min(10_000);
                sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Err(err) => {
                return Err(SdkError::WsSubscribeFailed {
                    attempts: attempt,
                    reason: err.to_string(),
                });
            }
        }
    }
}

/// Fetch historical events (HTTP) then subscribe live (WS).
pub async fn subscribe_with_catchup(
    rpc: &CirclesRpc,
    ws_url: &str,
    filter: Value,
    max_attempts: Option<usize>,
    catch_up_from_block: Option<u64>,
    catch_up_filter: Option<Vec<Filter>>,
    address: Option<circles_types::Address>,
) -> Result<(Vec<CirclesEvent>, CirclesSubscription<CirclesEvent>), SdkError> {
    let catch_up_events = if let Some(from_block) = catch_up_from_block {
        rpc.events()
            .circles_events(address, from_block, None, catch_up_filter)
            .await?
    } else {
        Vec::new()
    };
    let sub = subscribe_with_retries(ws_url, filter, max_attempts).await?;
    Ok((catch_up_events, sub))
}

/// Spawn a handler over a subscription; errors are logged via tracing at warn.
pub fn spawn_event_handler<H>(
    mut sub: CirclesSubscription<CirclesEvent>,
    handler: H,
) -> tokio::task::JoinHandle<()>
where
    H: Fn(CirclesEvent) + Send + Sync + 'static,
{
    let handler = std::sync::Arc::new(handler);
    tokio::spawn(async move {
        while let Some(item) = sub.next().await {
            match item {
                Ok(evt) => handler(evt),
                Err(err) => {
                    warn!(error = %err, "ws event stream error");
                }
            }
        }
    })
}
