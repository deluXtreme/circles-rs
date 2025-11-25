use circles_rpc::events::parser::parse;
use circles_types::{CirclesEvent, RpcSubscriptionEvent};

#[test]
fn decode_http_circles_events() {
    let raw: Vec<RpcSubscriptionEvent> =
        serde_json::from_str(include_str!("fixtures/circles_events_http.json"))
            .expect("parse circles_events payload");
    assert_eq!(raw.len(), 2);

    let parsed: Vec<CirclesEvent> = raw.into_iter().map(|e| parse(e).unwrap()).collect();
    assert_eq!(
        parsed[0].event_type,
        circles_types::CirclesEventType::CrcV2RegisterHuman
    );
    assert_eq!(parsed[0].base.block_number, 30000000);
    assert_eq!(
        parsed[1].event_type,
        circles_types::CirclesEventType::CrcV2Trust
    );
    assert_eq!(parsed[1].base.log_index, 5);
}
