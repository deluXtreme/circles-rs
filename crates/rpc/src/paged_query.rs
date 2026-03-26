use crate::error::Result;
use circles_types::{
    Conjunction, Cursor, CursorColumn, Filter, FilterPredicate, FilterType, PagedQueryParams,
    PagedResult, SortOrder,
};
use futures::{Stream, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Represents a page of results returned by [`PagedQuery`].
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// Items contained in this page.
    pub items: Vec<T>,
    /// Cursor pointing to the first row.
    pub first_cursor: Option<Cursor>,
    /// Cursor pointing to the last row.
    pub last_cursor: Option<Cursor>,
    /// Whether more results are available after this page.
    pub has_more: bool,
}

pub type PagedFetch<TRow> = Arc<
    dyn Fn(PagedQueryParams) -> Pin<Box<dyn Future<Output = Result<PagedResult<TRow>>> + Send>>
        + Send
        + Sync,
>;

/// Generic paginator that wraps the `circles_query` RPC method.
/// The fetcher function is responsible for honoring `current_cursor` if desired.
// TODO: table-aware default ordering + helper to apply cursor to params.
pub struct PagedQuery<TRow: Clone + Serialize> {
    fetch: PagedFetch<TRow>,
    /// Base params to reuse across calls.
    pub params: PagedQueryParams,
    /// Last cursor that was seen (advanced after each page).
    pub current_cursor: Option<Cursor>,
}

impl<TRow> PagedQuery<TRow>
where
    TRow: Clone + Serialize + DeserializeOwned + Send + 'static,
{
    pub fn new(fetch: PagedFetch<TRow>, params: PagedQueryParams) -> Self {
        Self {
            fetch,
            params,
            current_cursor: None,
        }
    }

    /// Fetch the next page. Consumers can track `current_cursor` to drive cursor-based filters.
    pub async fn next_page(&mut self) -> Result<Option<Page<TRow>>> {
        let mut params = self.params.clone();

        if let Some(cursor) = &self.current_cursor {
            let cursor_filter = build_cursor_filter(cursor, &params.resolved_cursor_columns());
            params.filter = combine_filters(params.filter.take(), cursor_filter);
        }

        let result = (self.fetch)(params).await?;

        if result.results.is_empty() {
            return Ok(None);
        }

        // Advance cursor to the last item returned.
        self.current_cursor = result.last_cursor.clone();

        Ok(Some(Page {
            items: result.results,
            first_cursor: result.first_cursor,
            last_cursor: result.last_cursor,
            has_more: result.has_more,
        }))
    }

    /// Convert this paginator into a stream of rows.
    pub fn into_stream(self) -> impl Stream<Item = Result<TRow>> {
        futures::stream::unfold(self, |mut state| async move {
            match state.next_page().await {
                Ok(Some(page)) => {
                    let has_more = page.has_more;
                    let items = page.items;
                    if has_more {
                        Some((Ok(items), state))
                    } else {
                        None
                    }
                }
                Ok(None) => None,
                Err(e) => Some((Err(e), state)),
            }
        })
        .flat_map(|res| match res {
            Ok(vec) => futures::stream::iter(vec.into_iter().map(Ok)).boxed(),
            Err(err) => futures::stream::iter(vec![Err(err)]).boxed(),
        })
    }
}

fn build_cursor_filter(cursor: &Cursor, cursor_columns: &[CursorColumn]) -> Vec<Filter> {
    let mut or_predicates = Vec::new();

    for level in 0..cursor_columns.len() {
        let current_column = &cursor_columns[level];
        let Some(cursor_value) = cursor_column_value(cursor, &current_column.name) else {
            continue;
        };

        if level == 0 {
            or_predicates.push(comparison_predicate(current_column, cursor_value));
            continue;
        }

        let mut and_predicates = Vec::new();
        for previous_column in cursor_columns.iter().take(level) {
            let Some(previous_value) = cursor_column_value(cursor, &previous_column.name) else {
                continue;
            };
            and_predicates
                .push(FilterPredicate::equals(previous_column.name.clone(), previous_value).into());
        }
        and_predicates.push(comparison_predicate(current_column, cursor_value));
        or_predicates.push(Conjunction::and(and_predicates).into());
    }

    if or_predicates.is_empty() {
        Vec::new()
    } else {
        vec![Conjunction::or(or_predicates).into()]
    }
}

fn combine_filters(
    base_filters: Option<Vec<Filter>>,
    cursor_filter: Vec<Filter>,
) -> Option<Vec<Filter>> {
    match (base_filters, cursor_filter.is_empty()) {
        (None, true) => None,
        (Some(filters), true) => Some(filters),
        (None, false) => Some(cursor_filter),
        (Some(base_filters), false) => {
            let mut predicates = base_filters;
            predicates.extend(cursor_filter);
            Some(vec![Conjunction::and(predicates).into()])
        }
    }
}

