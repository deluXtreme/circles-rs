use crate::error::Result;
use circles_types::{
    Cursor, Filter, FilterPredicate, FilterType, PagedQueryParams, PagedResult, SortOrder,
};
use futures::{Stream, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
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

        // Inject cursor filters if a cursor is present.
        if let Some(cursor) = &self.current_cursor {
            let cmp = match params.sort_order {
                SortOrder::ASC => FilterType::GreaterOrEqualThan,
                SortOrder::DESC => FilterType::LessOrEqualThan,
            };
            let mut filters: Vec<Filter> = params.filter.unwrap_or_default();
            filters.push(
                FilterPredicate::new(cmp.clone(), "blockNumber".to_string(), cursor.block_number)
                    .into(),
            );
            filters.push(
                FilterPredicate::new(
                    cmp.clone(),
                    "transactionIndex".to_string(),
                    cursor.transaction_index,
                )
                .into(),
            );
            filters.push(
                FilterPredicate::new(cmp.clone(), "logIndex".to_string(), cursor.log_index).into(),
            );
            if let Some(batch) = cursor.batch_index {
                filters.push(FilterPredicate::new(cmp, "batchIndex".to_string(), batch).into());
            }
            params.filter = Some(filters);
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
