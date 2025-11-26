use alloy_primitives::address;
use alloy_primitives::U256;
use circles_transfers::TransferBuilder;
use circles_types::{
    Address, CirclesConfig, PathfindingResult, PathfindingTransferStep, TokenInfo,
};
use circles_utils::converter::atto_circles_to_atto_static_circles;
use std::collections::HashMap;

fn demo_config() -> CirclesConfig {
    CirclesConfig {
        circles_rpc_url: "http://localhost:8545".into(), // unused in these unit tests
        pathfinder_url: "".into(),
        profile_service_url: "".into(),
        v1_hub_address: Address::ZERO,
        v2_hub_address: address!("0x0000000000000000000000000000000000000001"),
        name_registry_address: Address::ZERO,
        base_group_mint_policy: Address::ZERO,
        standard_treasury: Address::ZERO,
        core_members_group_deployer: Address::ZERO,
        base_group_factory_address: Address::ZERO,
        lift_erc20_address: Address::ZERO,
        invitation_escrow_address: Address::ZERO,
        invitation_farm_address: Address::ZERO,
        referrals_module_address: Address::ZERO,
    }
}

#[test]
fn builder_initializes() {
    let cfg = demo_config();
    // Just ensure construction succeeds.
    let _builder = TransferBuilder::new(cfg).expect("builder constructs");
}

#[test]
fn assemble_orders_approval_unwrap_operate_rewrap() {
    let cfg = demo_config();
    let builder = TransferBuilder::new(cfg).unwrap();
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let to = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    // Path uses an inflationary wrapper owned by `to`.
    let wrapper = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut token_info_map = HashMap::new();
    token_info_map.insert(
        wrapper,
        TokenInfo {
            block_number: 0,
            timestamp: 1_700_000_000,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            info_type: None,
            token_type: "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
            token: wrapper,
            token_owner: to,
        },
    );
    let path = PathfindingResult {
        max_flow: U256::from(1_000_000u64),
        transfers: vec![PathfindingTransferStep {
            from,
            to,
            token_owner: format!("{wrapper:#x}"),
            value: U256::from(1_000_000u64),
        }],
    };
    // Wrapped totals: wrapper carries the full amount, type inflationary.
    let mut wrapped = HashMap::new();
    wrapped.insert(
        wrapper,
        (
            U256::from(1_000_000u64),
            "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
        ),
    );
    // Static balance > used amount so we expect a re-wrap tx.
    let mut balances = HashMap::new();
    balances.insert(wrapper, U256::from(2_000_000u64));

    // We bypass approval check by directly calling inner assembly with approval assumed needed.
    // Call inner assembly directly (non-async) and skip the approval check by
    // setting a flag on the builder (uses approval by default).
    let txs = builder
        .assemble_transactions_inner(
            from,
            to,
            path,
            token_info_map,
            wrapped,
            balances,
            circles_types::AdvancedTransferOptions {
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: None,
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                max_transfers: None,
                tx_data: None,
            },
            false, // skip approval check in tests
        )
        .unwrap();

    // Expect: approval, unwrap, operate, re-wrap.
    assert_eq!(txs.len(), 4);
    // Approval and operate go to hub; unwrap to wrapper; re-wrap to hub.
    assert_eq!(txs[0].to, builder.config().v2_hub_address);
    assert_eq!(txs[1].to, wrapper);
    assert_eq!(txs[2].to, builder.config().v2_hub_address);
    assert_eq!(txs[3].to, builder.config().v2_hub_address);
}

#[test]
fn demurraged_only_has_no_rewrap() {
    let cfg = demo_config();
    let builder = TransferBuilder::new(cfg).unwrap();
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let to = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let wrapper = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mut token_info_map = HashMap::new();
    token_info_map.insert(
        wrapper,
        TokenInfo {
            block_number: 0,
            timestamp: 0,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            info_type: None,
            token_type: "CrcV2_ERC20WrapperDeployed_Demurraged".into(),
            token: wrapper,
            token_owner: to,
        },
    );
    let path = PathfindingResult {
        max_flow: U256::from(1_000_000u64),
        transfers: vec![PathfindingTransferStep {
            from,
            to,
            token_owner: format!("{wrapper:#x}"),
            value: U256::from(1_000_000u64),
        }],
    };
    let mut wrapped = HashMap::new();
    wrapped.insert(
        wrapper,
        (
            U256::from(1_000_000u64),
            "CrcV2_ERC20WrapperDeployed_Demurraged".into(),
        ),
    );
    let balances = HashMap::new();

    let txs = builder
        .assemble_transactions(
            from,
            to,
            path,
            token_info_map,
            wrapped,
            balances,
            circles_types::AdvancedTransferOptions {
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: None,
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                max_transfers: None,
                tx_data: None,
            },
            false,
        )
        .unwrap();

    // Expect: approval, unwrap, operate. No rewrap.
    assert_eq!(txs.len(), 3);
    assert_eq!(txs[1].to, wrapper);
    assert_eq!(txs[2].to, builder.config().v2_hub_address);
}

