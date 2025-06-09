use alloy_primitives::{Address, U256};
use pathfinder::{create_flow_matrix, find_path};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // CLI flags omitted for brevity
    let from =
        Address::parse_checksummed("0xcF6Dc192dc292D5F2789DA2DB02D6dD4f41f4214", None).unwrap();
    let to =
        Address::parse_checksummed("0xeDe0C2E70E8e2d54609c1BdF79595506B6F623FE", None).unwrap();
    let amount = U256::from(1000000000000000000_u64); // 1 CRC
    let rpc = "https://rpc.aboutcircles.com/";

    // 1. ask RPC for transfers
    let transfers = find_path(rpc, from, to, amount, true).await.unwrap();

    // 2. build ABI flow-matrix
    let matrix = create_flow_matrix(from, to, amount, &transfers).unwrap();

    println!("flowVertices = {:?}", matrix.flow_vertices);
    Ok(())
}
