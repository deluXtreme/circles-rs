use crate::client::RpcClient;
use crate::error::{CirclesRpcError, Result};
use crate::methods::QueryMethods;
use crate::paged_query::{PagedFetch, PagedQuery};
use circles_types::{
    Address, Conjunction, CursorColumn, Filter, FilterPredicate, GroupMembershipRow,
    GroupQueryParams, GroupRow, GroupTokenHolderRow, OrderBy, PagedQueryParams, PagedResponse,
    SortOrder,
};
use serde::Serialize;
use std::pin::Pin;
use std::sync::Arc;

const DEFAULT_FIND_GROUPS_LIMIT: u32 = 50;
const DEFAULT_GROUP_MEMBERS_LIMIT: u32 = 100;
const DEFAULT_GROUP_MEMBERSHIPS_LIMIT: u32 = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeGroupQueryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    name_starts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol_starts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_in: Option<Vec<Address>>,
}

impl From<GroupQueryParams> for NativeGroupQueryParams {
    fn from(params: GroupQueryParams) -> Self {
        Self {
            name_starts_with: params.name_starts_with,
            symbol_starts_with: params.symbol_starts_with,
            owner_in: params.owner_in,
        }
    }
}

/// Methods for group membership lookups (`circles_getGroupMemberships`, `circles_getGroups`).
#[derive(Clone, Debug)]
pub struct GroupMethods {
    client: RpcClient,
}

impl GroupMethods {
    /// Create a new accessor for group-related RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// Native paged group discovery via `circles_findGroups`.
    pub async fn find_groups_page(
        &self,
        limit: Option<u32>,
        params: Option<GroupQueryParams>,
        cursor: Option<&str>,
    ) -> Result<PagedResponse<GroupRow>> {
        if let Some(params_ref) = params.as_ref()
            && (params_ref.group_address_in.is_some()
                || params_ref.group_type_in.is_some()
                || params_ref.mint_handler_equals.is_some()
                || params_ref.treasury_equals.is_some())
        {
            return Err(CirclesRpcError::InvalidResponse {
                message: "circles_findGroups only supports name_starts_with, symbol_starts_with, and owner_in filters".into(),
            });
        }

        self.client
            .call(
                "circles_findGroups",
                (
                    limit.unwrap_or(DEFAULT_FIND_GROUPS_LIMIT),
                    params.map(NativeGroupQueryParams::from),
                    cursor.map(str::to_owned),
                ),
            )
            .await
    }

