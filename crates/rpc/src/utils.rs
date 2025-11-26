//! Shared helpers for address normalization and conversions.

use alloy_primitives::Address;
use circles_types::{Cursor, EventRow};

/// Ensure addresses are consistently checksummed.
pub fn normalize_address(addr: Address) -> Address {
    addr
}

/// Build a cursor filter tuple (column, value) based on the provided cursor.
pub fn cursor_filters(cursor: &Cursor, sort_desc: bool) -> Vec<(String, serde_json::Value)> {
    let op = if sort_desc { "<=" } else { ">=" };
    vec![
        (
            format!("blockNumber {op}"),
            serde_json::json!(cursor.block_number),
        ),
        (
            format!("transactionIndex {op}"),
            serde_json::json!(cursor.transaction_index),
        ),
        (
            format!("logIndex {op}"),
            serde_json::json!(cursor.log_index),
        ),
    ]
}

/// Extract cursors from rows when the row type includes the base event fields.
pub fn extract_cursor(row: &impl CursorLike) -> Cursor {
    row.to_cursor()
}

pub trait CursorLike {
    fn to_cursor(&self) -> Cursor;
}

impl CursorLike for EventRow {
    fn to_cursor(&self) -> Cursor {
        Cursor {
            block_number: self.block_number,
            transaction_index: self.transaction_index,
            log_index: self.log_index,
            batch_index: self.batch_index,
            timestamp: self.timestamp,
        }
    }
}
