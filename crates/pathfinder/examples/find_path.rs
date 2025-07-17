use alloy_primitives::Address;
use alloy_primitives::aliases::{U192, U256};
use circles_pathfinder::{
    create_flow_matrix, encode_redeem_trusted_data, find_path, prepare_flow_for_contract_simple,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let from =
        Address::parse_checksummed("0xcF6Dc192dc292D5F2789DA2DB02D6dD4f41f4214", None).unwrap();
    let to =
        Address::parse_checksummed("0xeDe0C2E70E8e2d54609c1BdF79595506B6F623FE", None).unwrap();
    let amount = U192::from(1000000000000000000_u64); // 1 CRC
    let rpc = "https://rpc.aboutcircles.com/";

    // 1. ask RPC for transfers
    let transfers = find_path(rpc, from, to, amount, true).await.unwrap();

    // 2. build ABI flow-matrix
    let _matrix = create_flow_matrix(from, to, amount, &transfers).unwrap();
    let encoded_data = test_redeem_trusted_data_encoding().await;

    println!("Encoded data: \n{:?}", encoded_data[0]);
    Ok(())
}

async fn test_redeem_trusted_data_encoding() -> Vec<Vec<u8>> {
    let rpc_url = "https://rpc.aboutcircles.com/";

    // Hardcoded JSON payloads as strings
    let _payload1 = r#"
    {
        "id": "0x4652021487668a2c25747c81dc7d553d3c3121df19fac8c7f49e5adc478d1d31",
        "subscriber": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
        "recipient": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
        "amount": "10000000000000000",
        "category": "trusted",
        "next_redeem_at": 0
    }
    "#;

    let _payload2 = r#"
    {
        "id": "0xdc849e3b51c6cd3b3c5b5f028c7889f1b2d722f9f8ddbaffd3693208e34a494e",
        "subscriber": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
        "recipient": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
        "amount": "10000000000000000",
        "category": "trusted",
        "next_redeem_at": 0
    }
    "#;

    // Parse payloads (assuming we have serde for real test)
    // For simplicity, hardcode values
    let subs = vec![
        (
            "0x4652021487668a2c25747c81dc7d553d3c3121df19fac8c7f49e5adc478d1d31",
            "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
            "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
            "10000000000000000",
        ),
        (
            "0xdc849e3b51c6cd3b3c5b5f028c7889f1b2d722f9f8ddbaffd3693208e34a494e",
            "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
            "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
            "10000000000000000",
        ),
    ];

    let mut redemptions = vec![];
    for (_id, sub_str, rec_str, amt_str) in subs {
        let subscriber = Address::from_str(sub_str).unwrap();
        let recipient = Address::from_str(rec_str).unwrap();
        let amount = U192::from_str_radix(amt_str, 10).unwrap();

        let path_data = prepare_flow_for_contract_simple(
            rpc_url, subscriber, recipient, amount, false, // use_wrapped_balances = false
        )
        .await
        .expect("Failed to prepare flow data");

        let data = encode_redeem_trusted_data(
            path_data.flow_vertices,
            path_data.flow_edges,
            path_data.streams,
            path_data.packed_coordinates,
            U256::ZERO,
        );
        redemptions.push(data);
    }
    redemptions
}
