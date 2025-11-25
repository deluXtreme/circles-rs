use circles_types::TrustRelation;

#[test]
fn decode_trust_relations() {
    let rows: Vec<TrustRelation> =
        serde_json::from_str(include_str!("fixtures/trust_relations.json"))
            .expect("parse trust relations");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].truster,
        "0xde374ece6fa50e781e81aac78e811b33d16912c7"
            .parse::<alloy_primitives::Address>()
            .unwrap()
    );
    assert_eq!(
        rows[0].trustee,
        "0xfeed00000000000000000000000000000000beef"
            .parse::<alloy_primitives::Address>()
            .unwrap()
    );
    assert_eq!(rows[0].expiry_time, 1800000000);
}
