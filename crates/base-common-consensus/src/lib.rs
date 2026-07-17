#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://avatars.githubusercontent.com/u/16627100?s=200&v=4",
    html_favicon_url = "https://avatars.githubusercontent.com/u/16627100?s=200&v=4",
    issue_tracker_base_url = "https://github.com/base/base/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    missing_docs,
    unexpected_cfgs,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unimplemented,
    clippy::allow_attributes,
    clippy::missing_const_for_fn,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::semicolon_if_nothing_returned,
    clippy::unnecessary_semicolon,
    clippy::too_many_arguments,
    clippy::result_large_err
)]

extern crate alloc;

#[cfg(feature = "evm")]
use revm as _;

#[cfg(feature = "reth")]
mod reth_compat;
#[cfg(feature = "reth")]
pub use reth_compat::{BaseBlockBody, BasePrimitives, CompactTxDeposit, DepositReceiptExt};

mod receipts;
pub use receipts::{
    BaseReceipt, BaseReceiptEnvelope, BaseTxReceipt, DepositReceipt, DepositReceiptWithBloom,
    Eip8130Receipt,
};

mod transaction;
#[cfg(feature = "serde")]
pub use transaction::serde_deposit_tx_rpc;
pub use transaction::{
    AccountChange, ActorChange, ActorChangeType, BasePooledTransaction, BaseTransaction,
    BaseTransactionInfo, BaseTxEnvelope, BaseTypedTransaction, Call, ConfigChange, CreateEntry,
    Delegation, DepositInfo, DepositTransaction, Eip8130Constants, Eip8130Contracts, Eip8130Signed,
    InitialActor, OpTxType, Scope, TxDeposit, TxEip8130, DEPOSIT_TX_TYPE_ID, EIP8130_REJECTION_MSG,
    EIP8130_TX_TYPE_ID,
};

mod extra;
pub use extra::{EIP1559ParamEncoder, EIP1559ParamError, HoloceneExtraData, JovianExtraData};

mod source;
pub use source::{
    BaseTimeDepositSource, DepositSourceDomain, DepositSourceDomainIdentifier, L1InfoDepositSource,
    UpgradeDepositSource, UserDepositSource,
};

mod predeploys;
pub use predeploys::{Deployers, Predeploys, SystemAddresses};

mod block;
pub use block::BaseBlock;

/// Signed transaction type alias for [`BaseTxEnvelope`].
pub type BaseTransactionSigned = BaseTxEnvelope;

/// Bincode-compatible serde implementations for consensus types.
///
/// `bincode` crate doesn't work well with optionally serializable serde fields, but some of the
/// consensus types require optional serialization for RPC compatibility. This module makes so that
/// all fields are serialized.
///
/// Read more: <https://github.com/bincode-org/bincode/issues/326>
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub mod serde_bincode_compat {
    pub use super::{
        receipts::serde_bincode_compat::{BaseReceipt, DepositReceipt},
        transaction::serde_bincode_compat::TxDeposit,
    };

    /// Bincode-compatible serde implementations for transaction types.
    pub mod transaction {
        pub use crate::transaction::serde_bincode_compat::*;
    }
}

/// Wincode schemas for testing serialization roundtrips.
#[cfg(test)]
pub(crate) mod wincode_schemas {
    use alloy_consensus::Receipt;
    use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
    use alloy_primitives::{Address, Log, Signature, TxKind, B256, U256};
    use wincode::{SchemaRead, SchemaWrite};

    use crate::{
        AccountChange, ActorChange, ActorChangeType, BaseReceipt, BaseTxEnvelope, Call,
        ConfigChange, CreateEntry, Delegation, DepositReceipt, Eip8130Signed, InitialActor,
        TxDeposit, TxEip8130,
    };

    fn addr(a: Address) -> [u8; 20] {
        a.0 .0
    }
    fn hash(b: B256) -> [u8; 32] {
        b.0
    }
    fn u256b(v: U256) -> [u8; 32] {
        v.to_be_bytes()
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) enum TxKindSchema {
        Call([u8; 20]),
        Create,
    }

