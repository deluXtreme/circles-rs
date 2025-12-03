use alloy_primitives::address;
use circles_sdk::{Sdk, config};
use circles_types::AdvancedTransferOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Minimal config: uses circles_types::CirclesConfig defaults if you pass your own.
    // Here we hardcode mainnet endpoints (replace with env/config as needed).
    let config = config::gnosis_mainnet();

    let sdk = Sdk::new(config, None)?;
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let info = sdk.avatar_info(avatar).await?;
    println!("Avatar info: {:?}", info.avatar_type);

    let balances = sdk.data_balances(avatar, true, true).await?;
    println!("Balances entries: {}", balances.len());

    let trust = sdk.data_trust(avatar).await?;
    println!("Trust relations: {}", trust.len());

    // Pathfind self to self (max flow) as a demo (requires Human avatar).
    if let circles_sdk::Avatar::Human(human) = sdk.get_avatar(avatar).await? {
        let max_flow = human
            .max_flow_to(
                avatar,
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: None,
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    max_transfers: None,
                    tx_data: None,
                }),
            )
            .await?;
        println!("Max flow: {}", max_flow.max_flow);
    } else {
        println!("Avatar is not human; skipping pathfind demo");
    }

    Ok(())
}
