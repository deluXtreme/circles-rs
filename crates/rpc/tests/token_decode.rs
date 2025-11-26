use circles_types::{Balance, TokenBalanceResponse};

#[test]
fn decode_token_balances() {
    let raw: Vec<TokenBalanceResponse> =
        serde_json::from_str(include_str!("fixtures/token_balances.json"))
            .expect("parse token balances");
    assert_eq!(raw.len(), 2);
    match raw[0].balance {
        Balance::Raw(v) => assert_eq!(v, alloy_primitives::U256::from(0x1234u64)),
        _ => panic!("expected raw balance"),
    }
    match raw[1].balance {
        Balance::Raw(v) => assert_eq!(v, alloy_primitives::U256::from(100000u64)),
        _ => panic!("expected raw balance"),
    }
}
