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

use super::{
    AccountId, AssetId, Assets, Authorship, Balance, Balances, ParachainInfo, ParachainSystem,
    PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee, XcmpQueue,
};
use frame_support::{
    match_types, parameter_types,
    traits::{Everything, Nothing, PalletInfoAccess},
};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use psc_common::{
    impls::ToStakingPot,
    xcm_config::{DenyTeleportToRelayChain, DenyThenTry},
};
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, AsPrefixedGeneralIndex,
    ConvertedConcreteAssetId, CurrencyAdapter, EnsureXcmOrigin, FungiblesAdapter, IsConcrete,
    LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
    WeightInfoBounds,
};
use xcm_executor::{traits::JustTry, XcmExecutor};

parameter_types! {
       pub const DotLocation: MultiLocation = MultiLocation::parent();
       pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
       pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
       pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
       pub const Local: MultiLocation = Here.into();
       pub AssetsPalletLocation: MultiLocation =
               PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
       pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the parent `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<DotLocation>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteAssetId<
        AssetId,
        Balance,
        AsPrefixedGeneralIndex<AssetsPalletLocation, AssetId, JustTry>,
        JustTry,
    >,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We only want to allow teleports of known assets. We use non-zero issuance as an indication
    // that this asset is known.
    psc_common::impls::NonZeroIssuance<AccountId, Assets>,
    // The account to use for tracking teleports.
    CheckingAccount,
>;
/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `RuntimeOrigin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
       pub const MaxInstructions: u32 = 100;
       pub XcmAssetFeesReceiver: Option<AccountId> = Authorship::author();
}

match_types! {
       pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
               MultiLocation { parents: 1, interior: Here } |
               MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
       };
       pub type ParentOrSiblings: impl Contains<MultiLocation> = {
               MultiLocation { parents: 1, interior: Here } |
               MultiLocation { parents: 1, interior: X1(_) }
       };
}

pub type Barrier = DenyThenTry<
    DenyTeleportToRelayChain,
    (
        TakeWeightCredit,
        AllowTopLevelPaidExecutionFrom<Everything>,
        // Parent and its exec plurality get free execution
        AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
        // Expected responses are OK.
        AllowKnownQueryResponses<PolkadotXcm>,
        // Subscriptions for version tracking are OK.
        AllowSubscriptionsFrom<ParentOrSiblings>,
    ),
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::PscXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
    type Trader =
        UsingComponents<WeightToFee, DotLocation, AccountId, Balances, ToStakingPot<Runtime>>;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
}

/// Converts a local signed origin into an XCM multilocation.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    // We want to disallow users sending (arbitrary) XCMs from this chain.
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
    type XcmRouter = XcmRouter;
    // We support local origins dispatching XCM executions in principle...
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    // Disallow teleport transfer
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::PscXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
    type LocationInverter = LocationInverter<Ancestry>;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}
