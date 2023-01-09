# Polkadot Smart Chain (PSC)
  PSC is an acronym for polkadot smart contract platform. It is an EVM smart contract parachain with DOT as the native Gas fee.

## Polkadot's Parachain
PSC is a Polkadot parachain which uses directly `DOT` as transaction fees and jointly developed by the ChainX team and OmniBTC team.
The ChainX team will build PSC into a cross-chain interoperability hub among the Bitcoin network, Polkadot ecology, EVM ecology, MoveVM ecology, and Cosmos ecology based on technologies such as Zkrollup, XCMP, ibc, and Lightning Network.

## Part of OmniBTC liquidity aggregation
[DolaProtocol](https://github.com/OmniBTC/OmniProtocol)  is a chain-wide liquidity aggregation and settlement system with the single coin pool of each public chain as the core, Wormhole, Layerzero and other cross-chain messaging protocols as the bridge, and Sui public chain as the settlement center.

There are `DOT` and assets of each parachain on the Polkadot platform. We will deploy a single currency pool on the Polkadot Smart Chain to become OmniBTC’s liquidity pool site on the Polkadot platform, which is connected to other Polkadot parachains through the XCMP protocol. 
***Committed to allowing the original assets on Polkadot to circulate with mainstream native assets such as BTC/ETH.***

## Transfer DOT from Polkadot to PSC by DMP

![dmp](./docs/dmp.png)

## Transfer DOT from PSC to Polkadot by UMP

![ump](./docs/ump.png)

## **metamask config**
```txt
Network name: Polkadot Smart Chain
RPC URL: https://psc-parachain.coming.chat/rpc
Chain ID: 1508
Currency symbol: DOT
```

for [local zombienet](./zombienet/psc-small-network.toml), use `RPC URL: http://127.0.0.1:8546`

## Substrate Account & EVM address
see [assets-bridge](./pallets/assets-bridge/README.md)

[convert evm address to dot account](./scripts/js/src/evm_to_dot.js)

```txt
EXISTENTIAL_DEPOSIT = 0.01 DOT
wasm transfer:  0.0047732 DOT
evm transfer:  0.0046515 DOT
evm-address mapping reseve: 0.1 DOT
```