    /// Native paged group-member lookup via `circles_getGroupMembers`.
    pub async fn get_group_members_page(
        &self,
        group: Address,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedResponse<GroupMembershipRow>> {
        self.client
            .call(
                "circles_getGroupMembers",
                (
                    group,
                    limit.unwrap_or(DEFAULT_GROUP_MEMBERS_LIMIT),
                    cursor.map(str::to_owned),
                ),
            )
            .await
    }

    /// Native paged group-membership lookup via `circles_getGroupMemberships`.
    pub async fn get_group_memberships_page(
        &self,
        avatar: Address,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedResponse<GroupMembershipRow>> {
        self.client
            .call(
                "circles_getGroupMemberships",
                (
                    avatar,
                    limit.unwrap_or(DEFAULT_GROUP_MEMBERSHIPS_LIMIT),
                    cursor.map(str::to_owned),
                ),
            )
            .await
    }

    fn paged_query<TRow>(&self, params: PagedQueryParams) -> PagedQuery<TRow>
    where
        TRow: serde::de::DeserializeOwned
            + serde::Serialize
            + Clone
            + Send
            + Sync
            + std::fmt::Debug
            + Unpin
            + 'static,
    {
        let client = self.client.clone();
        let fetch: PagedFetch<TRow> = Arc::new(move |params: PagedQueryParams| {
            let client = client.clone();
            Box::pin(async move { QueryMethods::new(client).paged_query::<TRow>(params).await })
                as Pin<
                    Box<
                        dyn std::future::Future<Output = Result<circles_types::PagedResult<TRow>>>
                            + Send,
                    >,
                >
        });
        PagedQuery::new(fetch, params)
    }

    /// circles_getGroupMemberships
    pub async fn get_memberships(&self, avatar: Address) -> Result<Vec<GroupMembershipRow>> {
        let mut cursor: Option<String> = None;
        let mut rows = Vec::new();

        loop {
            let page = self
                .get_group_memberships_page(
                    avatar,
                    Some(DEFAULT_GROUP_MEMBERSHIPS_LIMIT),
                    cursor.as_deref(),
                )
                .await?;
            rows.extend(page.results);
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(rows)
    }

    /// Legacy "groups by owner" helper backed by paged `circles_findGroups`.
    pub async fn get_groups(&self, avatar: Address) -> Result<Vec<GroupRow>> {
        let mut cursor: Option<String> = None;
        let mut rows = Vec::new();

        loop {
            let page = self
                .find_groups_page(
                    Some(DEFAULT_FIND_GROUPS_LIMIT),
                    Some(GroupQueryParams {
                        owner_in: Some(vec![avatar]),
                        ..Default::default()
                    }),
                    cursor.as_deref(),
                )
                .await?;
            rows.extend(page.results);
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(rows)
    }

    /// Paged `GroupMemberships` query filtered by member address.
    pub fn get_group_memberships(
        &self,
        avatar: Address,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<GroupMembershipRow> {
        self.paged_query(PagedQueryParams {
            namespace: "V_CrcV2".into(),
            table: "GroupMemberships".into(),
            sort_order,
            columns: vec![
                "blockNumber".into(),
                "timestamp".into(),
                "transactionIndex".into(),
                "logIndex".into(),
                "transactionHash".into(),
                "group".into(),
                "member".into(),
                "expiryTime".into(),
            ],
            filter: Some(vec![
                FilterPredicate::equals("member".into(), format!("{avatar:#x}")).into(),
            ]),
            cursor_columns: None,
            order_columns: None,
            limit,
        })
    }

    /// Paged `GroupMemberships` query filtered by group address.
    pub fn get_group_members(
        &self,
        group: Address,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<GroupMembershipRow> {
        self.paged_query(PagedQueryParams {
            namespace: "V_CrcV2".into(),
            table: "GroupMemberships".into(),
            sort_order,
            columns: vec![
                "blockNumber".into(),
                "timestamp".into(),
                "transactionIndex".into(),
                "logIndex".into(),
                "transactionHash".into(),
                "group".into(),
                "member".into(),
                "expiryTime".into(),
            ],
            filter: Some(vec![
                FilterPredicate::equals("group".into(), format!("{group:#x}")).into(),
            ]),
            cursor_columns: None,
            order_columns: None,
            limit,
        })
    }

    /// Paged `GroupTokenHoldersBalance` query matching the TS group holders helper.
    pub fn get_group_holders(&self, group: Address, limit: u32) -> PagedQuery<GroupTokenHolderRow> {
        self.paged_query(PagedQueryParams {
            namespace: "V_CrcV2".into(),
            table: "GroupTokenHoldersBalance".into(),
            sort_order: SortOrder::DESC,
            columns: vec![
                "group".into(),
                "holder".into(),
                "totalBalance".into(),
                "demurragedTotalBalance".into(),
                "fractionOwnership".into(),
            ],
            filter: Some(vec![
                FilterPredicate::equals("group".into(), format!("{group:#x}")).into(),
            ]),
            cursor_columns: Some(vec![CursorColumn::asc("holder".into())]),
            order_columns: Some(vec![
                OrderBy::desc("totalBalance".into()),
                OrderBy::asc("holder".into()),
            ]),
            limit,
        })
    }

    /// Paged `Groups` query with optional filters matching the TS helper.
    pub fn get_groups_paged(
        &self,
        limit: u32,
        params: Option<GroupQueryParams>,
        sort_order: SortOrder,
    ) -> PagedQuery<GroupRow> {
        self.paged_query(PagedQueryParams {
            namespace: "V_CrcV2".into(),
            table: "Groups".into(),
            sort_order,
            columns: vec![
                "blockNumber".into(),
                "timestamp".into(),
                "transactionIndex".into(),
                "logIndex".into(),
                "transactionHash".into(),
                "group".into(),
                "type".into(),
                "owner".into(),
                "mintPolicy".into(),
                "mintHandler".into(),
                "treasury".into(),
                "service".into(),
                "feeCollection".into(),
                "memberCount".into(),
                "name".into(),
                "symbol".into(),
                "cidV0Digest".into(),
                "erc20WrapperDemurraged".into(),
                "erc20WrapperStatic".into(),
            ],
            filter: build_group_filters(params),
            cursor_columns: None,
            order_columns: None,
            limit,
        })
    }

    /// Fetch groups across pages until `limit` rows are collected.
    pub async fn find_groups(
        &self,
        limit: u32,
        params: Option<GroupQueryParams>,
    ) -> Result<Vec<GroupRow>> {
        let mut query = self.get_groups_paged(limit, params, SortOrder::DESC);
        let mut rows = Vec::new();

        while let Some(page) = query.next_page().await? {
            rows.extend(page.items);
            if rows.len() as u32 >= limit || !page.has_more {
                break;
            }
        }

        rows.truncate(limit as usize);
        Ok(rows)
    }
}

fn build_group_filters(params: Option<GroupQueryParams>) -> Option<Vec<Filter>> {
    let params = params?;

    let mut filters = Vec::new();

    if let Some(name_prefix) = params.name_starts_with {
        filters.push(FilterPredicate::like("name".into(), format!("{name_prefix}%")).into());
    }

    if let Some(symbol_prefix) = params.symbol_starts_with {
        filters.push(FilterPredicate::like("symbol".into(), format!("{symbol_prefix}%")).into());
    }

    if let Some(group_addresses) = params.group_address_in
        && !group_addresses.is_empty()
    {
        let predicates: Vec<Filter> = group_addresses
            .into_iter()
            .map(|addr| FilterPredicate::equals("group".into(), format!("{addr:#x}")).into())
            .collect();
        filters.push(if predicates.len() == 1 {
            predicates.into_iter().next().expect("one predicate")
        } else {
            Conjunction::or(predicates).into()
        });
    }

    if let Some(group_types) = params.group_type_in
        && !group_types.is_empty()
    {
        let predicates: Vec<Filter> = group_types
            .into_iter()
            .map(|group_type| FilterPredicate::equals("type".into(), group_type).into())
            .collect();
        filters.push(if predicates.len() == 1 {
            predicates.into_iter().next().expect("one predicate")
        } else {
            Conjunction::or(predicates).into()
        });
    }

    if let Some(owners) = params.owner_in
        && !owners.is_empty()
    {
        let predicates: Vec<Filter> = owners
            .into_iter()
            .map(|addr| FilterPredicate::equals("owner".into(), format!("{addr:#x}")).into())
            .collect();
        filters.push(if predicates.len() == 1 {
            predicates.into_iter().next().expect("one predicate")
        } else {
            Conjunction::or(predicates).into()
        });
    }

    if let Some(mint_handler) = params.mint_handler_equals {
        filters.push(
            FilterPredicate::equals("mintHandler".into(), format!("{mint_handler:#x}")).into(),
        );
    }

    if let Some(treasury) = params.treasury_equals {
        filters.push(FilterPredicate::equals("treasury".into(), format!("{treasury:#x}")).into());
    }

    if filters.len() > 1 {
        Some(vec![Conjunction::and(filters).into()])
    } else if filters.is_empty() {
        None
    } else {
        Some(filters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods() -> GroupMethods {
        let url = "https://rpc.example.com".parse().expect("valid url");
        GroupMethods::new(RpcClient::http(url))
    }

    #[test]
    fn group_memberships_query_uses_membership_table() {
        let query =
            methods().get_group_memberships(Address::repeat_byte(0x11), 50, SortOrder::DESC);

        assert_eq!(query.params.namespace, "V_CrcV2");
        assert_eq!(query.params.table, "GroupMemberships");
        assert_eq!(query.params.limit, 50);
    }

    #[test]
    fn group_filters_match_ts_shape() {
        let query = methods().get_groups_paged(
            25,
            Some(GroupQueryParams {
                name_starts_with: Some("Comm".into()),
                symbol_starts_with: None,
                group_address_in: Some(vec![
                    Address::repeat_byte(0x22),
                    Address::repeat_byte(0x33),
                ]),
                group_type_in: Some(vec!["Standard".into()]),
                owner_in: Some(vec![Address::repeat_byte(0x44)]),
                mint_handler_equals: Some(Address::repeat_byte(0x55)),
                treasury_equals: None,
            }),
            SortOrder::ASC,
        );

        let filters = query.params.filter.expect("filters");
        assert_eq!(filters.len(), 1);
        match &filters[0] {
            Filter::Conjunction(and) => {
                assert_eq!(and.predicates.len(), 5);
            }
            other => panic!("expected conjunction filter, got {other:?}"),
        }
    }

    #[test]
    fn group_holders_query_matches_ts_cursor_and_order_shape() {
        let query = methods().get_group_holders(Address::repeat_byte(0x66), 10);

        assert_eq!(query.params.table, "GroupTokenHoldersBalance");
        assert_eq!(query.params.sort_order, SortOrder::DESC);
        assert_eq!(
            query.params.cursor_columns,
            Some(vec![CursorColumn::asc("holder".into())])
        );
        assert_eq!(
            query.params.order_columns,
            Some(vec![
                OrderBy::desc("totalBalance".into()),
                OrderBy::asc("holder".into()),
            ])
        );
    }
}
