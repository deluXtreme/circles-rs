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

#[test]
fn decode_rich_token_balance_payload() {
    let raw: Vec<TokenBalanceResponse> = serde_json::from_str(
        r#"
        [
          {
            "tokenAddress": "0x1234567890abcdef1234567890abcdef12345678",
            "tokenId": "0x1234567890abcdef1234567890abcdef12345678",
            "tokenOwner": "0xabcdef1234567890abcdef1234567890abcdef12",
            "tokenType": "CrcV2_RegisterHuman",
            "version": 2,
            "attoCircles": "1000000000000000000",
            "circles": 1.0,
            "staticAttoCircles": "1000000000000000000",
            "staticCircles": 1.0,
            "attoCrc": "1000000000000000000",
            "crc": 1.0,
            "isErc20": false,
            "isErc1155": true,
            "isWrapped": false,
            "isInflationary": false,
            "isGroup": false
          }
        ]
        "#,
    )
    .expect("parse rich token balances");

    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].token_address, raw[0].token_id);
    assert_eq!(raw[0].token_type.as_deref(), Some("CrcV2_RegisterHuman"));
    assert_eq!(raw[0].version, Some(2));
    assert!(raw[0].is_erc1155);
    match raw[0].balance {
        Balance::Raw(v) => assert_eq!(
            v,
            alloy_primitives::U256::from(1_000_000_000_000_000_000u128)
        ),
        _ => panic!("expected raw balance"),
    }
}
