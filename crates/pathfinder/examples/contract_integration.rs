use alloy_primitives::Address;
use alloy_primitives::aliases::U192;
use circles_pathfinder::{PathData, FindPathParams, prepare_flow_for_contract};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example addresses (replace with real addresses)
    let sender = Address::from_str("0x52e14be00d5acff4424ad625662c6262b4fd1a58")?;
    let receiver = Address::from_str("0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214")?;
    let amount = U192::from_str("1000000000000000000")?; // 1 CRC in wei
    let rpc_url = "https://rpc.aboutcircles.com/";

    println!("Finding path and preparing contract data...");
    println!("From: {}", sender);
    println!("To: {}", receiver);
    println!("Amount: {} wei", amount);

    let params = FindPathParams {
        from: sender,
        to: receiver,
        target_flow: amount,
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
    };

    // NEW API: One function call does everything
    let path_data: PathData = prepare_flow_for_contract(rpc_url, params).await?;

    println!("\nFlow matrix prepared for contract calls:");
    println!(
        "Flow vertices: {} addresses",
        path_data.flow_vertices.len()
    );
    println!("Flow edges: {} transfers", path_data.flow_edges.len());
    println!("Streams: {} streams", path_data.streams.len());
    println!(
        "Packed coordinates: {} bytes",
        path_data.packed_coordinates.len()
    );
    println!("Source coordinate: {}", path_data.source_coordinate);

    // Demonstrate contract-ready data
    println!("\nContract-ready data:");

    // Flow vertices (already Address types)
    println!(
        "Vertices (first 3): {:?}",
        &path_data.flow_vertices[..3.min(path_data.flow_vertices.len())]
    );

    // Flow edges (raw tuples - convert to contract types)
    for (i, (stream_sink_id, amount)) in path_data.flow_edges.iter().take(3).enumerate() {
        println!(
            "Edge {}: stream_sink_id={}, amount={}",
            i, stream_sink_id, amount
        );
    }

    // Streams (raw tuples - convert to contract types)
    for (i, (source_coordinate, flow_edge_ids, _data)) in path_data.streams.iter().enumerate() {
        println!(
            "Stream {}: source_coordinate={}, flow_edge_ids={:?}",
            i, source_coordinate, flow_edge_ids
        );
    }

    // Packed coordinates (raw bytes - convert to Bytes for contracts)
    println!(
        "Packed coordinates length: {} bytes",
        path_data.packed_coordinates.len()
    );

    // Example: How you would use this in a smart contract call
    println!("\nExample smart contract usage:");
    println!("```rust");
    println!("let contract = MyContract::new(contract_address, provider);");
    println!("let tx = contract");
    println!("    .redeemPayment(");
    println!("        module_address,");
    println!("        subscription_id,");
    println!("        path_data.flow_vertices,           // Vec<Address>");
    println!("        path_data.to_flow_edges(),         // Vec<FlowEdge>");
    println!("        path_data.to_streams(),            // Vec<Stream>");
    println!("        path_data.to_packed_coordinates()  // Bytes");
    println!("    )");
    println!("    .send()");
    println!("    .await?;");
    println!("```");

    // Demonstrate decomposition for tuple-based contract calls
    let (vertices, edges, streams, packed_coords) = path_data.to_contract_params();
    println!("\nFor tuple-based contract calls:");
    println!("Decomposed into {} components ready for contract", 4);
    println!("- {} vertices", vertices.len());
    println!("- {} edges", edges.len());
    println!("- {} streams", streams.len());
    println!("- {} bytes of coordinates", packed_coords.len());

    Ok(())
}