fn comparison_predicate(column: &CursorColumn, value: Value) -> Filter {
    let filter_type = match column.sort_order {
        SortOrder::ASC => FilterType::GreaterThan,
        SortOrder::DESC => FilterType::LessThan,
    };
    FilterPredicate::new(filter_type, column.name.clone(), value).into()
}

fn cursor_column_value(cursor: &Cursor, column: &str) -> Option<Value> {
    match column {
        "blockNumber" => Some(
            cursor
                .value(column)
                .cloned()
                .unwrap_or_else(|| Value::from(cursor.block_number)),
        ),
        "transactionIndex" => Some(
            cursor
                .value(column)
                .cloned()
                .unwrap_or_else(|| Value::from(cursor.transaction_index)),
        ),
        "logIndex" => Some(
            cursor
                .value(column)
                .cloned()
                .unwrap_or_else(|| Value::from(cursor.log_index)),
        ),
        "batchIndex" => cursor
            .value(column)
            .cloned()
            .or_else(|| cursor.batch_index.map(Value::from)),
        "timestamp" => cursor
            .value(column)
            .cloned()
            .or_else(|| cursor.timestamp.map(Value::from)),
        _ => cursor.value(column).cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct HolderRow {
        group: String,
        holder: String,
        #[serde(rename = "totalBalance")]
        total_balance: String,
        #[serde(rename = "demurragedTotalBalance")]
        demurraged_total_balance: String,
        #[serde(rename = "fractionOwnership")]
        fraction_ownership: f64,
    }

    #[tokio::test]
    async fn next_page_builds_ts_style_custom_cursor_filter() {
        let seen_params = Arc::new(Mutex::new(Vec::<PagedQueryParams>::new()));
        let seen_params_fetch = Arc::clone(&seen_params);
        let fetch: PagedFetch<HolderRow> = Arc::new(move |params: PagedQueryParams| {
            let seen_params = Arc::clone(&seen_params_fetch);
            Box::pin(async move {
                let call_index = {
                    let mut guard = seen_params.lock().expect("lock params");
                    guard.push(params.clone());
                    guard.len()
                };

                if call_index == 1 {
                    let mut cursor = Cursor::default();
                    cursor.insert_value(
                        "holder".to_string(),
                        json!("0x2222222222222222222222222222222222222222"),
                    );

                    Ok(PagedResult {
                        limit: params.limit,
                        size: 1,
                        first_cursor: Some(cursor.clone()),
                        last_cursor: Some(cursor),
                        sort_order: params.sort_order,
                        has_more: true,
                        results: vec![HolderRow {
                            group: "0x1111111111111111111111111111111111111111".into(),
                            holder: "0x2222222222222222222222222222222222222222".into(),
                            total_balance: "100".into(),
                            demurraged_total_balance: "100".into(),
                            fraction_ownership: 0.5,
                        }],
                    })
                } else {
                    Ok(PagedResult {
                        limit: params.limit,
                        size: 0,
                        first_cursor: None,
                        last_cursor: None,
                        sort_order: params.sort_order,
                        has_more: false,
                        results: Vec::new(),
                    })
                }
            })
        });

        let mut query = PagedQuery::new(
            fetch,
            PagedQueryParams {
                namespace: "V_CrcV2".into(),
                table: "GroupTokenHoldersBalance".into(),
                sort_order: SortOrder::DESC,
                columns: vec![
                    "group".into(),
                    "holder".into(),
                    "totalBalance".into(),
                    "demurragedTotalBalance".into(),
                    "fractionOwnership".into(),
                ],
                filter: Some(vec![
                    FilterPredicate::equals(
                        "group".into(),
                        "0x1111111111111111111111111111111111111111",
                    )
                    .into(),
                ]),
                cursor_columns: Some(vec![CursorColumn::asc("holder".into())]),
                order_columns: Some(vec![
                    circles_types::OrderBy::desc("totalBalance".into()),
                    circles_types::OrderBy::asc("holder".into()),
                ]),
                limit: 50,
            },
        );

        assert!(query.next_page().await.expect("first page").is_some());
        assert!(query.next_page().await.expect("second page").is_none());

        let recorded = seen_params.lock().expect("lock params");
        assert_eq!(recorded.len(), 2);
        let second_filter = serde_json::to_value(recorded[1].filter.clone().expect("filter"))
            .expect("serialize filter");
        assert_eq!(second_filter[0]["Type"], json!("Conjunction"));
        assert_eq!(second_filter[0]["ConjunctionType"], json!("And"));
        assert_eq!(
            second_filter[0]["Predicates"][1]["ConjunctionType"],
            json!("Or")
        );
        assert_eq!(
            second_filter[0]["Predicates"][1]["Predicates"][0]["Column"],
            json!("holder")
        );
        assert_eq!(
            second_filter[0]["Predicates"][1]["Predicates"][0]["FilterType"],
            json!("GreaterThan")
        );
    }
}
