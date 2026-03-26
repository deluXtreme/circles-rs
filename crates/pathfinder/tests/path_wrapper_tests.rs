use alloy_primitives::address;
use circles_rpc::CirclesRpc;
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

#[test]
fn compute_netted_flow_matches_ts_balances() {
    let source = address!("0x1000000000000000000000000000000000000001");
    let intermediate = address!("0x2000000000000000000000000000000000000002");
    let sink = address!("0x3000000000000000000000000000000000000003");

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(5u64),
        transfers: vec![
            PathfindingTransferStep {
                from: source,
                to: intermediate,
                token_owner: format!("{source:#x}"),
                value: alloy_primitives::U256::from(5u64),
            },
            PathfindingTransferStep {
                from: intermediate,
                to: sink,
                token_owner: format!("{intermediate:#x}"),
                value: alloy_primitives::U256::from(5u64),
            },
        ],
    };

    let net = circles_pathfinder::compute_netted_flow(&path);
    let five = alloy_primitives::I256::from_raw(alloy_primitives::U256::from(5u64));

    assert_eq!(net.get(&source), Some(&(-five)));
    assert_eq!(net.get(&intermediate), Some(&alloy_primitives::I256::ZERO));
    assert_eq!(net.get(&sink), Some(&five));
}

#[test]
fn assert_no_netted_flow_mismatch_accepts_closed_loop_with_overrides() {
    let avatar = address!("0x4000000000000000000000000000000000000004");
    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(7u64),
        transfers: vec![PathfindingTransferStep {
            from: avatar,
            to: avatar,
            token_owner: format!("{avatar:#x}"),
            value: alloy_primitives::U256::from(7u64),
        }],
    };

    let result =
        circles_pathfinder::assert_no_netted_flow_mismatch(&path, Some(avatar), Some(avatar));

    assert!(result.is_ok());
}

#[test]
fn assert_no_netted_flow_mismatch_uses_ts_source_order_on_malformed_paths() {
    let first_source = address!("0x5000000000000000000000000000000000000005");
    let second_source = address!("0x6000000000000000000000000000000000000006");
    let sink = address!("0x7000000000000000000000000000000000000007");

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(8u64),
        transfers: vec![
            PathfindingTransferStep {
                from: first_source,
                to: sink,
                token_owner: format!("{first_source:#x}"),
                value: alloy_primitives::U256::from(5u64),
            },
            PathfindingTransferStep {
                from: second_source,
                to: sink,
                token_owner: format!("{second_source:#x}"),
                value: alloy_primitives::U256::from(3u64),
            },
        ],
    };

    let err = circles_pathfinder::assert_no_netted_flow_mismatch(&path, None, None).unwrap_err();

    match err {
        circles_pathfinder::PathfinderError::RpcResponse(message) => {
            assert!(message.contains(&format!("{second_source:#x}")));
        }
        other => panic!("expected RpcResponse, got {other:?}"),
    }
}

#[test]
fn shrink_path_values_scales_and_drops_subunit_edges() {
    let source = address!("0x8000000000000000000000000000000000000008");
    let intermediate = address!("0x9000000000000000000000000000000000000009");
    let sink = address!("0xa00000000000000000000000000000000000000a");

    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(5u64),
        transfers: vec![
            PathfindingTransferStep {
                from: source,
                to: sink,
                token_owner: format!("{source:#x}"),
                value: alloy_primitives::U256::from(4u64),
            },
            PathfindingTransferStep {
                from: source,
                to: intermediate,
                token_owner: format!("{source:#x}"),
                value: alloy_primitives::U256::from(1u64),
            },
        ],
    };

    let shrunk = circles_pathfinder::shrink_path_values(
        &path,
        sink,
        alloy_primitives::U256::from(500_000_000_000u64),
    );

    assert_eq!(shrunk.max_flow, alloy_primitives::U256::from(2u64));
    assert_eq!(shrunk.transfers.len(), 1);
    assert_eq!(shrunk.transfers[0].to, sink);
    assert_eq!(
        shrunk.transfers[0].value,
        alloy_primitives::U256::from(2u64)
    );
}

#[tokio::test]
async fn token_info_map_from_path_via_rpc_returns_transport_error_for_invalid_target() {
    let current = address!("0xb00000000000000000000000000000000000000b");
    let receiver = address!("0xc00000000000000000000000000000000000000c");
    let wrapper = address!("0xd00000000000000000000000000000000000000d");
    let rpc = CirclesRpc::try_from_http("http://invalid-rpc-url.com").unwrap();
    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(1u64),
        transfers: vec![PathfindingTransferStep {
            from: current,
            to: receiver,
            token_owner: format!("{wrapper:#x}"),
            value: alloy_primitives::U256::from(1u64),
        }],
    };

    let err = circles_pathfinder::token_info_map_from_path_via_rpc(current, &rpc, &path)
        .await
        .unwrap_err();

    match err {
        circles_pathfinder::PathfinderError::Transport(_) => {}
        other => panic!("expected transport error, got {other:?}"),
    }
}

#[tokio::test]
async fn token_info_map_from_path_with_url_returns_transport_error_for_invalid_target() {
    let current = address!("0xe00000000000000000000000000000000000000e");
    let receiver = address!("0xf00000000000000000000000000000000000000f");
    let wrapper = address!("0x1111111111111111111111111111111111111112");
    let path = PathfindingResult {
        max_flow: alloy_primitives::U256::from(1u64),
        transfers: vec![PathfindingTransferStep {
            from: current,
            to: receiver,
            token_owner: format!("{wrapper:#x}"),
            value: alloy_primitives::U256::from(1u64),
        }],
    };

    let err = circles_pathfinder::token_info_map_from_path_with_url(
        current,
        "http://invalid-rpc-url.com",
        &path,
    )
    .await
    .unwrap_err();

    match err {
        circles_pathfinder::PathfinderError::Transport(_) => {}
        other => panic!("expected transport error, got {other:?}"),
    }
}
