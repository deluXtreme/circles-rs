use crate::client::RpcClient;
use crate::error::{CirclesRpcError, Result};
use circles_types::{
    CirclesQueryResponse, Cursor, CursorColumn, OrderBy, PagedQueryParams, PagedResult, QueryParams,
};
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
            cursor_columns,
            order_columns,
            limit,
        } = params.clone();

        let order: Vec<OrderBy> = if let Some(order_columns) = order_columns.clone() {
            if order_columns.is_empty() {
                params.resolved_order_columns()
            } else {
                order_columns
            }
        } else {
            params.resolved_order_columns()
        };
        let cursor_columns = if let Some(cursor_columns) = cursor_columns {
            if cursor_columns.is_empty() {
                params.resolved_cursor_columns()
            } else {
                cursor_columns
            }
        } else {
            params.resolved_cursor_columns()
        };

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
        let cursors = self.extract_cursors(&result.columns, &result.rows, &cursor_columns);
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
        cursor_columns: &[CursorColumn],
    ) -> Vec<Cursor> {
        rows.iter()
            .filter_map(|row| self.extract_cursor(columns, row, cursor_columns))
            .collect()
    }

    fn extract_cursor(
        &self,
        columns: &[String],
        row: &[Value],
        cursor_columns: &[CursorColumn],
    ) -> Option<Cursor> {
        let mut block_number: Option<u64> = None;
        let mut tx_index: Option<u32> = None;
        let mut log_index: Option<u32> = None;
        let mut batch_index: Option<u32> = None;
        let mut timestamp: Option<u64> = None;
        let mut cursor = Cursor::default();
        let mut has_cursor_values = false;

        for (col, val) in columns.iter().zip(row.iter()) {
            match col.as_str() {
                "blockNumber" => block_number = Self::as_u64(val),
                "transactionIndex" => tx_index = Self::as_u32(val),
                "logIndex" => log_index = Self::as_u32(val),
                "batchIndex" => batch_index = Self::as_u32(val),
                "timestamp" => timestamp = Self::as_u64(val),
                _ => {}
            }

            if cursor_columns.iter().any(|column| column.name == *col) {
                cursor.insert_value(col.clone(), val.clone());
                has_cursor_values = true;
            }
        }

        if let (Some(b), Some(tx), Some(log)) = (block_number, tx_index, log_index) {
            cursor.block_number = b;
            cursor.transaction_index = tx;
            cursor.log_index = log;
            cursor.batch_index = batch_index;
            cursor.timestamp = timestamp;
            return Some(cursor);
        }

        if has_cursor_values {
            return Some(cursor);
        }

        None
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
