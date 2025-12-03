use alloy_primitives::FixedBytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CidError {
    #[error("invalid base58 cid: {0}")]
    InvalidCid(String),
    #[error("cid digest must be 32 bytes, got {0} bytes")]
    InvalidLength(usize),
}

/// Convert CIDv0 (base58btc) to bytes32 digest.
pub fn cid_v0_to_digest(cid: &str) -> Result<FixedBytes<32>, CidError> {
    let data = bs58::decode(cid)
        .into_vec()
        .map_err(|e| CidError::InvalidCid(e.to_string()))?;
    // CIDv0 = 0x12 0x20 + 32-byte multihash digest
    if data.len() != 34 {
        return Err(CidError::InvalidLength(data.len()));
    }
    Ok(FixedBytes::from_slice(&data[2..]))
}

#[cfg(test)]
mod tests {
    use super::cid_v0_to_digest;

    #[test]
    fn cid_roundtrip_length() {
        let cid = "QmfDWxB9jtEGHLi6ToJKWyoXeRzu64WBSKUfnwCWKHLsFn";
        let digest = cid_v0_to_digest(cid).unwrap();
        assert_eq!(digest.len(), 32);
    }
}
