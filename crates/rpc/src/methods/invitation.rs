use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, Balance};
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
}
