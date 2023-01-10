// Copyright (C) 2022-2023 Polkadot Smart Chain (PSC).
// This file is part of PSC.

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # Polkadot Smart Chain Runtime
//!
//! Polkadot Smart Chain is a parachain that provides an interface to create, manage, and use
//! assets. Assets are fungible.
//!
//! ## Assets
//!
//! - Fungibles: Configuration of `pallet-assets`.
//!
//! ## Other Functionality
//!
//! ### Native Balances
//!
//! Polkadot Smart Chain uses its parent DOT token as its native asset.
//!
//! ### Governance
//!
//! Polkadot Smart Chain Sudo
//!
//! ### Collator Selection
//!
//! Polkadot Smart Chain uses `pallet-collator-selection`, a simple first-come-first-served
//! registration system where collators can reserve a small bond to join the block producer set.
//! There is no slashing.
//!
//! ### XCM
//!
//! Polkadot Smart Chain can also serve as a reserve location to other parachains for DOT as well
//! as other local assets.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

// evm
use codec::{Decode, Encode};
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use fp_rpc::TransactionStatus;
use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    parameter_types,
    traits::{EitherOfDiverse, Get},
    weights::{constants::WEIGHT_PER_SECOND, ConstantMultiplier, Weight},
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot,
};
use pallet_ethereum::{Call::transact, Transaction as EthereumTransaction};
use pallet_evm::{
    Account as EVMAccount, EnsureAddressNever, EnsureAddressRoot, FeeCalculator,
    HashedAddressMapping, Runner,
};

// Polkadot imports
use pallet_xcm::{EnsureXcm, IsMajorityOfBody};
use polkadot_runtime_common::{BlockHashCount, Bounded};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdLookup, BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable,
        PostDispatchInfoOf, UniqueSaturatedInto,
    },
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult, FixedPointNumber, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm::latest::BodyId;
use xcm_executor::XcmExecutor;

use constants::{currency::*, fee::WeightToFee};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use precompiles::PscPrecompiles;
pub use psc_common as common;
use psc_common::{
    impls::{DealWithFees, ToStakingPot},
    opaque, AccountId, AssetId, AuraId, Balance, BlockNumber, Hash, Header, Index, Signature,
    AVERAGE_ON_INITIALIZE_RATIO, HOURS, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO, SLOT_DURATION,
};
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
use xcm_config::{DotLocation, XcmConfig, XcmOriginToTransactDispatchOrigin};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
mod precompiles;
mod weights;
pub mod xcm_config;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("psc"),
    impl_name: create_runtime_str!("psc"),
    authoring_version: 1,
    spec_version: 2,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
            .base_block(BlockExecutionWeight::get())
            .for_class(DispatchClass::all(), |weights| {
                weights.base_extrinsic = ExtrinsicBaseWeight::get();
            })
            .for_class(DispatchClass::Normal, |weights| {
                weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
            })
            .for_class(DispatchClass::Operational, |weights| {
                weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
                // Operational transactions have some extra reserved space, so that they
                // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
                weights.reserved = Some(
                    MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
                );
            })
            .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
            .build_or_panic();
    pub const SS58Prefix: u8 = 0;
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type AccountId = AccountId;
    type RuntimeCall = RuntimeCall;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
    type Version = Version;
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
    pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ToStakingPot<Runtime>;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = MILLICENTS;
    pub const OperationalFeeMultiplier: u8 = 5;

    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(90, 100);
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
    R,
    TargetBlockFullness,
    AdjustmentVariable,
    MinimumMultiplier,
    MaximumMultiplier,
>;

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction =
        pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees<Runtime>>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
    pub const AssetDeposit: Balance = 10 * UNITS; // 10 UNITS deposit to create fungible asset class
    pub const AssetAccountDeposit: Balance = deposit(1, 16);
    pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const AssetsStringLimit: u32 = 50;
    /// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
    // https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
    pub const ExecutiveBody: BodyId = BodyId::Executive;
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = AssetsStringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
    type AssetAccountDeposit = AssetAccountDeposit;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
    pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but lets be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const MaxCandidates: u32 = 1000;
    pub const MinCandidates: u32 = 5;
    pub const SessionLength: BlockNumber = 6 * HOURS;
    pub const MaxInvulnerables: u32 = 100;
}

/// We allow root and the Relay Chain council to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin =
    EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<DotLocation, ExecutiveBody>>>;

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = PotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
}

parameter_types! {
    // 0x1111111111111111111111111111111111111111
    pub EvmCaller: H160 = H160::from_slice(&[17u8;20][..]);
    pub ClaimBond: Balance = 10 * EXISTENTIAL_DEPOSIT;
}
impl pallet_assets_bridge::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type EvmCaller = EvmCaller;
    type ClaimBond = ClaimBond;
}

