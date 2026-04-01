use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{
    Address, AvatarType, EnrichedTransaction, EnrichedTransactionHistoryOptions,
    PagedAggregatedTrustRelationsResponse, PagedProfileSearchResponse, PagedResponse,
    PagedValidInvitersResponse, ProfileView, TrustNetworkSummary,
};

const DEFAULT_TRUST_LIMIT: u32 = 50;
const DEFAULT_INVITER_LIMIT: u32 = 50;
const DEFAULT_TX_LIMIT: u32 = 20;
const DEFAULT_SEARCH_LIMIT: u32 = 20;

/// Dedicated SDK-enablement RPC methods.
///
/// These map to consolidated host endpoints that replace multiple lower-level RPC calls.
#[derive(Clone, Debug)]
pub struct SdkMethods {
    client: RpcClient,
}

impl SdkMethods {
    /// Create a new accessor for SDK-enablement RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// `circles_getProfileView`
    pub async fn get_profile_view(&self, address: Address) -> Result<ProfileView> {
        self.client.call("circles_getProfileView", (address,)).await
    }

    /// `circles_getTrustNetworkSummary`
    pub async fn get_trust_network_summary(
        &self,
        address: Address,
        max_depth: Option<u32>,
    ) -> Result<TrustNetworkSummary> {
        match max_depth {
            Some(max_depth) => {
                self.client
                    .call("circles_getTrustNetworkSummary", (address, max_depth))
                    .await
            }
            None => {
                self.client
                    .call("circles_getTrustNetworkSummary", (address,))
                    .await
            }
        }
    }

    /// `circles_getAggregatedTrustRelationsEnriched`
    pub async fn get_aggregated_trust_relations_enriched(
        &self,
        address: Address,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedAggregatedTrustRelationsResponse> {
        match (limit, cursor) {
            (None, None) => {
                self.client
                    .call("circles_getAggregatedTrustRelationsEnriched", (address,))
                    .await
            }
            (Some(limit), None) => {
                self.client
                    .call(
                        "circles_getAggregatedTrustRelationsEnriched",
                        (address, limit),
                    )
                    .await
            }
            (limit, cursor) => {
                self.client
                    .call(
                        "circles_getAggregatedTrustRelationsEnriched",
                        (
                            address,
                            limit.unwrap_or(DEFAULT_TRUST_LIMIT),
                            cursor.map(str::to_owned),
                        ),
                    )
                    .await
            }
        }
    }

    /// `circles_getValidInviters`
    pub async fn get_valid_inviters(
        &self,
        address: Address,
        minimum_balance: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedValidInvitersResponse> {
        if limit.is_none() && cursor.is_none() {
            match minimum_balance {
                Some(minimum_balance) => {
                    self.client
                        .call(
                            "circles_getValidInviters",
                            (address, minimum_balance.to_owned()),
                        )
                        .await
                }
                None => {
                    self.client
                        .call("circles_getValidInviters", (address,))
                        .await
                }
            }
        } else {
            self.client
                .call(
                    "circles_getValidInviters",
                    (
                        address,
                        minimum_balance.map(str::to_owned),
                        limit.unwrap_or(DEFAULT_INVITER_LIMIT),
                        cursor.map(str::to_owned),
                    ),
                )
                .await
        }
    }

    /// `circles_getTransactionHistoryEnriched`
    pub async fn get_transaction_history_enriched(
        &self,
        address: Address,
        from_block: u64,
        options: EnrichedTransactionHistoryOptions,
    ) -> Result<PagedResponse<EnrichedTransaction>> {
        if options.to_block.is_none()
            && options.limit.is_none()
            && options.cursor.is_none()
            && options.version.is_none()
            && options.exclude_intermediary.is_none()
        {
            self.client
                .call(
                    "circles_getTransactionHistoryEnriched",
                    (address, from_block),
                )
                .await
        } else {
            self.client
                .call(
                    "circles_getTransactionHistoryEnriched",
                    (
                        address,
                        from_block,
                        options.to_block,
                        options.limit.unwrap_or(DEFAULT_TX_LIMIT),
                        options.cursor,
                        options.version,
                        options.exclude_intermediary.unwrap_or(true),
                    ),
                )
                .await
        }
    }

    /// `circles_searchProfileByAddressOrName`
    pub async fn search_profile_by_address_or_name(
        &self,
        query: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
        types: Option<Vec<AvatarType>>,
    ) -> Result<PagedProfileSearchResponse> {
        if limit.is_none() && cursor.is_none() && types.is_none() {
            self.client
                .call("circles_searchProfileByAddressOrName", (query.to_owned(),))
                .await
        } else {
            self.client
                .call(
                    "circles_searchProfileByAddressOrName",
                    (
                        query.to_owned(),
                        limit.unwrap_or(DEFAULT_SEARCH_LIMIT),
                        cursor.map(str::to_owned),
                        types,
                    ),
                )
                .await
        }
    }
}