    impl From<TxKind> for TxKindSchema {
        fn from(value: TxKind) -> Self {
            match value {
                TxKind::Call(a) => Self::Call(addr(a)),
                TxKind::Create => Self::Create,
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct LogDataSchema {
        pub topics: Vec<[u8; 32]>,
        pub data: Vec<u8>,
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct LogSchema {
        pub address: [u8; 20],
        pub data: LogDataSchema,
    }

    impl From<&Log> for LogSchema {
        fn from(value: &Log) -> Self {
            Self {
                address: addr(value.address),
                data: LogDataSchema {
                    topics: value.data.topics().iter().map(|t| hash(*t)).collect(),
                    data: value.data.data.to_vec(),
                },
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct ReceiptSchema {
        pub status: bool,
        pub cumulative_gas_used: u64,
        pub logs: Vec<LogSchema>,
    }

    impl From<&Receipt<Log>> for ReceiptSchema {
        fn from(value: &Receipt<Log>) -> Self {
            Self {
                status: value.status.coerce_status(),
                cumulative_gas_used: value.cumulative_gas_used,
                logs: value.logs.iter().map(LogSchema::from).collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct DepositReceiptSchema {
        pub inner: ReceiptSchema,
        pub deposit_nonce: Option<u64>,
        pub deposit_receipt_version: Option<u64>,
    }

    impl From<&DepositReceipt<Log>> for DepositReceiptSchema {
        fn from(value: &DepositReceipt<Log>) -> Self {
            Self {
                inner: (&value.inner).into(),
                deposit_nonce: value.deposit_nonce,
                deposit_receipt_version: value.deposit_receipt_version,
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct Eip8130ReceiptSchema {
        pub inner: ReceiptSchema,
        pub phase_statuses: Vec<u8>,
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) enum BaseReceiptSchema {
        Legacy(ReceiptSchema),
        Eip2930(ReceiptSchema),
        Eip1559(ReceiptSchema),
        Eip7702(ReceiptSchema),
        Deposit(DepositReceiptSchema),
        Eip8130(Eip8130ReceiptSchema),
    }

    impl From<&BaseReceipt> for BaseReceiptSchema {
        fn from(value: &BaseReceipt) -> Self {
            match value {
                BaseReceipt::Legacy(r) => Self::Legacy(r.into()),
                BaseReceipt::Eip2930(r) => Self::Eip2930(r.into()),
                BaseReceipt::Eip1559(r) => Self::Eip1559(r.into()),
                BaseReceipt::Eip7702(r) => Self::Eip7702(r.into()),
                BaseReceipt::Deposit(r) => Self::Deposit(r.into()),
                BaseReceipt::Eip8130(r) => Self::Eip8130(Eip8130ReceiptSchema {
                    inner: (&r.inner).into(),
                    phase_statuses: r.phase_statuses.clone(),
                }),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxDepositSchema {
        pub source_hash: [u8; 32],
        pub from: [u8; 20],
        pub to: TxKindSchema,
        pub mint: u128,
        pub value: [u8; 32],
        pub gas_limit: u64,
        pub is_system_transaction: bool,
        pub input: Vec<u8>,
    }

    impl From<&TxDeposit> for TxDepositSchema {
        fn from(value: &TxDeposit) -> Self {
            Self {
                source_hash: hash(value.source_hash),
                from: addr(value.from),
                to: value.to.into(),
                mint: value.mint,
                value: u256b(value.value),
                gas_limit: value.gas_limit,
                is_system_transaction: value.is_system_transaction,
                input: value.input.to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct AccessListItemSchema {
        pub address: [u8; 20],
        pub storage_keys: Vec<[u8; 32]>,
    }

    impl From<&AccessListItem> for AccessListItemSchema {
        fn from(value: &AccessListItem) -> Self {
            Self {
                address: addr(value.address),
                storage_keys: value.storage_keys.iter().map(|k| hash(*k)).collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct SignedAuthorizationSchema {
        pub chain_id: [u8; 32],
        pub address: [u8; 20],
        pub nonce: u64,
        pub y_parity: bool,
        pub r: [u8; 32],
        pub s: [u8; 32],
    }

    impl From<&SignedAuthorization> for SignedAuthorizationSchema {
        fn from(value: &SignedAuthorization) -> Self {
            Self {
                chain_id: u256b(value.chain_id),
                address: addr(value.address),
                nonce: value.nonce,
                y_parity: value.y_parity() != 0,
                r: u256b(value.r()),
                s: u256b(value.s()),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct SignatureSchema {
        pub r: [u8; 32],
        pub s: [u8; 32],
        pub v: bool,
    }

    impl From<Signature> for SignatureSchema {
        fn from(sig: Signature) -> Self {
            Self { r: u256b(sig.r()), s: u256b(sig.s()), v: sig.v() }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxLegacySchema {
        pub chain_id: Option<u64>,
        pub nonce: u64,
        pub gas_price: u128,
        pub gas_limit: u64,
        pub to: TxKindSchema,
        pub value: [u8; 32],
        pub input: Vec<u8>,
    }

    impl From<&alloy_consensus::TxLegacy> for TxLegacySchema {
        fn from(v: &alloy_consensus::TxLegacy) -> Self {
            Self {
                chain_id: v.chain_id,
                nonce: v.nonce,
                gas_price: v.gas_price,
                gas_limit: v.gas_limit,
                to: v.to.into(),
                value: u256b(v.value),
                input: v.input.to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxEip2930Schema {
        pub chain_id: u64,
        pub nonce: u64,
        pub gas_price: u128,
        pub gas_limit: u64,
        pub to: TxKindSchema,
        pub value: [u8; 32],
        pub input: Vec<u8>,
        pub access_list: Vec<AccessListItemSchema>,
    }

    impl From<&alloy_consensus::TxEip2930> for TxEip2930Schema {
        fn from(v: &alloy_consensus::TxEip2930) -> Self {
            Self {
                chain_id: v.chain_id,
                nonce: v.nonce,
                gas_price: v.gas_price,
                gas_limit: v.gas_limit,
                to: v.to.into(),
                value: u256b(v.value),
                input: v.input.to_vec(),
                access_list: v.access_list.iter().map(AccessListItemSchema::from).collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxEip1559Schema {
        pub chain_id: u64,
        pub nonce: u64,
        pub max_priority_fee_per_gas: u128,
        pub max_fee_per_gas: u128,
        pub gas_limit: u64,
        pub to: TxKindSchema,
        pub value: [u8; 32],
        pub input: Vec<u8>,
        pub access_list: Vec<AccessListItemSchema>,
    }

    impl From<&alloy_consensus::TxEip1559> for TxEip1559Schema {
        fn from(v: &alloy_consensus::TxEip1559) -> Self {
            Self {
                chain_id: v.chain_id,
                nonce: v.nonce,
                max_priority_fee_per_gas: v.max_priority_fee_per_gas,
                max_fee_per_gas: v.max_fee_per_gas,
                gas_limit: v.gas_limit,
                to: v.to.into(),
                value: u256b(v.value),
                input: v.input.to_vec(),
                access_list: v.access_list.iter().map(AccessListItemSchema::from).collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxEip7702Schema {
        pub chain_id: u64,
        pub nonce: u64,
        pub max_priority_fee_per_gas: u128,
        pub max_fee_per_gas: u128,
        pub gas_limit: u64,
        pub to: [u8; 20],
        pub value: [u8; 32],
        pub input: Vec<u8>,
        pub access_list: Vec<AccessListItemSchema>,
        pub authorization_list: Vec<SignedAuthorizationSchema>,
    }

    impl From<&alloy_consensus::TxEip7702> for TxEip7702Schema {
        fn from(v: &alloy_consensus::TxEip7702) -> Self {
            Self {
                chain_id: v.chain_id,
                nonce: v.nonce,
                max_priority_fee_per_gas: v.max_priority_fee_per_gas,
                max_fee_per_gas: v.max_fee_per_gas,
                gas_limit: v.gas_limit,
                to: addr(v.to),
                value: u256b(v.value),
                input: v.input.to_vec(),
                access_list: v.access_list.iter().map(AccessListItemSchema::from).collect(),
                authorization_list: v
                    .authorization_list
                    .iter()
                    .map(SignedAuthorizationSchema::from)
                    .collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct CallSchema {
        pub to: [u8; 20],
        pub data: Vec<u8>,
    }

    impl From<&Call> for CallSchema {
        fn from(v: &Call) -> Self {
            Self { to: addr(v.to), data: v.data.to_vec() }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct InitialActorSchema {
        pub actor_id: [u8; 32],
        pub authenticator: [u8; 20],
        pub scope: u8,
        pub policy_data: Vec<u8>,
    }

    impl From<&InitialActor> for InitialActorSchema {
        fn from(v: &InitialActor) -> Self {
            Self {
                actor_id: hash(v.actor_id),
                authenticator: addr(v.authenticator),
                scope: v.scope,
                policy_data: v.policy_data.to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) enum ActorChangeTypeSchema {
        Authorize,
        Revoke,
    }

    impl From<ActorChangeType> for ActorChangeTypeSchema {
        fn from(value: ActorChangeType) -> Self {
            match value {
                ActorChangeType::Authorize => Self::Authorize,
                ActorChangeType::Revoke => Self::Revoke,
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct ActorChangeSchema {
        pub change_type: ActorChangeTypeSchema,
        pub actor_id: [u8; 32],
        pub data: Vec<u8>,
    }

    impl From<&ActorChange> for ActorChangeSchema {
        fn from(v: &ActorChange) -> Self {
            Self {
                change_type: v.change_type.into(),
                actor_id: hash(v.actor_id),
                data: v.data.to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct CreateEntrySchema {
        pub user_salt: [u8; 32],
        pub code: Vec<u8>,
        pub initial_actors: Vec<InitialActorSchema>,
    }

    impl From<&CreateEntry> for CreateEntrySchema {
        fn from(v: &CreateEntry) -> Self {
            Self {
                user_salt: hash(v.user_salt),
                code: v.code.to_vec(),
                initial_actors: v.initial_actors.iter().map(InitialActorSchema::from).collect(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct ConfigChangeSchema {
        pub chain_id: u64,
        pub sequence: u64,
        pub actor_changes: Vec<ActorChangeSchema>,
        pub auth: Vec<u8>,
    }

    impl From<&ConfigChange> for ConfigChangeSchema {
        fn from(v: &ConfigChange) -> Self {
            Self {
                chain_id: v.chain_id,
                sequence: v.sequence,
                actor_changes: v.actor_changes.iter().map(ActorChangeSchema::from).collect(),
                auth: v.auth.to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct DelegationSchema {
        pub target: [u8; 20],
    }

    impl From<&Delegation> for DelegationSchema {
        fn from(v: &Delegation) -> Self {
            Self { target: addr(v.target) }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) enum AccountChangeSchema {
        Create(CreateEntrySchema),
        ConfigChange(ConfigChangeSchema),
        Delegation(DelegationSchema),
    }

    impl From<&AccountChange> for AccountChangeSchema {
        fn from(value: &AccountChange) -> Self {
            match value {
                AccountChange::Create(e) => Self::Create(e.into()),
                AccountChange::ConfigChange(e) => Self::ConfigChange(e.into()),
                AccountChange::Delegation(e) => Self::Delegation(e.into()),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct TxEip8130Schema {
        pub chain_id: u64,
        pub sender: Option<[u8; 20]>,
        pub nonce_key: [u8; 32],
        pub nonce_sequence: u64,
        pub expiry: u64,
        pub max_priority_fee_per_gas: u128,
        pub max_fee_per_gas: u128,
        pub gas_limit: u64,
        pub account_changes: Vec<AccountChangeSchema>,
        pub calls: Vec<Vec<CallSchema>>,
        pub metadata: Vec<u8>,
        pub payer: Option<[u8; 20]>,
    }

    impl From<&TxEip8130> for TxEip8130Schema {
        fn from(v: &TxEip8130) -> Self {
            Self {
                chain_id: v.chain_id,
                sender: v.sender.map(addr),
                nonce_key: u256b(v.nonce_key),
                nonce_sequence: v.nonce_sequence,
                expiry: v.expiry,
                max_priority_fee_per_gas: v.max_priority_fee_per_gas,
                max_fee_per_gas: v.max_fee_per_gas,
                gas_limit: v.gas_limit,
                account_changes: v.account_changes.iter().map(AccountChangeSchema::from).collect(),
                calls: v
                    .calls
                    .iter()
                    .map(|phase| phase.iter().map(CallSchema::from).collect())
                    .collect(),
                metadata: v.metadata.to_vec(),
                payer: v.payer.map(addr),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) struct Eip8130SignedSchema {
        pub tx: TxEip8130Schema,
        pub sender_auth: Vec<u8>,
        pub payer_auth: Vec<u8>,
    }

    impl From<&Eip8130Signed> for Eip8130SignedSchema {
        fn from(v: &Eip8130Signed) -> Self {
            Self {
                tx: v.tx().into(),
                sender_auth: v.sender_auth().to_vec(),
                payer_auth: v.payer_auth().to_vec(),
            }
        }
    }

    #[derive(SchemaWrite, SchemaRead)]
    pub(crate) enum BaseTxEnvelopeSchema {
        Legacy { signature: SignatureSchema, transaction: TxLegacySchema },
        Eip2930 { signature: SignatureSchema, transaction: TxEip2930Schema },
        Eip1559 { signature: SignatureSchema, transaction: TxEip1559Schema },
        Eip7702 { signature: SignatureSchema, transaction: TxEip7702Schema },
        Deposit { hash: [u8; 32], transaction: TxDepositSchema },
        Eip8130 { transaction: Eip8130SignedSchema },
    }

    impl From<&BaseTxEnvelope> for BaseTxEnvelopeSchema {
        fn from(value: &BaseTxEnvelope) -> Self {
            match value {
                BaseTxEnvelope::Legacy(s) => {
                    Self::Legacy { signature: (*s.signature()).into(), transaction: s.tx().into() }
                }
                BaseTxEnvelope::Eip2930(s) => {
                    Self::Eip2930 { signature: (*s.signature()).into(), transaction: s.tx().into() }
                }
                BaseTxEnvelope::Eip1559(s) => {
                    Self::Eip1559 { signature: (*s.signature()).into(), transaction: s.tx().into() }
                }
                BaseTxEnvelope::Eip7702(s) => {
                    Self::Eip7702 { signature: (*s.signature()).into(), transaction: s.tx().into() }
                }
                BaseTxEnvelope::Deposit(s) => {
                    Self::Deposit { hash: hash(s.seal()), transaction: s.inner().into() }
                }
                BaseTxEnvelope::Eip8130(s) => Self::Eip8130 { transaction: s.into() },
            }
        }
    }
}
