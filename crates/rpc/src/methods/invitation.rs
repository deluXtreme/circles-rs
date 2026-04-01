use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{
    Address, AllInvitationsResponse, AtScaleInvitation, Balance, EscrowInvitation,
    InvitationOriginResponse, InvitationsFromResponse, TrustInvitation,
};
use futures::pin_mut;
use futures::stream::{self, StreamExt};

/// Methods for invitation discovery and balance lookups.
///
/// `get_invitations` fetches inviters then batches `circles_getInvitationBalance`
/// with bounded concurrency to avoid hammering the RPC.
#[derive(Clone, Debug)]
pub struct InvitationMethods {
    client: RpcClient,
}

/// Invitation + balance row returned by `get_invitations`.
#[derive(Debug, serde::Deserialize)]
pub struct InvitationRow {
    /// The address that sent the invitation.
    pub inviter: Address,
    /// The invitee address queried.
    pub invitee: Address,
    /// Balance available for the invitee from this inviter.
    pub invitation_balance: Balance,
}

impl InvitationMethods {
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// `circles_getInvitationOrigin` — reconstruct how an avatar joined Circles.
    pub async fn get_invitation_origin(
        &self,
        address: Address,
    ) -> Result<Option<InvitationOriginResponse>> {
        self.client
            .call("circles_getInvitationOrigin", (address,))
            .await
    }

    /// TS parity helper: return only the direct inviter address when present.
    pub async fn get_invited_by(&self, address: Address) -> Result<Option<Address>> {
        Ok(self
            .get_invitation_origin(address)
            .await?
            .and_then(|origin| origin.inviter))
    }

    /// `circles_getTrustInvitations` — trust-based invitations for this avatar.
    pub async fn get_trust_invitations(
        &self,
        address: Address,
        minimum_balance: Option<String>,
    ) -> Result<Vec<TrustInvitation>> {
        match minimum_balance {
            Some(minimum_balance) => {
                self.client
                    .call("circles_getTrustInvitations", (address, minimum_balance))
                    .await
            }
            None => {
                self.client
                    .call("circles_getTrustInvitations", (address,))
                    .await
            }
        }
    }

    /// `circles_getEscrowInvitations` — active escrow invitations for this avatar.
    pub async fn get_escrow_invitations(&self, address: Address) -> Result<Vec<EscrowInvitation>> {
        self.client
            .call("circles_getEscrowInvitations", (address,))
            .await
    }

    /// `circles_getAtScaleInvitations` — unclaimed at-scale invitations for this avatar.
    pub async fn get_at_scale_invitations(
        &self,
        address: Address,
    ) -> Result<Vec<AtScaleInvitation>> {
        self.client
            .call("circles_getAtScaleInvitations", (address,))
            .await
    }

    /// `circles_getAllInvitations` — return trust, escrow, and at-scale invitations.
    pub async fn get_all_invitations(
        &self,
        address: Address,
        minimum_balance: Option<String>,
    ) -> Result<AllInvitationsResponse> {
        match minimum_balance {
            Some(minimum_balance) => {
                self.client
                    .call("circles_getAllInvitations", (address, minimum_balance))
                    .await
            }
            None => {
                self.client
                    .call("circles_getAllInvitations", (address,))
                    .await
            }
        }
    }

    /// circles_getInvitations — batches balance lookups concurrently per invitee.
    pub async fn get_invitations(&self, invitee: Address) -> Result<Vec<InvitationRow>> {
        // Fetch inviters list first.
        let inviters: Vec<Address> = self
            .client
            .call("circles_getInvitations", (invitee,))
            .await?;

        // Concurrently fetch balances with bounded concurrency to avoid hammering the RPC.
        const MAX_CONCURRENT: usize = 10;
        let client = self.client.clone();
        let invitee_addr = invitee;
        let stream = stream::iter(inviters.into_iter().map(move |inviter| {
            let client = client.clone();
            async move {
                let bal: Balance = client
                    .call("circles_getInvitationBalance", (inviter, invitee_addr))
                    .await?;
                Ok::<_, crate::error::CirclesRpcError>(InvitationRow {
                    inviter,
                    invitee: invitee_addr,
                    invitation_balance: bal,
                })
            }
        }))
        .buffer_unordered(MAX_CONCURRENT);

        let mut rows = Vec::new();
        pin_mut!(stream);
        while let Some(res) = stream.next().await {
            rows.push(res?);
        }
        Ok(rows)
    }

    /// `circles_getInvitationsFrom` — accepted or pending invitees for an inviter.
    pub async fn get_invitations_from(
        &self,
        address: Address,
        accepted: bool,
    ) -> Result<InvitationsFromResponse> {
        self.client
            .call("circles_getInvitationsFrom", (address, accepted))
            .await
    }
}
