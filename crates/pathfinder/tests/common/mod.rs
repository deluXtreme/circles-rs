use alloy_primitives::Address;
use alloy_primitives::aliases::U192;
use circles_types::TransferStep;

/// Create a sample address from a hex string (pads short addresses to 20 bytes)
pub fn address_from_str(hex_str: &str) -> Address {
    let clean_hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    // Pad to 40 characters (20 bytes) if needed
    let padded = if clean_hex.len() < 40 {
        format!("{:0>40}", clean_hex)
    } else {
        clean_hex.to_string()
    };

    format!("0x{}", padded)
        .parse()
        .expect("Invalid address format")
}

/// Create a sample TransferStep for testing
#[allow(dead_code)]
pub fn sample_transfer_step(
    from: Address,
    to: Address,
    token_owner: Address,
    value: U192,
) -> TransferStep {
    TransferStep {
        from_address: from,
        to_address: to,
        token_owner,
        value,
    }
}

/// Convert a value in wei (as string) to U256
pub fn wei_from_str(wei_str: &str) -> U192 {
    U192::from_str_radix(wei_str, 10).expect("Invalid wei value")
}

/// Common test constants
#[allow(dead_code)]
pub const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/";
pub const ONE_ETH_WEI: &str = "1000000000000000000";
#[allow(dead_code)]
pub const ONE_TENTH_ETH_WEI: &str = "100000000000000000";

/// Sample addresses used in tests
pub mod addresses {
    use super::address_from_str;
    use alloy_primitives::Address;

    pub fn sender() -> Address {
        address_from_str("0x52e14be00d5acff4424ad625662c6262b4fd1a58")
    }

    pub fn receiver() -> Address {
        address_from_str("0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214")
    }

    #[allow(dead_code)]
    pub fn intermediate_a5() -> Address {
        address_from_str("0xa5")
    }

    #[allow(dead_code)]
    pub fn intermediate_63() -> Address {
        address_from_str("0x63")
    }

    #[allow(dead_code)]
    pub fn token_owner_7b() -> Address {
        address_from_str("0x7b")
    }

    #[allow(dead_code)]
    pub fn token_owner_f7() -> Address {
        address_from_str("0xf7")
    }
}