#[test]
fn mixed_wrappers_include_rewrap_for_inflationary() {
    let cfg = demo_config();
    let builder = TransferBuilder::new(cfg).unwrap();
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let sink = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let dem_wrapper = address!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let inf_wrapper = address!("0xcccccccccccccccccccccccccccccccccccccccc");

    let mut token_info_map = HashMap::new();
    token_info_map.insert(
        dem_wrapper,
        TokenInfo {
            block_number: 0,
            timestamp: 0,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            info_type: None,
            token_type: "CrcV2_ERC20WrapperDeployed_Demurraged".into(),
            token: dem_wrapper,
            token_owner: sink,
        },
    );
    token_info_map.insert(
        inf_wrapper,
        TokenInfo {
            block_number: 0,
            timestamp: 1_700_000_000,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            info_type: None,
            token_type: "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
            token: inf_wrapper,
            token_owner: sink,
        },
    );

    let path = PathfindingResult {
        max_flow: U256::from(2_000_000u64),
        transfers: vec![
            PathfindingTransferStep {
                from,
                to: sink,
                token_owner: format!("{dem_wrapper:#x}"),
                value: U256::from(1_000_000u64),
            },
            PathfindingTransferStep {
                from,
                to: sink,
                token_owner: format!("{inf_wrapper:#x}"),
                value: U256::from(1_000_000u64),
            },
        ],
    };

    let mut wrapped = HashMap::new();
    wrapped.insert(
        dem_wrapper,
        (
            U256::from(1_000_000u64),
            "CrcV2_ERC20WrapperDeployed_Demurraged".into(),
        ),
    );
    wrapped.insert(
        inf_wrapper,
        (
            U256::from(1_000_000u64),
            "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
        ),
    );

    let mut balances = HashMap::new();
    balances.insert(inf_wrapper, U256::from(2_000_000u64)); // enough for leftover

    let txs = builder
        .assemble_transactions(
            from,
            sink,
            path,
            token_info_map,
            wrapped,
            balances,
            circles_types::AdvancedTransferOptions {
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: None,
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                max_transfers: None,
                tx_data: None,
            },
            false,
        )
        .unwrap();

    // Expect approval + 2 unwraps + operate + 1 rewrap = 5 txs.
    assert_eq!(txs.len(), 5);
    // Unwraps should target both wrappers (order not guaranteed), operate and rewrap go to hub.
    assert!(txs.iter().any(|tx| tx.to == dem_wrapper));
    assert!(txs.iter().any(|tx| tx.to == inf_wrapper));
    assert_eq!(
        txs.iter()
            .filter(|tx| tx.to == builder.config().v2_hub_address)
            .count(),
        3
    );
}

#[test]
fn inflationary_no_leftover_skips_rewrap() {
    let cfg = demo_config();
    let builder = TransferBuilder::new(cfg).unwrap();
    let from = address!("0xde374ece6fa50e781e81aac78e811b33d16912c7");
    let sink = address!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let inf_wrapper = address!("0xcccccccccccccccccccccccccccccccccccccccc");

    let mut token_info_map = HashMap::new();
    token_info_map.insert(
        inf_wrapper,
        TokenInfo {
            block_number: 0,
            timestamp: 0,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: alloy_primitives::TxHash::ZERO,
            version: 2,
            info_type: None,
            token_type: "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
            token: inf_wrapper,
            token_owner: sink,
        },
    );

    let path = PathfindingResult {
        max_flow: U256::from(1_000_000u64),
        transfers: vec![PathfindingTransferStep {
            from,
            to: sink,
            token_owner: format!("{inf_wrapper:#x}"),
            value: U256::from(1_000_000u64),
        }],
    };

    let mut wrapped = HashMap::new();
    wrapped.insert(
        inf_wrapper,
        (
            U256::from(1_000_000u64),
            "CrcV2_ERC20WrapperDeployed_Inflationary".into(),
        ),
    );

    let mut balances = HashMap::new();
    // Static balance zero -> no leftover for rewrap.
    // Note: this forces the leftover==0 branch. A more realistic case would set
    // balance == static_used (computed with matching timestamp) to mirror the
    // converter exactly; we can add that when fixtures are aligned.
    balances.insert(inf_wrapper, U256::ZERO);

    let txs = builder
        .assemble_transactions(
            from,
            sink,
            path,
            token_info_map,
            wrapped,
            balances,
            circles_types::AdvancedTransferOptions {
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: None,
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                max_transfers: None,
                tx_data: None,
            },
            false,
        )
        .unwrap();

    // Expect approval + unwrap + operate, no rewrap.
    assert_eq!(txs.len(), 3);
    assert!(txs.iter().any(|tx| tx.to == inf_wrapper));
    assert_eq!(
        txs.iter()
            .filter(|tx| tx.to == builder.config().v2_hub_address)
            .count(),
        2
    );
}

// Integration-style tests that exercise actual RPC/pathfinding are out of scope
// for unit tests; they require a live Circles RPC. Add mocks when available.
