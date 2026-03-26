use crate::client::RpcClient;
use crate::error::{CirclesRpcError, Result};
use crate::methods::QueryMethods;
use crate::paged_query::{PagedFetch, PagedQuery};
use circles_types::{
    Address, Conjunction, FilterPredicate, PagedQueryParams, PagedResult, SortOrder,
    TransactionHistoryRow, U256,
};
use circles_utils::converter::{
    atto_circles_to_atto_crc, atto_circles_to_atto_static_circles, atto_circles_to_circles,
};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTransactionHistoryRow {
    block_number: u64,
    timestamp: u64,
    transaction_index: u32,
    log_index: u32,
    transaction_hash: alloy_primitives::TxHash,
    version: u32,
    from: Address,
    to: Address,
    id: String,
    token_address: Address,
    value: String,
}

fn enrich_transaction_row(row: RawTransactionHistoryRow) -> Result<TransactionHistoryRow> {
    let atto_circles =
        U256::from_str(&row.value).map_err(|e| CirclesRpcError::InvalidResponse {
            message: format!("invalid transfer summary value: {e}"),
        })?;
    let circles = atto_circles_to_circles(atto_circles);
    let atto_crc = atto_circles_to_atto_crc(atto_circles, row.timestamp);
    let crc = atto_circles_to_circles(atto_crc);
    let static_atto_circles =
        atto_circles_to_atto_static_circles(atto_circles, Some(row.timestamp));
    let static_circles = atto_circles_to_circles(static_atto_circles);

    Ok(TransactionHistoryRow {
        block_number: row.block_number,
        timestamp: row.timestamp,
        transaction_index: row.transaction_index,
        log_index: row.log_index,
        transaction_hash: row.transaction_hash,
        version: row.version,
        from: row.from,
        to: row.to,
        id: row.id,
        token_address: row.token_address,
        value: row.value,
        circles: Some(circles),
        atto_circles: Some(atto_circles),
        static_circles: Some(static_circles),
        static_atto_circles: Some(static_atto_circles),
        crc: Some(crc),
        atto_crc: Some(atto_crc),
    })
}

/// Transaction history methods mirroring the TS RPC helper.
#[derive(Clone, Debug)]
pub struct TransactionMethods {
    client: RpcClient,
}

impl TransactionMethods {
    /// Create a new accessor for transaction-history RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// Paged transaction history from the `V_Crc.TransferSummary` view.
    pub fn get_transaction_history(
        &self,
        avatar: Address,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<TransactionHistoryRow> {
        let params = PagedQueryParams {
            namespace: "V_Crc".into(),
            table: "TransferSummary".into(),
            sort_order,
            columns: vec![
                "blockNumber".into(),
                "timestamp".into(),
                "transactionIndex".into(),
                "logIndex".into(),
                "transactionHash".into(),
                "version".into(),
                "from".into(),
                "to".into(),
                "id".into(),
                "tokenAddress".into(),
                "value".into(),
            ],
            filter: Some(vec![
                Conjunction::and(vec![
                    FilterPredicate::equals("version".into(), 2).into(),
                    Conjunction::or(vec![
                        FilterPredicate::equals("from".into(), format!("{avatar:#x}")).into(),
                        FilterPredicate::equals("to".into(), format!("{avatar:#x}")).into(),
                    ])
                    .into(),
                ])
                .into(),
            ]),
            limit,
        };

        let client = self.client.clone();
        let fetch: PagedFetch<TransactionHistoryRow> = Arc::new(move |params: PagedQueryParams| {
            let client = client.clone();
            Box::pin(async move {
                let raw = QueryMethods::new(client)
                    .paged_query::<RawTransactionHistoryRow>(params)
                    .await?;
                let rows = raw
                    .results
                    .into_iter()
                    .map(enrich_transaction_row)
                    .collect::<Result<Vec<_>>>()?;
                Ok(PagedResult {
                    limit: raw.limit,
                    size: rows.len() as u32,
                    first_cursor: raw.first_cursor,
                    last_cursor: raw.last_cursor,
                    sort_order: raw.sort_order,
                    has_more: raw.has_more,
                    results: rows,
                })
            })
                as Pin<
                    Box<
                        dyn std::future::Future<Output = Result<PagedResult<TransactionHistoryRow>>>
                            + Send,
                    >,
                >
        });

        PagedQuery::new(fetch, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods() -> TransactionMethods {
        let url = "https://rpc.example.com".parse().expect("valid url");
        TransactionMethods::new(RpcClient::http(url))
    }

    #[test]
    fn transaction_history_query_uses_transfer_summary_view() {
        let query =
            methods().get_transaction_history(Address::repeat_byte(0x11), 50, SortOrder::DESC);

        assert_eq!(query.params.namespace, "V_Crc");
        assert_eq!(query.params.table, "TransferSummary");
        assert_eq!(query.params.limit, 50);
    }

    #[test]
    fn enrich_transaction_row_populates_amount_fields() {
        let row = enrich_transaction_row(RawTransactionHistoryRow {
            block_number: 1,
            timestamp: 1_700_000_000,
            transaction_index: 2,
            log_index: 3,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            from: Address::repeat_byte(0x11),
            to: Address::repeat_byte(0x22),
            id: "1".into(),
            token_address: Address::repeat_byte(0x33),
            value: "1000000000000000000".into(),
        })
        .expect("enrich");

        assert_eq!(
            row.atto_circles,
            Some(U256::from(1_000_000_000_000_000_000u64))
        );
        assert!(row.circles.expect("circles") > 0.0);
        assert!(row.static_circles.expect("static circles") > 0.0);
        assert!(row.crc.expect("crc") > 0.0);
        assert!(row.static_atto_circles.expect("static atto") > U256::ZERO);
        assert!(row.atto_crc.expect("atto crc") > U256::ZERO);
    }
}
