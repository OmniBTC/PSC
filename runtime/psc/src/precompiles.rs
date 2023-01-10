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

use pallet_evm::{Precompile, PrecompileHandle, PrecompileResult, PrecompileSet};
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use sp_core::H160;
use sp_std::marker::PhantomData;

/// We include the nine Istanbul precompiles
/// (https://github.com/ethereum/go-ethereum/blob/3c46f557/core/vm/contracts.go#L69)
/// as well as a special precompile for dispatching Substrate extrinsics
pub struct PscPrecompiles<R>(PhantomData<R>);

impl<R> PscPrecompiles<R>
where
    R: pallet_evm::Config,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(PhantomData::<R>)
    }
    /// Return all addresses that contain precompiles. This can be used to populate dummy code
    /// under the precompile.
    pub fn used_addresses() -> sp_std::vec::Vec<H160> {
        sp_std::vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 1024, 1025, 1026]
            .into_iter()
            .map(hash)
            .collect()
    }
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither PSC specific
/// 2048-4095 PSC specific precompiles
impl<R> PrecompileSet for PscPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            a if a == hash(6) => Some(Bn128Add::execute(handle)),
            a if a == hash(7) => Some(Bn128Mul::execute(handle)),
            a if a == hash(8) => Some(Bn128Pairing::execute(handle)),
            a if a == hash(9) => Some(Blake2F::execute(handle)),
            // Non-PSC specific nor Ethereum precompiles :
            a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
            // a if a == hash(1025) => Some(Dispatch::<R>::execute(handle)),
            a if a == hash(1026) => Some(ECRecoverPublicKey::execute(handle)),
            _ => None,
        }
    }
    fn is_precompile(&self, address: H160) -> bool {
        Self::used_addresses().contains(&address)
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
