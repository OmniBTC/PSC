pub use crate as assets_bridge;
pub use assets_bridge::{Config, Error, Event as AssetsBridgeEvent};

use frame_support::{
    pallet_prelude::Weight,
    parameter_types,
    traits::{ConstU32, GenesisBuild},
};
use frame_system as system;
use sp_core::{H160, H256};
pub use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Evm: pallet_evm::{Pallet, Call, Storage, Config, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        AssetsBridge: assets_bridge::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 44;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<5>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type RuntimeOrigin = RuntimeOrigin;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MinimumPeriod: u64 = 1000;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const AssetDeposit: u64 = 1;
    pub const ApprovalDeposit: u64 = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: u64 = 1;
    pub const MetadataDepositPerByte: u64 = 1;

    // 0x1111111111111111111111111111111111111111
    pub EvmCaller: H160 = H160::from_slice(&[17u8;20][..]);
    pub ClaimBond: u128 = 2;

    pub const WeightPerGas: Weight = Weight::from_ref_time(20_000);
}

impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId32>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type WeightInfo = ();
    type Extra = ();
    type AssetAccountDeposit = ();
}

impl pallet_evm::Config for Test {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = pallet_evm::EnsureAddressRoot<Self::AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressNever<Self::AccountId>;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = ();
    type PrecompilesValue = ();
    type ChainId = ();
    type BlockGasLimit = ();
    type OnChargeTransaction = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
}

impl assets_bridge::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type EvmCaller = EvmCaller;
    type ClaimBond = ClaimBond;
}

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE.into(), 1000), (BOB.into(), 1000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    assets_bridge::GenesisConfig::<Test> { admin_key: Some(ALICE.into()) }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));

    ext
}

pub(crate) fn last_event() -> RuntimeEvent {
    system::Pallet::<Test>::events().pop().expect("Event expected").event
}

pub(crate) fn expect_event<E: Into<RuntimeEvent>>(e: E) {
    assert_eq!(last_event(), e.into());
}