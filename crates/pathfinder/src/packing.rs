use alloy_primitives::Address;
use types::TransferStep;

/// Pack `u16` coordinates into a byte-array (big-endian, no padding).
pub fn pack_coordinates(coords: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(coords.len() * 2);
    for &c in coords {
        out.push((c >> 8) as u8);
        out.push((c & 0xff) as u8);
    }
    out
}

/// Build a sorted vertex list + index map (like TS `transformToFlowVertices`)
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
