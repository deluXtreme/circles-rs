use alloy_primitives::Address;
use circles_types::TokenInfo;

pub fn mock_token_info(token: Address, token_owner: Address, token_type: &str) -> TokenInfo {
    TokenInfo {
        block_number: 0,
        timestamp: 0,
        transaction_index: 0,
        log_index: 0,
        transaction_hash: alloy_primitives::TxHash::ZERO,
        version: 2,
        info_type: None,
        token_type: token_type.to_string(),
        token,
        token_owner,
    }
}
