use circles_rpc::RpcClient;
use circles_rpc::methods::QueryMethods;
use circles_types::{CursorColumn, OrderBy, PagedQueryParams, SortOrder};
use serde_json::{Value, json};

// Helper: construct a QueryMethods with a dummy RpcClient; we only use its decode helpers.
fn query_methods() -> QueryMethods {
    QueryMethods::new(RpcClient::http("http://localhost".parse().unwrap()))
}

#[test]
fn decode_circles_query_rows() {
    let raw: Value =
        serde_json::from_str(include_str!("fixtures/circles_query_response.json")).unwrap();
    let columns: Vec<String> = raw["Columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let rows: Vec<Vec<Value>> = raw["Rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_array().unwrap().to_vec())
        .collect();

    let methods = query_methods();
    let decoded: Vec<Value> = methods.decode_rows(columns.clone(), rows.clone()).unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(
        decoded[0]["avatar"],
        json!("0xde374ece6fa50e781e81aac78e811b33d16912c7")
    );

    // Ensure cursors extract correctly.
    let cursor_columns = PagedQueryParams::new(
        "V_Crc".to_string(),
        "TransferSummary".to_string(),
        SortOrder::DESC,
        columns.clone(),
        rows.len() as u32,
    )
    .resolved_cursor_columns();
    let cursors = methods.extract_cursors(&columns, &rows, &cursor_columns);
    assert_eq!(cursors.len(), 2);
    assert_eq!(cursors[0].block_number, 30000000);
    assert_eq!(cursors[0].transaction_index, 1);
    assert_eq!(cursors[0].log_index, 0);
    assert_eq!(cursors[0].timestamp, Some(1710000000));
}

#[test]
fn paged_query_has_more_logic() {
    // Simulate rows equal to limit to check cursor extraction; has_more calculation is implicit when size == limit.
    let methods = query_methods();
    let columns = vec![
        "blockNumber".to_string(),
        "transactionIndex".to_string(),
        "logIndex".to_string(),
        "timestamp".to_string(),
        "avatar".to_string(),
    ];
    let rows = vec![
        vec![json!(10), json!(0), json!(0), json!(1700), json!("0x1")],
        vec![json!(9), json!(1), json!(0), json!(1699), json!("0x2")],
    ];
    let cursor_columns = PagedQueryParams::new(
        "V_Crc".to_string(),
        "Avatars".to_string(),
        SortOrder::DESC,
        columns.clone(),
        rows.len() as u32,
    )
    .resolved_cursor_columns();
    let cursors = methods.extract_cursors(&columns, &rows, &cursor_columns);
    assert_eq!(cursors.len(), 2);
    assert_eq!(cursors[1].timestamp, Some(1699));

    let params = PagedQueryParams {
        namespace: "V_Crc".to_string(),
        table: "Avatars".to_string(),
        sort_order: SortOrder::DESC,
        columns: vec!["avatar".to_string()],
        filter: None,
        cursor_columns: None,
        order_columns: None,
        limit: 2,
    };

    // Build a fake CirclesQueryResponse-like payload and reuse decode_rows to simulate paged_query output.
    let decoded: Vec<Value> = methods.decode_rows(columns.clone(), rows.clone()).unwrap();
    assert_eq!(decoded.len(), params.limit as usize);
}

#[test]
fn extract_cursor_with_batch_index() {
    let raw: Value =
        serde_json::from_str(include_str!("fixtures/circles_query_batch.json")).unwrap();
    let columns: Vec<String> = raw["Columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let rows: Vec<Vec<Value>> = raw["Rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_array().unwrap().to_vec())
        .collect();

    let methods = query_methods();
    let cursor_columns = PagedQueryParams::new(
        "V_Crc".to_string(),
        "TransferBatch".to_string(),
        SortOrder::DESC,
        columns.clone(),
        rows.len() as u32,
    )
    .resolved_cursor_columns();
    let cursors = methods.extract_cursors(&columns, &rows, &cursor_columns);
    assert_eq!(cursors.len(), 1);
    let cursor = &cursors[0];
    assert_eq!(cursor.block_number, 40000000);
    assert_eq!(cursor.transaction_index, 2);
    assert_eq!(cursor.log_index, 7);
    assert_eq!(cursor.batch_index, Some(3));
    assert_eq!(cursor.timestamp, Some(1715000000));
}

#[test]
fn extract_cursor_with_custom_cursor_columns() {
    let columns = vec![
        "group".to_string(),
        "holder".to_string(),
        "totalBalance".to_string(),
        "demurragedTotalBalance".to_string(),
        "fractionOwnership".to_string(),
    ];
    let rows = vec![vec![
        json!("0x1111111111111111111111111111111111111111"),
        json!("0x2222222222222222222222222222222222222222"),
        json!("100"),
        json!("100"),
        json!(0.5),
    ]];

    let methods = query_methods();
    let cursor_columns = vec![CursorColumn::asc("holder".into())];
    let cursors = methods.extract_cursors(&columns, &rows, &cursor_columns);
    assert_eq!(cursors.len(), 1);
    assert_eq!(
        cursors[0].value("holder"),
        Some(&json!("0x2222222222222222222222222222222222222222"))
    );

    let params = PagedQueryParams::new(
        "V_CrcV2".into(),
        "GroupTokenHoldersBalance".into(),
        SortOrder::DESC,
        columns,
        50,
    )
    .with_cursor_columns(cursor_columns.clone())
    .with_order_columns(vec![
        OrderBy::desc("totalBalance".into()),
        OrderBy::asc("holder".into()),
    ]);
    assert_eq!(params.resolved_cursor_columns(), cursor_columns);
}
