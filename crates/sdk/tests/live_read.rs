mod common;

use circles_sdk::Sdk;

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
