// USAGE:
//  nodejs src/get_contributors.js
const ENDPOINT= "wss://rpc.polkadot.io";
const PARAID = 2053;
const DUMP="./psc-contributors.json"
// block 13374400
const LAST_HASH="0x0aa896860ba4c3bb4c538967e8cd290d1ad8c7b112404d10efbf4165043f2ef3"

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { u8aConcat, u8aToHex } = require('@polkadot/util');
const { blake2AsU8a, encodeAddress } = require('@polkadot/util-crypto');
const fs = require('fs');

function createChildKey(trieIndex) {
    return u8aToHex(
        u8aConcat(
            ':child_storage:default:',
            blake2AsU8a(
                u8aConcat('crowdloan', trieIndex.toU8a())
            )
        )
    );
}

async function main () {
    const wsProvider = new WsProvider(process.env.ENDPOINT || ENDPOINT);
    const api = await ApiPromise.create({ provider: wsProvider });
    const paraId = parseInt(process.env.PARAID || PARAID);
    const dumpJson = process.env.DUMP || DUMP;
    const blockHash = LAST_HASH;

    const fund = await api.query.crowdloan.funds.at(blockHash, paraId);
    const trieIndex = fund.unwrap().fundIndex;
    const childKey = createChildKey(trieIndex);

    const keys = await api.rpc.childstate.getKeys(childKey, '0x', blockHash);
    const ss58Keys = keys.map(k => encodeAddress(k, 0));
    console.log(ss58Keys);

    const values = await Promise.all(keys.map(k => api.rpc.childstate.getStorage(childKey, k, blockHash)));
    const contributions = values.map((v, idx) => ({
        contributor: ss58Keys[idx],
        balance: api.createType('(Balance, Vec<u8>)', v.unwrap()).toJSON()[0],
    }));

    console.log(contributions);

    if (dumpJson) {
        const jsonStr = JSON.stringify(contributions, undefined, 2);
        fs.writeFileSync(dumpJson, jsonStr, {encoding: 'utf-8'});
    }
}

main().catch(console.error).finally(() => process.exit());
