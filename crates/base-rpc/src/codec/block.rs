use std::sync::Arc;

use alloy_consensus::{BlockHeader, TxReceipt};
use alloy_rlp::Encodable;
use reth_execution_types::Chain;
use reth_exex::ExExNotification;
use reth_primitives_traits::{Block as _, RecoveredBlock};
use reth_storage_api::{BlockReader, ReceiptProvider};

use crate::{
    codec::error::CodecError,
    exex::base::v1::{
        Block, BlockStreamResponse, Header as ProtoHeader, Log as ProtoLog,
        TransactionEnvelope as ProtoTransactionEnvelope,
        TransactionReceipt as ProtoTransactionReceipt,
    },
    BaseBlock, BasePrimitives, BaseReceipt,
};

pub fn notification_to_responses(
    notification: &ExExNotification<BasePrimitives>,
) -> Result<Vec<BlockStreamResponse>, CodecError> {
    match notification {
        ExExNotification::ChainCommitted { new } => chain_to_responses(new),
        ExExNotification::ChainReorged { old, new } => {
            let mut responses = chain_to_responses(old)?;
            responses.extend(chain_to_responses(new)?);
            Ok(responses)
        }
        ExExNotification::ChainReverted { old } => chain_to_responses(old),
    }
}

pub fn chain_to_responses(
    chain: &Chain<BasePrimitives>,
) -> Result<Vec<BlockStreamResponse>, CodecError> {
    chain
        .blocks_and_receipts()
        .map(|(block, receipts)| {
            let block = block_to_proto(block, receipts)?;
            Ok(BlockStreamResponse {
                block: ::buffa::MessageField::some(block),
                __buffa_unknown_fields: Default::default(),
            })
        })
        .collect()
}

pub fn block_to_proto(
    block: &RecoveredBlock<BaseBlock>,
    receipts: &[BaseReceipt],
) -> Result<Block, CodecError> {
    let header = header_to_proto(block.header())?;

    let transactions = block
        .body()
        .transactions
        .iter()
        .map(|tx| {
            let mut buf = Vec::new();
            tx.encode(&mut buf);
            ProtoTransactionEnvelope { rlp: buf.into(), __buffa_unknown_fields: Default::default() }
        })
        .collect();

    let transaction_receipts = receipts
        .iter()
        .map(|receipt| {
            let mut buf = Vec::new();
            receipt.encode(&mut buf);
            ProtoTransactionReceipt { rlp: buf.into(), __buffa_unknown_fields: Default::default() }
        })
        .collect();

    let logs = receipts
        .iter()
        .flat_map(|receipt| receipt.logs())
        .map(|log| ProtoLog {
            address: log.address.0.to_vec().into(),
            topics: log.topics().iter().map(|t| t.0.to_vec().into()).collect(),
            data: log.data.data.clone().into(),
            removed: false,
            __buffa_unknown_fields: Default::default(),
        })
        .collect();

    Ok(Block {
        number: block.number(),
        hash: block.hash().0.to_vec().into(),
        header: ::buffa::MessageField::some(header),
        transactions,
        transaction_receipts,
        logs,
        __buffa_unknown_fields: Default::default(),
    })
}

fn header_to_proto(header: &alloy_consensus::Header) -> Result<ProtoHeader, CodecError> {
    Ok(ProtoHeader {
        parent_hash: header.parent_hash().0.to_vec().into(),
        ommers_hash: header.ommers_hash().0.to_vec().into(),
        beneficiary: header.beneficiary().0.to_vec().into(),
        state_root: header.state_root().0.to_vec().into(),
        transactions_root: header.transactions_root().0.to_vec().into(),
        receipts_root: header.receipts_root().0.to_vec().into(),
        logs_bloom: header.logs_bloom().0.to_vec().into(),
        difficulty: alloy_primitives::U256::from(header.difficulty()).to_le_bytes_vec().into(),
        gas_limit: header.gas_limit(),
        gas_used: header.gas_used(),
        timestamp: header.timestamp(),
        extra_data: header.extra_data().to_vec().into(),
        mix_hash: header.mix_hash().unwrap_or_default().0.to_vec().into(),
        nonce: header.nonce().unwrap_or_default().0.to_vec().into(),
        base_fee_per_gas: header.base_fee_per_gas(),
        blob_gas_used: header.blob_gas_used(),
        excess_blob_gas: header.excess_blob_gas(),
        parent_beacon_block_root: header.parent_beacon_block_root().map(|h| h.0.to_vec().into()),
        requests_hash: header.requests_hash().map(|h| h.0.to_vec().into()),
        withdrawals_root: header.withdrawals_root().map(|h| h.0.to_vec().into()),
        __buffa_unknown_fields: Default::default(),
    })
}

pub async fn fetch_block_responses<P>(
    provider: &Arc<P>,
    start: u64,
    end: u64,
) -> Result<Vec<BlockStreamResponse>, CodecError>
where
    P: BlockReader<Block = BaseBlock, Receipt = BaseReceipt>
        + ReceiptProvider
        + Send
        + Sync
        + 'static,
{
    let provider = Arc::clone(provider);
    tokio::task::spawn_blocking(move || -> Result<Vec<BlockStreamResponse>, CodecError> {
        let mut responses = Vec::new();
        for number in start..=end {
            let block = provider
                .block_by_number(number)
                .map_err(|e| CodecError::Provider(e.to_string()))?
                .ok_or(CodecError::BlockNotFound(number))?;
            let senders = block.recover_signers().map_err(|e| {
                CodecError::Provider(format!("Failed to recover senders for block {number}: {e}"))
            })?;
            let recovered = RecoveredBlock::new_unhashed(block, senders);
            let receipts = provider
                .receipts_by_block(recovered.number().into())
                .map_err(|e| CodecError::Provider(e.to_string()))?
                .ok_or(CodecError::ReceiptNotFound(number))?;
            let proto_block = block_to_proto(&recovered, &receipts)?;
            responses.push(BlockStreamResponse {
                block: ::buffa::MessageField::some(proto_block),
                __buffa_unknown_fields: Default::default(),
            });
        }
        Ok(responses)
    })
    .await
    .map_err(|e| CodecError::Provider(e.to_string()))?
}
