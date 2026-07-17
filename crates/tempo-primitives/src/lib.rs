//! Tempo primitive types

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg), allow(unexpected_cfgs))]
#![allow(
    missing_docs,
    unexpected_cfgs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::missing_const_for_fn,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::semicolon_if_nothing_returned,
    clippy::elidable_lifetime_names,
    clippy::single_match_else,
    clippy::use_self,
    clippy::wildcard_imports,
    clippy::needless_continue,
    clippy::unused_async_trait_impl,
    clippy::missing_fields_in_debug,
    clippy::pub_underscore_fields,
    clippy::single_char_pattern,
    clippy::transmute_ptr_to_ptr,
    clippy::match_wildcard_for_single_variants,
    clippy::map_unwrap_or,
    clippy::if_then_some_else_none,
    clippy::or_fun_call,
    clippy::bool_to_int_with_if,
    clippy::if_not_else
)]

pub use alloy_consensus::Header;

mod address;
pub use address::{is_tip20_prefix, MasterId, TempoAddressExt, UserTag};
pub mod ed25519;

#[cfg(feature = "evm")]
mod block;
#[cfg(feature = "evm")]
pub use block::TempoBlockEnv;

pub mod transaction;
pub use transaction::{
    derive_p256_address, AASigned, SignatureType, TempoSignature, TempoTransaction,
    TempoTxEnvelope, TempoTxType, MAX_WEBAUTHN_SIGNATURE_LENGTH, P256_SIGNATURE_LENGTH,
    SECP256K1_SIGNATURE_LENGTH, TEMPO_GAS_PRICE_SCALING_FACTOR, TEMPO_TX_TYPE_ID,
};

mod header;
pub use header::{TempoConsensusContext, TempoHeader};

pub mod subblock;
pub use subblock::{
    RecoveredSubBlock, SignedSubBlock, SubBlock, SubBlockMetadata, SubBlockVersion,
};

extern crate alloc;

use once_cell as _;

/// Tempo block.
pub type Block = alloy_consensus::Block<TempoTxEnvelope, TempoHeader>;

/// Tempo block body.
pub type BlockBody = alloy_consensus::BlockBody<TempoTxEnvelope, TempoHeader>;

#[cfg(feature = "reth")]
mod reth_compat;

/// Tempo receipt.
/// Implements reth trait bounds when the `reth` feature is enabled.
#[cfg(feature = "reth")]
pub use reth_compat::TempoReceipt;
#[cfg(not(feature = "reth"))]
pub type TempoReceipt<L = alloy_primitives::Log> = alloy_consensus::EthereumReceipt<TempoTxType, L>;

/// Marker type for Tempo node primitives.
/// Implements [`reth_primitives_traits::NodePrimitives`] when the `reth` feature is enabled.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TempoPrimitives;
