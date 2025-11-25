use alloy_primitives::U256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CirclesType enum
/// Represents the type of Circles ERC20 wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CirclesType {
    Demurrage = 0,
    Inflation = 1,
}

/// Information about a wrapped token found in a transfer path
/// Maps wrapper address to [amount used in path, wrapper type]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrappedTokenInfo {
    /// Amount of the wrapped token used in the path
    pub amount: U256,
    /// The type of wrapper (e.g., 'CrcV2_ERC20WrapperDeployed_Demurraged' or 'CrcV2_ERC20WrapperDeployed_Inflationary')
    pub token_type: String,
}

/// Record of wrapped tokens found in a transfer path
/// Maps wrapper address to wrapped token information
pub type WrappedTokensRecord = HashMap<String, WrappedTokenInfo>;
