use circles_types::TrustRelationType;

#[test]
fn decode_common_trust() {
    let rels: Vec<TrustRelationType> =
        serde_json::from_str(include_str!("fixtures/common_trust.json"))
            .expect("parse common trust");
    assert_eq!(
        rels,
        vec![TrustRelationType::Trusts, TrustRelationType::MutuallyTrusts]
    );
}