impl pallet_ethereum_chain_id::Config for Runtime {}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_PER_SECOND.ref_time() / GAS_PER_SECOND;

parameter_types! {
    pub BlockGasLimit: U256
        = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
    pub PrecompilesValue: PscPrecompiles<Runtime> = PscPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_ref_time(WEIGHT_PER_GAS);
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = BaseFee;
    type WeightPerGas = WeightPerGas;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = PscPrecompiles<Runtime>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = EthereumChainId;
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, DealWithFees<Runtime>>;
    type BlockGasLimit = BlockGasLimit;
    type FindAuthor = ();
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(220_000_000_000u128);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;

impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
    type DefaultElasticity = DefaultElasticity;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
}

impl pallet_hotfix_sufficients::Config for Runtime {
    type AddressMapping = HashedAddressMapping<BlakeTwo256>;
    type WeightInfo = pallet_hotfix_sufficients::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
       pub enum Runtime where
            Block = Block,
            NodeBlock = opaque::Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
       {
            // System support stuff.
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
            ParachainSystem: cumulus_pallet_parachain_system::{
                Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned,
            } = 1,
            // RandomnessCollectiveFlip = 2 removed
            Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
            ParachainInfo: parachain_info::{Pallet, Storage, Config} = 4,
            // Sudo.
            Sudo: pallet_sudo::{Pallet, Call, Storage, Event<T>, Config<T>} = 5,

            // Monetary stuff.
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
            TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 11,
            Assets: pallet_assets::{Pallet, Call, Storage, Config<T>, Event<T>} = 12,

            // Collator support. the order of these 5 are important and shall not change.
            Authorship: pallet_authorship::{Pallet, Call, Storage} = 20,
            CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 21,
            Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
            Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
            AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 24,

            // XCM helpers.
            XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
            PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config} = 31,
            CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 32,
            DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 33,

            // Handy utilities.
            Utility: pallet_utility::{Pallet, Call, Event} = 40,
            Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 41,

