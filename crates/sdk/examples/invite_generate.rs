use alloy_primitives::address;
use circles_sdk::{Sdk, config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Replace with real config/addresses; this example only builds txs and prints them.
    let config = config::gnosis_mainnet();

    let sdk = Sdk::new(config, None)?;
    let avatar = sdk
        .get_avatar(address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
        .await?;

    if let circles_sdk::Avatar::Human(h) = avatar {
        let generated = h.generate_invites(2).await?;
        println!("Secrets: {:?}", generated.secrets);
        println!("Signers: {:?}", generated.signers);
        for (i, tx) in generated.txs.iter().enumerate() {
            println!("Tx {i}: to={:?}, data_len={}", tx.to, tx.data.len());
        }
    } else {
        println!("Avatar not human; cannot generate invites");
    }

    Ok(())
}
