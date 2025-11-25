use circles_types::{CirclesBaseEvent, CirclesEvent, CirclesEventType, RpcSubscriptionEvent};
use serde::de::Error as DeError;
use std::collections::HashMap;

/// Parse a raw RpcSubscriptionEvent into a typed CirclesEvent without losing bytes.
pub fn parse(event: RpcSubscriptionEvent) -> Result<CirclesEvent, serde_json::Error> {
    let mut values = event.values;

    // Extract base fields.
    let block_number = take_u64(&mut values, "blockNumber");
    let transaction_index = take_u32(&mut values, "transactionIndex");
    let log_index = take_u32(&mut values, "logIndex");
    let timestamp = take_u64(&mut values, "timestamp");
    let transaction_hash = take_str(&mut values, "transactionHash").and_then(|s| s.parse().ok());

    let base = CirclesBaseEvent {
        block_number: block_number.unwrap_or_default(),
        timestamp,
        transaction_index: transaction_index.unwrap_or_default(),
        log_index: log_index.unwrap_or_default(),
        transaction_hash,
    };

    let event_type = parse_event_type(&event.event)?;

    Ok(CirclesEvent {
        base,
        event_type,
        data: values,
    })
}

fn parse_event_type(raw: &str) -> Result<CirclesEventType, serde_json::Error> {
    // First try as-is using serde's rename mapping.
    if let Ok(et) = serde_json::from_str::<CirclesEventType>(&format!("\"{}\"", raw)) {
        return Ok(et);
    }
    // Try inserting an underscore after namespace prefix (e.g., CrcV2RegisterHuman -> CrcV2_RegisterHuman).
    if raw.starts_with("CrcV2") && !raw.contains('_') {
        let alt = format!("CrcV2_{}", &raw[5..]);
        if let Ok(et) = serde_json::from_str::<CirclesEventType>(&format!("\"{}\"", alt)) {
            return Ok(et);
        }
    }
    Err(serde_json::Error::custom(format!(
        "unknown event type: {}",
        raw
    )))
}

fn take_u64(map: &mut HashMap<String, serde_json::Value>, key: &str) -> Option<u64> {
    map.remove(key).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

fn take_u32(map: &mut HashMap<String, serde_json::Value>, key: &str) -> Option<u32> {
    take_u64(map, key).map(|v| v as u32)
}

fn take_str(map: &mut HashMap<String, serde_json::Value>, key: &str) -> Option<String> {
    map.remove(key).and_then(|v| match v {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}
