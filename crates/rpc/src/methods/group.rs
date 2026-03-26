use crate::client::RpcClient;
use crate::error::Result;
use crate::methods::QueryMethods;
use crate::paged_query::{PagedFetch, PagedQuery};
use circles_types::{
    Address, Conjunction, Filter, FilterPredicate, GroupMembershipRow, GroupQueryParams, GroupRow,
    PagedQueryParams, SortOrder,
};
use std::pin::Pin;
use std::sync::Arc;

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
        self.client
            .call("circles_getGroupMemberships", (avatar,))
            .await
    }

    /// circles_getGroups
    pub async fn get_groups(&self, avatar: Address) -> Result<Vec<GroupRow>> {
        self.client.call("circles_getGroups", (avatar,)).await
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
}
