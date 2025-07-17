use alloy_primitives::aliases::U192;
use alloy_primitives::hex::ToHexExt;
use alloy_primitives::{Address, FixedBytes};
use circles_pathfinder::{
    create_flow_matrix, encode_redeem_flow_matrix, encode_redeem_trusted_data, find_path,
    prepare_flow_for_contract_simple,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let rpc_url = "https://rpc.aboutcircles.com/";

    // Hardcoded JSON payloads as strings
    let _payload = r#"
    {
        "id": "0x9c4412d30af600c6de7a2c746d92d63d30e67cac94946358f43422c2e08d067d",
        "subscriber": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
        "recipient": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
        "amount": "10000000000000000",
        "periods": 47,
        "category": "trusted",
        "next_redeem_at": 1752862015
    }
    "#;

    // Parse payloads (assuming we have serde for real test)
    // For simplicity, hardcode values
    let id_str = "0x9c4412d30af600c6de7a2c746d92d63d30e67cac94946358f43422c2e08d067d";
    let sub_str = "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214";
    let rec_str = "0x6b69683c8897e3d18e74b1ba117b49f80423da5d";
    let amt_str = "10000000000000000";
    let periods_str = "47";

    let _id: FixedBytes<32> = id_str.parse().unwrap();
    let subscriber = Address::from_str(sub_str).unwrap();
    let recipient = Address::from_str(rec_str).unwrap();
    let amount = U192::from_str_radix(amt_str, 10).unwrap();
    let periods = U192::from_str_radix(periods_str, 10).unwrap();
    let target_flow = amount * periods;

    let path_data = prepare_flow_for_contract_simple(
        rpc_url,
        subscriber,
        recipient,
        target_flow,
        true, // use_wrapped_balances
    )
    .await
    .expect("Failed to prepare flow data");

    let data = encode_redeem_trusted_data(
        path_data.flow_vertices,
        path_data.flow_edges,
        path_data.streams,
        path_data.packed_coordinates,
        path_data.source_coordinate,
    );

    let expected_data = "00000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000260000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000006b69683c8897e3d18e74b1ba117b49f80423da5d000000000000000000000000cf6dc192dc292d5f2789da2db02d6dd4f41f4214000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000685c682846f0000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060001000100000000000000000000000000000000000000000000000000000000";

    // 1. ask RPC for transfers
    let transfers = find_path(rpc_url, subscriber, recipient, target_flow, true)
        .await
        .unwrap();

    // 2. build ABI flow-matrix
    let matrix = create_flow_matrix(subscriber, recipient, target_flow, &transfers).unwrap();
    let matrix_encoded_data = encode_redeem_flow_matrix(matrix);
    let matrix_hex_data = matrix_encoded_data.encode_hex();

    let encoded_data = data;
    let hex_data = encoded_data.encode_hex();

    println!("Encoded data: \n{hex_data}");
    println!("-------------------------------------------------");
    println!("Expected data: \n{matrix_hex_data}");
    assert_eq!(hex_data, expected_data);
    assert_eq!(matrix_hex_data, expected_data);
    Ok(())
}
