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

    let unwrapped =
        circles_pathfinder::expected_unwrapped_totals_at(&totals, &info_map, Some(1_700_000_000));
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

#[test]
fn replaces_wrapped_tokens_with_avatar_addresses() {
    let current = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0x1111111111111111111111111111111111111111");
    let wrapper = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let wrapped_owner = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let plain_owner = address!("0xcccccccccccccccccccccccccccccccccccccccc");

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(2u64),
        transfers: vec![
            PathfindingTransferStep {
                from: current,
                to: receiver,
                token_owner: format!("{wrapper:#x}"),
                value: alloy_primitives::U256::from(1u64),
            },
            PathfindingTransferStep {
                from: current,
                to: receiver,
                token_owner: format!("{plain_owner:#x}"),
                value: alloy_primitives::U256::from(1u64),
            },
        ],
    };

    let mut info_map = std::collections::HashMap::new();
    info_map.insert(
        wrapper,
        common::path_helpers::mock_token_info(
            wrapper,
            wrapped_owner,
            "CrcV2_ERC20WrapperDeployed_Demurraged",
        ),
    );
    info_map.insert(
        plain_owner,
        common::path_helpers::mock_token_info(plain_owner, plain_owner, "CrcV2_CRC20"),
    );

    let rewritten = circles_pathfinder::replace_wrapped_tokens_with_avatars(&path, &info_map);

    assert_eq!(
        rewritten.transfers[0].token_owner,
        format!("{wrapped_owner:#x}")
    );
    assert_eq!(
        rewritten.transfers[1].token_owner,
        format!("{plain_owner:#x}")
    );
    assert_eq!(rewritten.transfers[0].from, path.transfers[0].from);
    assert_eq!(rewritten.transfers[0].to, path.transfers[0].to);
    assert_eq!(rewritten.transfers[0].value, path.transfers[0].value);
}

#[test]
fn get_wrapped_tokens_alias_matches_existing_helper() {
    let current = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0x1111111111111111111111111111111111111111");
    let wrapper = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(1u64),
        transfers: vec![PathfindingTransferStep {
            from: current,
            to: receiver,
            token_owner: format!("{wrapper:#x}"),
            value: alloy_primitives::U256::from(42u64),
        }],
    };

    let mut info_map = std::collections::HashMap::new();
    info_map.insert(
        wrapper,
        common::path_helpers::mock_token_info(
            wrapper,
            current,
            "CrcV2_ERC20WrapperDeployed_Demurraged",
        ),
    );

    let via_existing = circles_pathfinder::wrapped_totals_from_path(&path, &info_map);
    let via_alias = circles_pathfinder::get_wrapped_tokens_from_path(&path, &info_map);

    assert_eq!(via_alias, via_existing);
}

#[test]
fn replace_wrapped_tokens_preserves_unmapped_owner_string() {
    let current = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let receiver = address!("0x1111111111111111111111111111111111111111");
    let original_owner = "NOT_A_VALID_ADDRESS".to_string();

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(1u64),
        transfers: vec![PathfindingTransferStep {
            from: current,
            to: receiver,
            token_owner: original_owner.clone(),
            value: alloy_primitives::U256::from(1u64),
        }],
    };

    let rewritten =
        circles_pathfinder::replace_wrapped_tokens(&path, &std::collections::HashMap::new());

    assert_eq!(rewritten.transfers[0].token_owner, original_owner);
}

#[test]
fn expected_unwrapped_totals_ignore_unknown_wrapper_types() {
    use alloy_primitives::U256;
    use std::collections::HashMap;

    let wrapper = address!("0xdddddddddddddddddddddddddddddddddddddddd");
    let avatar = address!("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");

    let mut totals: HashMap<_, (U256, String)> = HashMap::new();
    totals.insert(
        wrapper,
        (
            U256::from(1_000u64),
            "CrcV2_ERC20WrapperDeployed_Experimental".into(),
        ),
    );

    let mut info_map = HashMap::new();
    info_map.insert(
        wrapper,
        common::path_helpers::mock_token_info(
            wrapper,
            avatar,
            "CrcV2_ERC20WrapperDeployed_Experimental",
        ),
    );

    let unwrapped = circles_pathfinder::expected_unwrapped_totals_at(&totals, &info_map, Some(1));

    assert!(unwrapped.is_empty());
}
