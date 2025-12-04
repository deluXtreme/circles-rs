use alloy_primitives::U256;
use alloy_primitives::address;
use circles_types::AdvancedTransferOptions;

#[test]
fn advanced_options_to_find_path_params() {
    let opts = AdvancedTransferOptions {
        use_wrapped_balances: Some(false),
        from_tokens: Some(vec![address!("1000000000000000000000000000000000000001")]),
        to_tokens: Some(vec![address!("2000000000000000000000000000000000000002")]),
        exclude_from_tokens: Some(vec![address!("3000000000000000000000000000000000000003")]),
        exclude_to_tokens: Some(vec![address!("4000000000000000000000000000000000000004")]),
        simulated_balances: None,
        max_transfers: Some(5),
        tx_data: None,
    };
    let params = opts.to_find_path_params(
        address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        U256::from(123u64),
    );
    assert_eq!(params.use_wrapped_balances, Some(false));
    assert_eq!(
        params.from_tokens.unwrap()[0],
        address!("1000000000000000000000000000000000000001")
    );
    assert_eq!(params.max_transfers, Some(5));
    assert_eq!(params.target_flow, U256::from(123u64));
}
