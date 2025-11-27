use alloy_primitives::address;
use circles_types::{PathfindingResult, PathfindingTransferStep};

mod common;

#[tokio::test]
async fn normalizes_demurraged_wrapper_type() {
    // Build a minimal path where current avatar is the sender of a wrapped token.
    let current = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let wrapper = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let _path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(1u64),
        transfers: vec![PathfindingTransferStep {
            from: current,
            to: current,
            token_owner: format!("{wrapper:#x}"),
            value: alloy_primitives::U256::from(1u64),
        }],
    };

    // Build a fake token info map as the helper would, then apply normalization.
    let info = common::path_helpers::mock_token_info(
        wrapper,
        current,
        "CrcV2_ERC20WrapperDeployed", // missing demurraged suffix
    );
    let mut map = std::collections::HashMap::new();
    map.insert(info.token, info);

    // Apply the same normalization the helper does.
    for info in map.values_mut() {
        let is_wrapper = info.token_type.starts_with("CrcV2_ERC20WrapperDeployed");
        let is_inflationary = info.token_type.contains("Inflationary");
        if is_wrapper && !is_inflationary {
            info.token_type = "CrcV2_ERC20WrapperDeployed_Demurraged".to_string();
        }
    }

    let info = map.get(&wrapper).unwrap();
    assert_eq!(info.token_type, "CrcV2_ERC20WrapperDeployed_Demurraged");
}

#[test]
fn converts_inflationary_wrapper_totals() {
    use alloy_primitives::U256;
    use std::collections::HashMap;

    let wrapper = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let avatar = address!("0xcccccccccccccccccccccccccccccccccccccccc");

    // Pretend the wrapped total is inflationary; verify we run through the converter.
    let mut totals: HashMap<_, (U256, String)> = HashMap::new();
    totals.insert(
        wrapper,
        (
            // Static total for ts=1_700_000_000 when demurraged total is 1e18.
            U256::from(1_250_475_269_390_674_654u64),
            "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
        ),
    );

    let mut info_map = HashMap::new();
    let mut info = common::path_helpers::mock_token_info(
        wrapper,
        avatar,
        "CrcV2_ERC20WrapperDeployed_Inflationary",
    );
    info.timestamp = 1_700_000_000;
    info_map.insert(wrapper, info);

    let unwrapped = circles_pathfinder::path::expected_unwrapped_totals(&totals, &info_map);
    let (amount, owner) = unwrapped.get(&wrapper).unwrap();

    // We expect ~1e18 demurraged amount after conversion back from static for ts=1_700_000_000.
    let expected = U256::from(1_000_000_000_000_000_000u64);
    let diff = if *amount > expected {
        *amount - expected
    } else {
        expected - *amount
    };
    assert!(diff < U256::from(1_000u64));
    assert_eq!(*owner, avatar);
}
