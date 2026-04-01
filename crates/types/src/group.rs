use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Deserializer, Serialize};

/// Group row information
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRow {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub group: Address,
    #[serde(rename = "type")]
    pub group_type: String,
    pub owner: Address,
    pub mint_policy: Option<Address>,
    pub mint_handler: Option<Address>,
    pub treasury: Option<Address>,
    pub service: Option<Address>,
    pub fee_collection: Option<Address>,
    pub member_count: Option<u32>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub cid_v0_digest: Option<String>,
    pub erc20_wrapper_demurraged: Option<Address>,
    pub erc20_wrapper_static: Option<Address>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupRowWire {
    #[serde(default)]
    block_number: u64,
    #[serde(default)]
    timestamp: u64,
    #[serde(default)]
    transaction_index: u32,
    #[serde(default)]
    log_index: u32,
    #[serde(default)]
    transaction_hash: Option<TxHash>,
    group: Address,
    #[serde(default, rename = "type")]
    group_type: Option<String>,
    #[serde(default)]
    owner: Option<Address>,
    #[serde(default)]
    mint: Option<Address>,
    #[serde(default)]
    mint_policy: Option<Address>,
    #[serde(default)]
    mint_handler: Option<Address>,
    #[serde(default)]
    treasury: Option<Address>,
    #[serde(default)]
    service: Option<Address>,
    #[serde(default)]
    fee_collection: Option<Address>,
    #[serde(default)]
    member_count: Option<u32>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    cid_v0_digest: Option<String>,
    #[serde(default)]
    erc20_wrapper_demurraged: Option<Address>,
    #[serde(default)]
    erc20_wrapper_static: Option<Address>,
}

impl<'de> Deserialize<'de> for GroupRow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = GroupRowWire::deserialize(deserializer)?;

        Ok(Self {
            block_number: wire.block_number,
            timestamp: wire.timestamp,
            transaction_index: wire.transaction_index,
            log_index: wire.log_index,
            transaction_hash: wire.transaction_hash.unwrap_or(TxHash::ZERO),
            group: wire.group,
            group_type: wire.group_type.unwrap_or_default(),
            owner: wire.owner.or(wire.mint).unwrap_or(Address::ZERO),
            mint_policy: wire.mint_policy,
            mint_handler: wire.mint_handler,
            treasury: wire.treasury,
            service: wire.service,
            fee_collection: wire.fee_collection,
            member_count: wire.member_count,
            name: wire.name,
            symbol: wire.symbol,
            cid_v0_digest: wire.cid_v0_digest,
            erc20_wrapper_demurraged: wire.erc20_wrapper_demurraged,
            erc20_wrapper_static: wire.erc20_wrapper_static,
        })
    }
}

/// Group membership row
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMembershipRow {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub group: Address,
    pub member: Address,
    pub expiry_time: u64,
}

/// Group token holder row for `GroupTokenHoldersBalance`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupTokenHolderRow {
    pub group: Address,
    pub holder: Address,
    pub total_balance: U256,
    pub demurraged_total_balance: U256,
    pub fraction_ownership: f64,
}

/// Group query parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupQueryParams {
    pub name_starts_with: Option<String>,
    pub symbol_starts_with: Option<String>,
    pub group_address_in: Option<Vec<Address>>,
    pub group_type_in: Option<Vec<String>>,
    pub owner_in: Option<Vec<Address>>,
    pub mint_handler_equals: Option<Address>,
    pub treasury_equals: Option<Address>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn group_row_deserializes_full_query_shape() {
        let value = json!({
            "blockNumber": 1,
            "timestamp": 2,
            "transactionIndex": 3,
            "logIndex": 4,
            "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "group": "0x1111111111111111111111111111111111111111",
            "type": "Standard",
            "owner": "0x2222222222222222222222222222222222222222",
            "name": "Berlin CRC",
            "symbol": "BCRC"
        });

        let row: GroupRow = serde_json::from_value(value).expect("deserialize full group row");

        assert_eq!(row.block_number, 1);
        assert_eq!(row.transaction_index, 3);
        assert_eq!(row.group_type, "Standard");
        assert_eq!(row.owner, Address::repeat_byte(0x22));
    }

    #[test]
    fn group_row_deserializes_plugin_find_groups_shape() {
        let value = json!({
            "group": "0x1111111111111111111111111111111111111111",
            "name": "Berlin CRC",
            "symbol": "BCRC",
            "mint": "0x2222222222222222222222222222222222222222",
            "treasury": "0x3333333333333333333333333333333333333333",
            "blockNumber": 7,
            "timestamp": 8
        });

        let row: GroupRow = serde_json::from_value(value).expect("deserialize plugin group row");

        assert_eq!(row.block_number, 7);
        assert_eq!(row.timestamp, 8);
        assert_eq!(row.owner, Address::repeat_byte(0x22));
        assert_eq!(row.treasury, Some(Address::repeat_byte(0x33)));
        assert_eq!(row.group_type, "");
        assert_eq!(row.transaction_hash, TxHash::ZERO);
    }
}
