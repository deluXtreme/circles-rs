use circles_rpc::methods::token::TokenHolderNormalized;
use circles_types::TokenHolder;

#[test]
fn decode_and_normalize_token_holders() {
    let raw: Vec<TokenHolder> = serde_json::from_str(include_str!("fixtures/token_holders.json"))
        .expect("parse token holders");
    assert_eq!(raw.len(), 2);

    let normalized: Vec<TokenHolderNormalized> =
        raw.into_iter().map(TokenHolderNormalized::from).collect();

    assert_eq!(
        normalized[0].balance,
        alloy_primitives::U256::from_str_radix("12345678901234567890", 10).unwrap()
    );
    assert_eq!(normalized[1].balance, alloy_primitives::U256::from(0));
}
