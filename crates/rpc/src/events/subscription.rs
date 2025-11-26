use crate::error::{CirclesRpcError, Result};
use crate::events::EventStream;
use alloy_primitives::B256;
use alloy_provider::RootProvider;
use futures::Stream;

/// Wrapper around an [`EventStream`] that will best-effort `eth_unsubscribe` on drop.
pub struct CirclesSubscription<T> {
    stream: EventStream<T>,
    id: B256,
    provider: RootProvider,
}

impl<T> CirclesSubscription<T> {
    /// Construct a new subscription wrapper from a stream and subscription id.
    pub fn new(stream: EventStream<T>, id: B256, provider: RootProvider) -> Self {
        Self {
            stream,
            id,
            provider,
        }
    }

    /// Explicitly unsubscribe. Consumes the subscription.
    pub fn unsubscribe(self) -> Result<()> {
        self.provider
            .unsubscribe(self.id)
            .map_err(CirclesRpcError::from)
    }
}

impl<T> Drop for CirclesSubscription<T> {
    fn drop(&mut self) {
        let _ = self.provider.unsubscribe(self.id);
    }
}

impl<T> Stream for CirclesSubscription<T>
where
    EventStream<T>: Stream<Item = Result<T>>,
{
    type Item = Result<T>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.stream).poll_next(cx)
    }
}
