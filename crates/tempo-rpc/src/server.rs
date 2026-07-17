use std::sync::Arc;

use alloy_consensus::BlockHeader;
use async_stream::try_stream;
use connectrpc::{RequestContext, Response, ServiceRequest, ServiceStream};
use reth_exex::ExExNotification;
use reth_storage_api::{BlockReader, ReceiptProvider};
use tokio::sync::broadcast;

use crate::{
    codec::block::{fetch_block_responses, notification_to_responses},
    exex::tempo::v1::{
        BackfillRequest, BackfillToLiveRequest, BlockStream, BlockStreamResponse, LiveRequest,
    },
};

type TempoBlock = tempo_primitives::Block;
type TempoReceipt = tempo_primitives::TempoReceipt;
type TempoPrimitives = tempo_primitives::TempoPrimitives;

#[derive(Debug)]
pub struct BlockStreamService<P> {
    exex_notifications: broadcast::Sender<ExExNotification<TempoPrimitives>>,
    provider: Arc<P>,
}

impl<P> BlockStreamService<P>
where
    P: BlockReader<Block = TempoBlock, Receipt = TempoReceipt>
        + ReceiptProvider
        + Send
        + Sync
        + 'static,
{
    pub fn new(
        exex_notifications: broadcast::Sender<ExExNotification<TempoPrimitives>>,
        provider: Arc<P>,
    ) -> Self {
        Self { exex_notifications, provider }
    }
}

impl<P> BlockStream for BlockStreamService<P>
where
    P: BlockReader<Block = TempoBlock, Receipt = TempoReceipt>
        + ReceiptProvider
        + Send
        + Sync
        + 'static,
{
    async fn live(
        &self,
        _ctx: RequestContext,
        _request: ServiceRequest<'_, LiveRequest>,
    ) -> connectrpc::ServiceResult<
        ServiceStream<impl connectrpc::Encodable<BlockStreamResponse> + Send + use<P>>,
    > {
        let rx = self.exex_notifications.subscribe();
        let stream = try_stream! {
            let mut rx = rx;
            loop {
                match rx.recv().await {
                    Ok(notification) => {
                        let responses = notification_to_responses(&notification)?;
                        for response in responses {
                            yield response;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("tempo-rpc live stream lagged by {n} messages");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        };
        Response::stream_ok(Box::pin(stream))
    }

    async fn backfill(
        &self,
        _ctx: RequestContext,
        request: ServiceRequest<'_, BackfillRequest>,
    ) -> connectrpc::ServiceResult<
        ServiceStream<impl connectrpc::Encodable<BlockStreamResponse> + Send + use<P>>,
    > {
        let req = request.to_owned_message();
        if req.start_block > req.end_block {
            return Err(connectrpc::ConnectError::new(
                connectrpc::ErrorCode::InvalidArgument,
                format!(
                    "start_block ({}) must be <= end_block ({})",
                    req.start_block, req.end_block
                ),
            ));
        }

        let provider = Arc::clone(&self.provider);
        let start = req.start_block;
        let end = req.end_block;

        let stream = try_stream! {
            let responses = fetch_block_responses(&provider, start, end).await?;
            for response in responses {
                yield response;
            }
        };
        Response::stream_ok(Box::pin(stream))
    }

    #[allow(unused_assignments)]
    async fn backfill_to_live(
        &self,
        _ctx: RequestContext,
        request: ServiceRequest<'_, BackfillToLiveRequest>,
    ) -> connectrpc::ServiceResult<
        ServiceStream<impl connectrpc::Encodable<BlockStreamResponse> + Send + use<P>>,
    > {
        let req = request.to_owned_message();
        let rx = self.exex_notifications.subscribe();
        let provider = Arc::clone(&self.provider);
        let start_block = req.start_block;

        let stream = try_stream! {
            let mut rx = rx;
            let mut current_start = start_block;

            loop {
                match rx.recv().await {
                    Ok(notification) => {
                        if let Some(committed) = notification.committed_chain() {
                            let tip = committed.tip().number();

                            if current_start <= tip {
                                let responses = fetch_block_responses(&provider, current_start, tip).await?;
                                for response in responses {
                                    yield response;
                                }
                                current_start = tip + 1;
                            }

                            let live_responses = notification_to_responses(&notification)?;
                            for response in live_responses {
                                yield response;
                            }

                            loop {
                                match rx.recv().await {
                                    Ok(notif) => {
                                        let responses = notification_to_responses(&notif)?;
                                        for response in responses {
                                            yield response;
                                        }
                                    }
                                    Err(broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("tempo-rpc backfill_to_live stream lagged by {n} messages");
                                        continue;
                                    }
                                    Err(broadcast::error::RecvError::Closed) => {
                                        break;
                                    }
                                }
                            }
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        Response::stream_ok(Box::pin(stream))
    }
}
