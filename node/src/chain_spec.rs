// Copyright (C) 2022 Polkadot Smart Chain (PSC).
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

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::{config::TelemetryEndpoints, ChainType};
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use psc_runtime::{
    common::{AccountId, AuraId, Balance, Signature},
    constants::currency::EXISTENTIAL_DEPOSIT,
};

const POLKADOT_PARA_ID: u32 = 2053;
const DEFAULT_PROTOCOL_ID: &str = "psc_polkadot";
const CHAINX_TELEMETRY_URL: &str = "wss://telemetry.chainx.org/submit/";

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<psc_runtime::GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn psc_session_keys(keys: AuraId) -> psc_runtime::SessionKeys {
    psc_runtime::SessionKeys { aura: keys }
}

type AssetId = u32;
type AssetName = Vec<u8>;
type AssetSymbol = Vec<u8>;
type AssetDecimals = u8;
type AssetSufficient = bool;
type AssetMinBalance = Balance;

#[allow(clippy::type_complexity)]
fn reserved_assets(
    root_key: &AccountId,
) -> (
    Vec<(AssetId, AccountId, AssetSufficient, AssetMinBalance)>,
    Vec<(AssetId, AssetName, AssetSymbol, AssetDecimals)>,
) {
    (
        vec![
            (0, root_key.clone(), true, 10_000_000_000u128),
            (1, root_key.clone(), true, 1u128),
            (2, root_key.clone(), true, 10_000_000_000u128),
            (3, root_key.clone(), true, 10_000_000_000u128),
            (4, root_key.clone(), true, 10_000_000_000u128),
            (5, root_key.clone(), true, 10_000_000_000u128),
            (6, root_key.clone(), true, 10_000_000_000u128),
            (7, root_key.clone(), true, 10_000_000_000u128),
            (8, root_key.clone(), true, 10_000_000_000u128),
            (9, root_key.clone(), true, 1u128),
        ],
        vec![
            (0, "Reserved0".to_string().into_bytes(), "RSV0".to_string().into_bytes(), 18),
            (1, "Reserved1".to_string().into_bytes(), "RSV1".to_string().into_bytes(), 8),
            (2, "Reserved2".to_string().into_bytes(), "RSV2".to_string().into_bytes(), 18),
            (3, "Reserved3".to_string().into_bytes(), "RSV3".to_string().into_bytes(), 18),
            (4, "Reserved4".to_string().into_bytes(), "RSV4".to_string().into_bytes(), 18),
            (5, "Reserved5".to_string().into_bytes(), "RSV5".to_string().into_bytes(), 18),
            (6, "Reserved6".to_string().into_bytes(), "RSV6".to_string().into_bytes(), 18),
            (7, "Reserved7".to_string().into_bytes(), "RSV7".to_string().into_bytes(), 18),
            (8, "Reserved8".to_string().into_bytes(), "RSV8".to_string().into_bytes(), 18),
            (9, "Reserved9".to_string().into_bytes(), "RSV9".to_string().into_bytes(), 8),
        ],
    )
}

pub fn development_config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("ss58Format".into(), 0.into());
    properties.insert("tokenSymbol".into(), "DOT".into());
    properties.insert("tokenDecimals".into(), 10.into());

    ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            psc_genesis(
                // initial collators.
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                POLKADOT_PARA_ID.into(),
                get_account_id_from_seed::<sr25519::Public>("Alice"),
            )
        },
        Vec::new(),
        Some(
            TelemetryEndpoints::new(vec![(CHAINX_TELEMETRY_URL.to_string(), 0)])
                .expect("PSC telemetry url is valid; qed"),
        ),
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Some(properties),
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: POLKADOT_PARA_ID,
        },
    )
}

pub fn psc_config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("ss58Format".into(), 0.into());
    properties.insert("tokenSymbol".into(), "DOT".into());
    properties.insert("tokenDecimals".into(), 10.into());

    ChainSpec::from_genesis(
        // Name
        "Polkadot Smart Chain",
        // ID
        "psc",
        ChainType::Live,
        move || {
            psc_genesis(
                // initial collators.
                vec![(
                    // 135jgXDi2HPt8KDmHrxLz44sNzNg1Dp3dcWmkmeQMaY9M8he
                    hex!("5c15207d5d764cc633fc7c29da559a1efc8a4369ce7868daeaf8844c6fc68739").into(),
                    hex!("ac0d16845456daf555c0102297f6337e52e600d879497610eb0ccf5c7e09485b")
                        .unchecked_into(),
                )],
                vec![],
                POLKADOT_PARA_ID.into(),
                hex!("5c15207d5d764cc633fc7c29da559a1efc8a4369ce7868daeaf8844c6fc68739").into(),
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        Some(
            TelemetryEndpoints::new(vec![(CHAINX_TELEMETRY_URL.to_string(), 0)])
                .expect("PSC telemetry url is valid; qed"),
        ),
        // Protocol ID
        Some(DEFAULT_PROTOCOL_ID),
        // Fork ID
        None,
        // Properties
        Some(properties),
        // Extensions
        Extensions {
            relay_chain: "polkadot".into(), // You MUST set this to the correct network!
            para_id: POLKADOT_PARA_ID,
        },
    )
}

fn psc_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    root_key: AccountId,
) -> psc_runtime::GenesisConfig {
    let assets_info = reserved_assets(&root_key);

    psc_runtime::GenesisConfig {
        system: psc_runtime::SystemConfig {
            code: psc_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        sudo: psc_runtime::SudoConfig { key: Some(root_key) },
        balances: psc_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, EXISTENTIAL_DEPOSIT * 4096))
                .collect(),
        },
        parachain_info: psc_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: psc_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: psc_runtime::SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),            // account id
                        acc,                    // validator id
                        psc_session_keys(aura), // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        polkadot_xcm: psc_runtime::PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
        ethereum_chain_id: psc_runtime::EthereumChainIdConfig { chain_id: 1508u64 },
        evm: Default::default(),
        ethereum: Default::default(),
        base_fee: psc_runtime::BaseFeeConfig::new(
            psc_runtime::DefaultBaseFeePerGas::get(),
            sp_runtime::Permill::zero(),
        ),
        assets: psc_runtime::AssetsConfig {
            assets: assets_info.0,
            metadata: assets_info.1,
            accounts: vec![],
        },
        assets_bridge: psc_runtime::AssetsBridgeConfig { admin_key: None },
    }
}
