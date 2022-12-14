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

//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use sc_client_api::AuxStore;
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
// evm
use fc_rpc::{
    EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
    SchemaV2Override, SchemaV3Override, StorageOverride,
};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_storage::EthereumStorageSchema;
use psc_runtime::common::{opaque::Block, AccountId, Balance, Hash, Index};
use sc_client_api::{
    backend::{Backend, StateBackend, StorageProvider},
    client::BlockchainEvents,
};
use sc_network::NetworkService;
use sc_transaction_pool::{ChainApi, Pool};
use sp_runtime::traits::BlakeTwo256;
use std::collections::BTreeMap;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P, A: ChainApi> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,

    /// EVM
    /// Graph pool instance.
    pub graph: Arc<Pool<A>>,
    /// The Node authority flag
    pub is_authority: bool,
    /// Network service
    pub network: Arc<NetworkService<Block, Hash>>,
    /// EthFilterApi pool.
    pub filter_pool: FilterPool,
    /// Backend.
    pub backend: Arc<fc_db::Backend<Block>>,
    /// Maximum number of logs in a query.
    pub max_past_logs: u32,
    /// Fee history cache.
    pub fee_history_cache: FeeHistoryCache,
    /// Maximum fee history cache size.
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
    /// Ethereum data access overrides.
    pub overrides: Arc<OverrideHandle<Block>>,
    /// Cache for Ethereum block data.
    pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
    C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    C: Send + Sync + 'static,
    C::Api: sp_api::ApiExt<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>,
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
{
    let mut overrides_map = BTreeMap::new();
    overrides_map.insert(
        EthereumStorageSchema::V1,
        Box::new(SchemaV1Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );
    overrides_map.insert(
        EthereumStorageSchema::V2,
        Box::new(SchemaV2Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );
    overrides_map.insert(
        EthereumStorageSchema::V3,
        Box::new(SchemaV3Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );

    Arc::new(OverrideHandle {
        schemas: overrides_map,
        fallback: Box::new(RuntimeApiStorageOverride::new(client)),
    })
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, BE, A>(
    deps: FullDeps<C, P, A>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
    C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
    C: BlockchainEvents<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: BlockBuilder<Block>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    P: TransactionPool<Block = Block> + 'static,
    A: ChainApi<Block = Block> + 'static,
{
    use fc_rpc::{
        Eth, EthApiServer, EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net,
        NetApiServer, Web3, Web3ApiServer,
    };
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        graph,
        is_authority,
        network,
        filter_pool,
        backend,
        max_past_logs,
        fee_history_cache,
        fee_history_cache_limit,
        overrides,
        block_data_cache,
    } = deps;

    io.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    io.merge(
        Eth::new(
            client.clone(),
            pool.clone(),
            graph,
            Some(psc_runtime::TransactionConverter),
            network.clone(),
            Vec::new(),
            overrides.clone(),
            backend.clone(),
            // Is authority.
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_cache_limit,
            10,
        )
        .into_rpc(),
    )?;

    io.merge(
        EthFilter::new(
            client.clone(),
            backend,
            filter_pool,
            500_usize, // max stored filters
            max_past_logs,
            block_data_cache,
        )
        .into_rpc(),
    )?;

    io.merge(
        EthPubSub::new(
            pool,
            client.clone(),
            network.clone(),
            subscription_task_executor,
            overrides,
        )
        .into_rpc(),
    )?;

    io.merge(
        Net::new(
            client.clone(),
            network,
            // Whether to format the `peer_count` response as Hex (default) or not.
            true,
        )
        .into_rpc(),
    )?;

    io.merge(Web3::new(client).into_rpc())?;

    Ok(io)
}
