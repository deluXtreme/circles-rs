use crate::client::RpcClient;
use crate::error::{CirclesRpcError, Result};
use circles_types::{CirclesQueryResponse, OrderBy, PagedQueryParams, PagedResult, QueryParams};
use serde_json::Value;

/// Methods for issuing `circles_query` requests and decoding the tabular response.
///
/// Includes a pager that adds stable ordering (block/tx/log/timestamp) and
/// extracts cursors for streaming.
#[derive(Clone, Debug)]
pub struct QueryMethods {
    client: RpcClient,
}

impl QueryMethods {
    /// Create a new accessor for `circles_query` RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// Direct `circles_query` invocation returning decoded rows.
    pub async fn circles_query<TRow>(&self, params: QueryParams) -> Result<Vec<TRow>>
    where
        TRow: serde::de::DeserializeOwned + Send + Sync + std::fmt::Debug + Unpin + 'static,
    {
        let result: CirclesQueryResponse = self.client.call("circles_query", (params,)).await?;
        self.decode_rows::<TRow>(result.columns, result.rows)
    }

    /// Convenience wrapper for paged queries using the `circles_query` method.
    /// Note: The underlying backend expects `QueryParams`; we translate from the
    /// higher-level `PagedQueryParams` struct.
    pub async fn paged_query<TRow>(&self, params: PagedQueryParams) -> Result<PagedResult<TRow>>
    where
        TRow: serde::de::DeserializeOwned
            + serde::Serialize
            + Clone
            + Send
            + Sync
            + std::fmt::Debug
            + Unpin
            + 'static,
    {
        let PagedQueryParams {
            namespace,
            table,
            sort_order,
            columns,
            filter,
            limit,
        } = params.clone();

        // Convert to QueryParams. If the caller supplied an order, we would add it here; for now
        // we ensure stable ordering via block/tx/log and timestamp.
        let mut order: Vec<OrderBy> = Vec::new();
        let dir = sort_order.clone();
        order.push(OrderBy::new("blockNumber".to_string(), dir.clone()));
        order.push(OrderBy::new("transactionIndex".to_string(), dir.clone()));
        order.push(OrderBy::new("logIndex".to_string(), dir.clone()));
        order.push(OrderBy::new("timestamp".to_string(), dir));

        let query_params = QueryParams {
            namespace,
            table,
            columns,
            filter: filter.unwrap_or_default(),
            order,
            limit: Some(limit),
        };

        let result: CirclesQueryResponse =
            self.client.call("circles_query", (query_params,)).await?;

        let rows = self.decode_rows::<TRow>(result.columns.clone(), result.rows.clone())?;
        let cursors = self.extract_cursors(&result.columns, &result.rows);
        let first_cursor = cursors.first().cloned();
        let last_cursor = cursors.last().cloned();
        let size = rows.len() as u32;
        let has_more = size == limit;

        Ok(PagedResult {
            limit,
            size,
            first_cursor,
            last_cursor,
            sort_order,
            has_more,
            results: rows,
        })
    }

    pub fn decode_rows<TRow>(
        &self,
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    ) -> Result<Vec<TRow>>
    where
        TRow: serde::de::DeserializeOwned,
    {
        rows.into_iter()
            .map(|row| self.decode_row::<TRow>(&columns, row))
            .collect()
    }

    pub fn decode_row<TRow>(&self, columns: &[String], row: Vec<Value>) -> Result<TRow>
    where
        TRow: serde::de::DeserializeOwned,
    {
        if columns.len() != row.len() {
            return Err(CirclesRpcError::InvalidResponse {
                message: "circles_query row length mismatch".to_string(),
            });
        }
        let mut map = serde_json::Map::new();
        for (col, val) in columns.iter().cloned().zip(row.into_iter()) {
            map.insert(col, val);
        }
        serde_json::from_value(Value::Object(map)).map_err(|e| CirclesRpcError::InvalidResponse {
            message: e.to_string(),
        })
    }

    pub fn extract_cursors(
        &self,
        columns: &[String],
        rows: &[Vec<Value>],
    ) -> Vec<circles_types::Cursor> {
        rows.iter()
            .filter_map(|row| self.extract_cursor(columns, row))
            .collect()
    }

    fn extract_cursor(&self, columns: &[String], row: &[Value]) -> Option<circles_types::Cursor> {
        let mut block_number: Option<u64> = None;
        let mut tx_index: Option<u32> = None;
        let mut log_index: Option<u32> = None;
        let mut batch_index: Option<u32> = None;
        let mut timestamp: Option<u64> = None;

        for (col, val) in columns.iter().zip(row.iter()) {
            match col.as_str() {
                "blockNumber" => block_number = Self::as_u64(val),
                "transactionIndex" => tx_index = Self::as_u32(val),
                "logIndex" => log_index = Self::as_u32(val),
                "batchIndex" => batch_index = Self::as_u32(val),
                "timestamp" => timestamp = Self::as_u64(val),
                _ => {}
            }
        }

        match (block_number, tx_index, log_index) {
            (Some(b), Some(tx), Some(log)) => Some(circles_types::Cursor {
                block_number: b,
                transaction_index: tx,
                log_index: log,
                batch_index,
                timestamp,
            }),
            _ => None,
        }
    }

    fn as_u64(val: &Value) -> Option<u64> {
        match val {
            Value::Number(n) => n.as_u64(),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    fn as_u32(val: &Value) -> Option<u32> {
        Self::as_u64(val).map(|v| v as u32)
    }
}
