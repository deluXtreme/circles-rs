use serde::{Deserialize, Serialize};

/// Decoded contract error information
/// Contains parsed error data from failed contract transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedContractError {
    pub error_name: String,
    pub args: Option<Vec<serde_json::Value>>,
    pub selector: String,
    pub raw_data: String,
    pub formatted_message: String,
}
