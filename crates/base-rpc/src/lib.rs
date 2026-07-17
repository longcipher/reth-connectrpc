#![allow(missing_docs, missing_debug_implementations, elided_lifetimes_in_paths, clippy::all)]

pub mod codec;
pub mod server;

connectrpc::include_generated!();

pub use base_common_consensus::{BaseBlock, BasePrimitives, BaseReceipt};
