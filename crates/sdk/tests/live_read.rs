mod common;

use circles_sdk::Sdk;
use serde_json::to_value;

#[tokio::test]
#[ignore]
async fn live_avatar_info_reads() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cfg) = common::maybe_live_config() else {
        eprintln!("skipping live test: set RUN_LIVE=1");
        return Ok(());
    };
    let Some(addr) = common::maybe_live_avatar() else {
        eprintln!("skipping live test: set LIVE_AVATAR=0x...");
        return Ok(());
    };
    let sdk = Sdk::new(cfg, None)?;
    let info = sdk.avatar_info(addr).await?;
    assert_eq!(info.avatar, addr);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn live_invitation_source_reads() -> Result<(), Box<dyn std::error::Error>> {
    let Some(cfg) = common::maybe_live_config() else {
        eprintln!("skipping live test: set RUN_LIVE=1");
        return Ok(());
    };
    let Some(addr) = common::maybe_live_avatar() else {
        eprintln!("skipping live test: set LIVE_AVATAR=0x...");
        return Ok(());
    };

    let sdk = Sdk::new(cfg, None)?;
    let all = sdk.data_all_invitations(addr, None).await?;
    let trust = sdk.data_trust_invitations(addr, None).await?;
    let escrow = sdk.data_escrow_invitations(addr).await?;
    let at_scale = sdk.data_at_scale_invitations(addr).await?;

    assert_eq!(to_value(&all.trust_invitations)?, to_value(&trust)?);
    assert_eq!(to_value(&all.escrow_invitations)?, to_value(&escrow)?);
    assert_eq!(to_value(&all.at_scale_invitations)?, to_value(&at_scale)?);
    Ok(())
}