            // Ethereum compatibility
            EthereumChainId: pallet_ethereum_chain_id::{Pallet, Call, Storage, Config} = 50,
            EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 51,
            Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin} = 52,
            AssetsBridge: pallet_assets_bridge::{Pallet, Call, Storage, Config<T>, Event<T>} = 53,
            BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 54,
            HotfixSufficients: pallet_hotfix_sufficients::{Pallet, Call} = 55,
       }
);

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        )
    }
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> opaque::UncheckedExtrinsic {
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        );
        let encoded = extrinsic.encode();
        opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) =>
                call.pre_dispatch_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) =>
                Some(call.dispatch(RuntimeOrigin::from(
                    pallet_ethereum::RawOrigin::EthereumTransaction(info),
                ))),
            _ => None,
        }
    }
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
    fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_assets, Assets]
        [pallet_balances, Balances]
        [pallet_multisig, Multisig]
        [pallet_session, SessionBench::<Runtime>]
        [pallet_utility, Utility]
        [pallet_timestamp, Timestamp]
        [pallet_collator_selection, CollatorSelection]
        [cumulus_pallet_xcmp_queue, XcmpQueue]
        // XCM
        // NOTE: Make sure you point to the individual modules below.
        [pallet_xcm_benchmarks::fungible, XcmBalances]
        [pallet_xcm_benchmarks::generic, XcmGeneric]
        [pallet_evm, EVM]
        [pallet_ethereum, Ethereum]
        [pallet_hotfix_sufficients]
    );
}

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> EVMAccount {
            let (account, _) = EVM::account_basic(&address);
            account
        }

        fn gas_price() -> U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            EVM::account_codes(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            EVM::account_storages(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let is_transactional = false;
            let validate = true;

            let evm_config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                config
            } else {
                <Runtime as pallet_evm::Config>::config().clone()
            };

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                &evm_config,
            ).map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let is_transactional = false;
            let validate = true;
            let evm_config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                config
            } else {
                <Runtime as pallet_evm::Config>::config().clone()
            };

            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                &evm_config,
            ).map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
            Ethereum::current_transaction_statuses()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            Ethereum::current_block()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            Ethereum::current_receipts()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<TransactionStatus>>
        ) {
            (
                Ethereum::current_block(),
                Ethereum::current_receipts(),
                Ethereum::current_transaction_statuses()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<EthereumTransaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<EthereumTransaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(BaseFee::elasticity())
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
                  SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade() -> (Weight, Weight) {
            log::info!("try-runtime::on_runtime_upgrade psc.");
            let weight = Executive::try_runtime_upgrade().unwrap();
                (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(block: Block, state_root_check: bool, select: frame_try_runtime::TryStateSelect) -> Weight {
            log::info!(
                target: "runtime::psc", "try-runtime: executing block #{} ({:?}) / root checks: {:?} / sanity-checks: {:?}",
                block.header.number,
                block.header.hash(),
                state_root_check,
                select,
            );
            Executive::try_execute_block(block, state_root_check, select).expect("try_execute_block failed")
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

            // This is defined once again in dispatch_benchmark, because list_benchmarks!
            // and add_benchmarks! are macros exported by define_benchmarks! macros and those types
            // are referenced in that call.
            type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet::<Runtime>;
            type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet::<Runtime>;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();
            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey, BenchmarkError};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            impl cumulus_pallet_session_benchmarking::Config for Runtime {}

            use xcm::latest::prelude::*;
            use xcm_config::DotLocation;
            use pallet_xcm_benchmarks::asset_instance_from;

            impl pallet_xcm_benchmarks::Config for Runtime {
                type XcmConfig = xcm_config::XcmConfig;
                type AccountIdConverter = xcm_config::LocationToAccountId;
                fn valid_destination() -> Result<MultiLocation, BenchmarkError> {
                    Ok(DotLocation::get())
                }
                fn worst_case_holding() -> MultiAssets {
                    // A mix of fungible, non-fungible, and concrete assets.
                    const HOLDING_FUNGIBLES: u32 = 100;
                    const HOLDING_NON_FUNGIBLES: u32 = 100;
                    let fungibles_amount: u128 = 100;
                    let mut assets = (0..HOLDING_FUNGIBLES)
                        .map(|i| {
                            MultiAsset {
                                id: Concrete(GeneralIndex(i as u128).into()),
                                fun: Fungible(fungibles_amount * i as u128),
                            }
                            .into()
                        })
                        .chain(core::iter::once(MultiAsset { id: Concrete(Here.into()), fun: Fungible(u128::MAX) }))
                        .chain((0..HOLDING_NON_FUNGIBLES).map(|i| MultiAsset {
                            id: Concrete(GeneralIndex(i as u128).into()),
                            fun: NonFungible(asset_instance_from(i)),
                        }))
                        .collect::<Vec<_>>();

                    assets.push(MultiAsset{
                        id: Concrete(DotLocation::get()),
                        fun: Fungible(1_000_000 * UNITS),
                    });

                    assets.into()
                }
            }

            parameter_types! {
                pub const TrustedTeleporter: Option<(MultiLocation, MultiAsset)> = Some((
                    DotLocation::get(),
                    MultiAsset { fun: Fungible(1 * UNITS), id: Concrete(DotLocation::get()) },
                ));
                pub const TrustedReserve: Option<(MultiLocation, MultiAsset)> = None;
                pub const CheckedAccount: Option<AccountId> = None;
            }

            impl pallet_xcm_benchmarks::fungible::Config for Runtime {
                type TransactAsset = Balances;
                type CheckedAccount = CheckedAccount;
                type TrustedTeleporter = TrustedTeleporter;
                type TrustedReserve = TrustedReserve;

                fn get_multi_asset() -> MultiAsset {
                    MultiAsset {
                        id: Concrete(DotLocation::get()),
                        fun: Fungible(1 * UNITS),
                    }
                }
            }

            impl pallet_xcm_benchmarks::generic::Config for Runtime {
                type RuntimeCall = RuntimeCall;

                fn worst_case_response() -> (u64, Response) {
                    (0u64, Response::Version(Default::default()))
                }

                fn transact_origin() -> Result<MultiLocation, BenchmarkError> {
                    Ok(DotLocation::get())
                }

                fn subscribe_origin() -> Result<MultiLocation, BenchmarkError> {
                    Ok(DotLocation::get())
                }

                fn claimable_asset() -> Result<(MultiLocation, MultiLocation, MultiAssets), BenchmarkError> {
                    let origin = DotLocation::get();
                    let assets: MultiAssets = (Concrete(DotLocation::get()), 1_000 * UNITS).into();
                    let ticket = MultiLocation { parents: 0, interior: Here };
                    Ok((origin, ticket, assets))
               }
            }

            type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet::<Runtime>;
            type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet::<Runtime>;

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
                //TODO: use from relay_well_known_keys::ACTIVE_CONFIG
                hex_literal::hex!("06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            Ok(batches)
        }
    }
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
    fn check_inherents(
        block: &Block,
        relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
    ) -> sp_inherents::CheckInherentsResult {
        let relay_chain_slot = relay_state_proof
            .read_slot()
            .expect("Could not read the relay chain slot from the proof");

        let inherent_data =
            cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
                relay_chain_slot,
                sp_std::time::Duration::from_secs(6),
            )
            .create_inherent_data()
            .expect("Could not create the timestamp inherent data");

        inherent_data.check_extrinsics(block)
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
       Runtime = Runtime,
       BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
       CheckInherents = CheckInherents,
}
