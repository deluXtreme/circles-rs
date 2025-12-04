use serde::{Deserialize, Serialize};

/// Filter types for query predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqualThan,
    LessOrEqualThan,
    Like,
}

/// Conjunction types for combining predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConjunctionType {
    And,
    Or,
}

/// Filter predicate for querying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPredicate {
    #[serde(rename = "Type")]
    pub predicate_type: String, // Always "FilterPredicate"
    #[serde(rename = "FilterType")]
    pub filter_type: FilterType,
    #[serde(rename = "Column")]
    pub column: String,
    #[serde(rename = "Value")]
    pub value: serde_json::Value, // Can be string, number, or boolean
}

impl FilterPredicate {
    pub fn new(
        filter_type: FilterType,
        column: String,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            predicate_type: "FilterPredicate".to_string(),
            filter_type,
            column,
            value: value.into(),
        }
    }

    pub fn equals(column: String, value: impl Into<serde_json::Value>) -> Self {
        Self::new(FilterType::Equals, column, value)
    }

    pub fn not_equals(column: String, value: impl Into<serde_json::Value>) -> Self {
        Self::new(FilterType::NotEquals, column, value)
    }

    pub fn greater_than(column: String, value: impl Into<serde_json::Value>) -> Self {
        Self::new(FilterType::GreaterThan, column, value)
    }

    pub fn less_than(column: String, value: impl Into<serde_json::Value>) -> Self {
        Self::new(FilterType::LessThan, column, value)
    }

    pub fn like(column: String, pattern: String) -> Self {
        Self::new(FilterType::Like, column, pattern)
    }
}

/// Conjunction for combining multiple predicates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conjunction {
    #[serde(rename = "Type")]
    pub conjunction_type_name: String, // Always "Conjunction"
    #[serde(rename = "ConjunctionType")]
    pub conjunction_type: ConjunctionType,
    #[serde(rename = "Predicates")]
    pub predicates: Vec<Filter>,
}

impl Conjunction {
    pub fn new(conjunction_type: ConjunctionType, predicates: Vec<Filter>) -> Self {
        Self {
            conjunction_type_name: "Conjunction".to_string(),
            conjunction_type,
            predicates,
        }
    }

    pub fn and(predicates: Vec<Filter>) -> Self {
        Self::new(ConjunctionType::And, predicates)
    }

    pub fn or(predicates: Vec<Filter>) -> Self {
        Self::new(ConjunctionType::Or, predicates)
    }
}

/// Filter type (either a predicate or conjunction).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Predicate(FilterPredicate),
    Conjunction(Conjunction),
}

impl From<FilterPredicate> for Filter {
    fn from(predicate: FilterPredicate) -> Self {
        Filter::Predicate(predicate)
    }
}

impl From<Conjunction> for Filter {
    fn from(conjunction: Conjunction) -> Self {
        Filter::Conjunction(conjunction)
    }
}

/// Order direction for query results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    ASC,
    DESC,
}

/// Order by clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    #[serde(rename = "Column")]
    pub column: String,
    #[serde(rename = "SortOrder")]
    pub sort_order: SortOrder,
}

impl OrderBy {
    pub fn new(column: String, sort_order: SortOrder) -> Self {
        Self { column, sort_order }
    }

    pub fn asc(column: String) -> Self {
        Self::new(column, SortOrder::ASC)
    }

    pub fn desc(column: String) -> Self {
        Self::new(column, SortOrder::DESC)
    }
}

/// Query parameters for `circles_query`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    #[serde(rename = "Namespace")]
    pub namespace: String,
    #[serde(rename = "Table")]
    pub table: String,
    #[serde(rename = "Columns")]
    pub columns: Vec<String>,
    #[serde(rename = "Filter")]
    pub filter: Vec<Filter>,
    #[serde(rename = "Order")]
    pub order: Vec<OrderBy>,
    #[serde(rename = "Limit", skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl QueryParams {
    pub fn new(namespace: String, table: String, columns: Vec<String>) -> Self {
        Self {
            namespace,
            table,
            columns,
            filter: Vec::new(),
            order: Vec::new(),
            limit: None,
        }
    }

    pub fn with_filter(mut self, filter: Vec<Filter>) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_order(mut self, order: Vec<OrderBy>) -> Self {
        self.order = order;
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Column information for table metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub column_type: String,
}

/// Table information from circles_tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    #[serde(rename = "Namespace")]
    pub namespace: String,
    #[serde(rename = "Table")]
    pub table: String,
    #[serde(rename = "Columns")]
    pub columns: Vec<ColumnInfo>,
}

/// Defines the minimum columns any event row must have for cursor-based pagination.
/// These values are important for determining cursor position in result sets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub block_number: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub batch_index: Option<u32>,
    pub timestamp: Option<u64>,
}

/// A cursor is a sortable unique identifier for a specific log entry.
/// Used to paginate through query results efficiently.
pub type Cursor = EventRow;

/// Result of a paginated query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<TRow>
where
    TRow: Clone + Serialize,
{
    /// The number of results that were requested
    pub limit: u32,
    /// The number of results that were returned
    pub size: u32,
    /// If the query returned results, this will be the cursor for the first result
    pub first_cursor: Option<Cursor>,
    /// If the query returned results, this will be the cursor for the last result
    pub last_cursor: Option<Cursor>,
    /// The sort order of the results
    pub sort_order: SortOrder,
    /// Whether there are more results available
    pub has_more: bool,
    /// The results of the query
    pub results: Vec<TRow>,
}

impl<TRow> PagedResult<TRow>
where
    TRow: Clone + Serialize,
{
    pub fn new(
        limit: u32,
        results: Vec<TRow>,
        sort_order: SortOrder,
        has_more: bool,
        first_cursor: Option<Cursor>,
        last_cursor: Option<Cursor>,
    ) -> Self {
        let size = results.len() as u32;
        Self {
            limit,
            size,
            first_cursor,
            last_cursor,
            sort_order,
            has_more,
            results,
        }
    }
}

/// Parameters for a paginated query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedQueryParams {
    /// The namespace of the table to query
    pub namespace: String,
    /// The name of the table to query
    pub table: String,
    /// The order to sort the results
    pub sort_order: SortOrder,
    /// The columns to return in the results
    pub columns: Vec<String>,
    /// The filters to apply to the query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<Filter>>,
    /// The number of results to return per page
    pub limit: u32,
}

impl PagedQueryParams {
    pub fn new(
        namespace: String,
        table: String,
        sort_order: SortOrder,
        columns: Vec<String>,
        limit: u32,
    ) -> Self {
        Self {
            namespace,
            table,
            sort_order,
            columns,
            filter: None,
            limit,
        }
    }

    pub fn with_filter(mut self, filter: Vec<Filter>) -> Self {
        self.filter = Some(filter);
        self
    }
}
