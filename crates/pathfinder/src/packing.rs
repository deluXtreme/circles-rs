// src/packing.rs
//! Coordinate packing and vertex transformation utilities.
//!
//! Provides efficient algorithms for packing coordinate data and transforming
//! transfer steps into deterministically sorted vertex lists.
use alloy_primitives::Address;
use circles_types::TransferStep;

/// Pack coordinate values into a compact byte representation.
///
/// Converts a sequence of `u16` coordinates into a packed byte array using
/// big-endian encoding. This format is optimized for on-chain storage and
/// smart contract consumption.
///
/// # Arguments
/// * `coords` - Slice of coordinate values to pack
///
/// # Returns
/// Packed byte array where each `u16` becomes 2 bytes in big-endian format.
///
/// # Examples
/// ```rust
/// use circles_pathfinder::pack_coordinates;
///
/// let coords = vec![0x1234, 0x5678];
/// let packed = pack_coordinates(&coords);
/// assert_eq!(packed, vec![0x12, 0x34, 0x56, 0x78]);
/// ```
pub fn pack_coordinates(coords: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(coords.len() * 2);
    for &c in coords {
        out.push((c >> 8) as u8);
        out.push((c & 0xff) as u8);
    }
    out
}

/// Transform transfer steps into sorted vertices and coordinate mapping.
///
/// Creates a deterministically sorted list of all unique addresses involved
/// in the transfers, plus a mapping from addresses to vertex indices. This
/// is used internally by flow matrix generation.
///
/// # Arguments
/// * `transfers` - Transfer steps to process
/// * `from` - Additional source address to include
/// * `to` - Additional destination address to include
///
/// # Returns
/// Tuple of (sorted_vertices, address_to_index_map)
pub fn transform_to_flow_vertices(
    transfers: &[TransferStep],
    from: Address,
    to: Address,
) -> (Vec<Address>, std::collections::HashMap<Address, usize>) {
    use std::collections::HashSet;
    let mut set: HashSet<Address> = HashSet::from([from, to]);
    for t in transfers {
        set.insert(t.from_address);
        set.insert(t.to_address);
        set.insert(t.token_owner);
    }
    // Sort by numeric value for determinism
    let mut sorted: Vec<_> = set.into_iter().collect();
    sorted.sort_by(|a, b| a.as_slice().cmp(b.as_slice())); // byte-wise ordering
    let idx = sorted
        .iter()
        .enumerate()
        .map(|(i, a)| (*a, i))
        .collect::<std::collections::HashMap<_, _>>();
    (sorted, idx)
}
