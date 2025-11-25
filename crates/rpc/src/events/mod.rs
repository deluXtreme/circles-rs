use crate::error::{CirclesRpcError, Result};
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub mod parser;
pub mod subscription;

/// Thin wrapper around subscription streams. Automatically maps transport errors
/// into `CirclesRpcError` and erases the underlying stream type.
pub struct EventStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T>> + Send>>,
}

impl<T> EventStream<T> {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<T>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Access the inner stream if needed.
    pub fn into_inner(self) -> Pin<Box<dyn Stream<Item = Result<T>> + Send>> {
        self.inner
    }
}

impl<T> Stream for EventStream<T> {
    type Item = Result<T>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(feature = "ws")]
impl<T> EventStream<T>
where
    T: alloy_json_rpc::RpcRecv + serde::de::DeserializeOwned + Send + 'static,
{
    /// Build an `EventStream` from an Alloy subscription handle. Returns the stream and subscription id.
    pub async fn from_subscription<P>(
        subscription: alloy_provider::GetSubscription<P, T>,
    ) -> Result<(Self, alloy_primitives::B256)>
    where
        P: alloy_json_rpc::RpcSend + 'static,
    {
        let subscription = subscription.await.map_err(CirclesRpcError::from)?;
        let id = *subscription.local_id();
        let stream = subscription
            .into_result_stream()
            .map(|res| res.map_err(CirclesRpcError::from));
        Ok((EventStream::new(stream), id))
    }
}
