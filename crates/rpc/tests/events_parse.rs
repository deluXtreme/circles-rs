use circles_rpc::events::parser::parse;
use circles_types::{CirclesEvent, RpcSubscriptionEvent};

#[test]
fn parse_rpc_subscription_event_into_circles_event() {
    let raw: RpcSubscriptionEvent =
        serde_json::from_str(include_str!("fixtures/circles_events.json")).unwrap();
    let parsed: CirclesEvent = parse(raw).expect("parse event");
    assert_eq!(format!("{:?}", parsed.event_type), "CrcV2RegisterHuman");
    assert_eq!(parsed.base.block_number, 30000000);
    assert_eq!(parsed.base.transaction_index, 1);
    assert_eq!(parsed.base.log_index, 0);
    assert_eq!(parsed.base.timestamp, Some(1710000000));
}
