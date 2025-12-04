mod common;

use circles_sdk::Sdk;
use circles_types::AdvancedTransferOptions;
use std::str::FromStr;

#[tokio::test]
#[ignore]
async fn live_max_flow_self() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cfg) = common::maybe_live_config() else {
        eprintln!("skipping live test: set RUN_LIVE=1");
        return Ok(());
    };
    let Some(addr) = common::maybe_live_avatar() else {
        eprintln!("skipping live test: set LIVE_AVATAR=0x...");
        return Ok(());
    };

    let sdk = Sdk::new(cfg, None)?;
    let avatar = sdk.get_avatar(addr).await?;

    // Self-to-self max flow is a safe read; options use wrapped balances by default.
    let options = AdvancedTransferOptions {
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
        simulated_balances: None,
        max_transfers: None,
        tx_data: None,
    };

    if let circles_sdk::Avatar::Human(h) = avatar {
        let res = h.max_flow_to(addr, Some(options)).await?;
        println!("max_flow to self: {}", res.max_flow);
    } else {
        eprintln!("skipping: avatar is not human, cannot pathfind self");
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn live_max_flow_to_targets() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cfg) = common::maybe_live_config() else {
        eprintln!("skipping live test: set RUN_LIVE=1");
        return Ok(());
    };
    let Some(addr) = common::maybe_live_avatar() else {
        eprintln!("skipping live test: set LIVE_AVATAR=0x...");
        return Ok(());
    };

    // Use env-provided targets if set, otherwise a baked-in list of active avatars.
    let targets: Vec<_> = std::env::var("LIVE_TARGETS")
        .ok()
        .and_then(|csv| {
            let parsed: Vec<_> = csv
                .split(',')
                .filter_map(|s| circles_types::Address::from_str(s.trim()).ok())
                .collect();
            if parsed.is_empty() {
                None
            } else {
                Some(parsed)
            }
        })
        .unwrap_or_else(|| {
            [
                "0xbf4d332242049ebf71da676ac5fa01a74121dc0d",
                "0x96821a4f2e986729759a146abedceacba690351c",
                "0xd447bdea939313c2e83654be3220e87fd0d7bdf6",
                "0x9c4722d5d93e721db31afffb0dcb6938fe0d9f8e",
                "0xa65d69e34da7ffcb45804aa437b1f4c9fedeaef7",
                "0xede0c2e70e8e2d54609c1bdf79595506b6f623fe",
                "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
                "0xf48554937f18885c7f15c432c596b5843648231d",
            ]
            .iter()
            .filter_map(|s| circles_types::Address::from_str(s).ok())
            .collect()
        });

    let sdk = Sdk::new(cfg, None)?;
    let avatar = sdk.get_avatar(addr).await?;

    if let circles_sdk::Avatar::Human(h) = avatar {
        let mut successes = 0usize;
        for target in targets {
            match h.max_flow_to(target, None).await {
                Ok(res) => {
                    successes += 1;
                    println!("max_flow to {:#x}: {}", target, res.max_flow);
                }
                Err(err) => {
                    eprintln!("max_flow_to {target:#x} failed: {err}");
                }
            }
        }
        if successes == 0 {
            eprintln!("no max_flow_to targets succeeded; check trust graph or targets");
        }
    } else {
        eprintln!("skipping: avatar is not human, cannot pathfind");
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn live_plan_transfer_self_zero() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cfg) = common::maybe_live_config() else {
        eprintln!("skipping live test: set RUN_LIVE=1");
        return Ok(());
    };
    let Some(addr) = common::maybe_live_avatar() else {
        eprintln!("skipping live test: set LIVE_AVATAR=0x...");
        return Ok(());
    };

    let sdk = Sdk::new(cfg, None)?;
    let avatar = sdk.get_avatar(addr).await?;

    if let circles_sdk::Avatar::Human(h) = avatar {
        // zero transfer planning may return NoPathFound; tolerate that
        match h
            .plan_transfer(addr, alloy_primitives::U256::from(0u64), None)
            .await
        {
            Ok(txs) => assert!(txs.is_empty(), "expected no txs for zero transfer"),
            Err(circles_sdk::SdkError::Transfers(
                circles_transfers::TransferError::NoPathFound { .. },
            )) => {
                eprintln!("no path found for zero transfer; tolerated");
            }
            Err(err) => return Err(err.into()),
        }
    } else {
        eprintln!("skipping: avatar is not human, cannot plan transfer");
    }

    Ok(())
}
