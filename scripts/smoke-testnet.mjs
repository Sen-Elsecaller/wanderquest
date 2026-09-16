// End-to-end proof against the deployed testnet contracts.
//
//   node scripts/smoke-testnet.mjs
//
// Walks the whole WanderQuest loop with a throwaway account, and then tries the
// three attacks the demo shows failing:
//
//   1. verified visit -> WQ minted
//   2. redeem at a merchant -> 95% paid, 5% burned
//   3. the same proof replayed -> rejected
//   4. a proof issued to someone else -> rejected
//   5. a proof signed by the wrong key -> rejected
//
// This is also the reference for what the signing backend and the app will do in
// F3: the message built here is exactly what `QuestManager::visit_message`
// rebuilds on-chain.

import { readFileSync } from "node:fs";
import { createPrivateKey, createPublicKey, sign as edSign, randomBytes } from "node:crypto";
import {
    Address,
    Contract,
    Keypair,
    Networks,
    TransactionBuilder,
    nativeToScVal,
    rpc,
    scValToNative,
    xdr,
} from "@stellar/stellar-sdk";

const RPC_URL = "https://soroban-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const repoRoot = new URL("..", import.meta.url);

const server = new rpc.Server(RPC_URL);
const deployment = JSON.parse(readFileSync(new URL("deployments/testnet.json", repoRoot), "utf8"));

// --- the signing backend's half ---

const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");

function locationKey(questId) {
    const env = readFileSync(new URL(".env", repoRoot), "utf8");
    const match = env.match(new RegExp(`^WQ_LOCATION_SECRET_${questId}=([0-9a-fA-F]{64})$`, "m"));
    if (!match) throw new Error(`no hay WQ_LOCATION_SECRET_${questId} en .env`);
    const der = Buffer.concat([PKCS8_ED25519_PREFIX, Buffer.from(match[1], "hex")]);
    return createPrivateKey({ key: der, format: "der", type: "pkcs8" });
}

/** The exact bytes the contract rebuilds: quest_id || nonce || XDR of the address. */
function visitMessage(questId, nonce, address) {
    const id = Buffer.alloc(4);
    id.writeUInt32BE(questId);
    return Buffer.concat([id, nonce, Address.fromString(address).toScVal().toXDR()]);
}

/** What `POST /api/location/sign` will return in F3. */
function signVisit(key, questId, nonce, address) {
    return edSign(null, visitMessage(questId, nonce, address), key);
}

// --- chain plumbing ---

async function fundAccount(publicKey) {
    const response = await fetch(`${FRIENDBOT}?addr=${publicKey}`);
    if (!response.ok && response.status !== 400) {
        throw new Error(`friendbot fallo: ${response.status}`);
    }
}

async function invoke(keypair, contractId, method, args) {
    const account = await server.getAccount(keypair.publicKey());
    const transaction = new TransactionBuilder(account, {
        fee: "1000000",
        networkPassphrase: Networks.TESTNET,
    })
        .addOperation(new Contract(contractId).call(method, ...args))
        .setTimeout(60)
        .build();

    const prepared = await server.prepareTransaction(transaction);
    prepared.sign(keypair);

    const sent = await server.sendTransaction(prepared);
    if (sent.status === "ERROR") throw new Error(`envio rechazado: ${JSON.stringify(sent.errorResult)}`);

    let result = await server.getTransaction(sent.hash);
    while (result.status === "NOT_FOUND") {
        await new Promise((resolve) => setTimeout(resolve, 1000));
        result = await server.getTransaction(sent.hash);
    }
    if (result.status !== "SUCCESS") throw new Error(`transaccion fallida: ${result.status}`);
    return { hash: sent.hash, value: result.returnValue ? scValToNative(result.returnValue) : null };
}

/** Runs an invocation that is supposed to be rejected, and says why it was. */
async function expectRejected(label, run) {
    try {
        await run();
        console.log(`  FALLO  ${label}: fue ACEPTADA y no debia serlo`);
        return false;
    } catch (error) {
        const reason = String(error.message).split("\n")[0].slice(0, 110);
        console.log(`  ok     ${label}: rechazada (${reason})`);
        return true;
    }
}

async function balanceOf(address) {
    const result = await server.simulateTransaction(
        new TransactionBuilder(await server.getAccount(address), {
            fee: "100000",
            networkPassphrase: Networks.TESTNET,
        })
            .addOperation(
                new Contract(deployment.wq_token).call("balance", nativeToScVal(address, { type: "address" })),
            )
            .setTimeout(30)
            .build(),
    );
    return BigInt(scValToNative(result.result.retval));
}

const wq = (stroops) => `${Number(stroops) / 1e7} WQ`;

// --- the walk ---

const QUEST_ID = 1;
const questReward = BigInt(deployment.quests.find((q) => q.quest_id === QUEST_ID).reward);

console.log(`red        testnet`);
console.log(`token      ${deployment.wq_token}`);
console.log(`manager    ${deployment.quest_manager}`);

const user = Keypair.random();
const merchant = Keypair.random();
const stranger = Keypair.random();
await Promise.all([fundAccount(user.publicKey()), fundAccount(merchant.publicKey()), fundAccount(stranger.publicKey())]);
console.log(`usuario    ${user.publicKey()}`);
console.log(`comercio   ${merchant.publicKey()}`);

const key = locationKey(QUEST_ID);

console.log("\n1. Visita verificada");
const nonce = randomBytes(32);
const signature = signVisit(key, QUEST_ID, nonce, user.publicKey());
const minted = await invoke(user, deployment.quest_manager, "complete_quest", [
    nativeToScVal(user.publicKey(), { type: "address" }),
    nativeToScVal(QUEST_ID, { type: "u32" }),
    xdr.ScVal.scvBytes(nonce),
    xdr.ScVal.scvBytes(signature),
]);
const afterMint = await balanceOf(user.publicKey());
console.log(`  ok     acunados ${wq(questReward)} -> balance ${wq(afterMint)}`);
console.log(`         https://stellar.expert/explorer/testnet/tx/${minted.hash}`);

console.log("\n2. Canje en el comercio");
const spend = 10_000_000n;
const fee = (spend * 5n) / 100n;
const redeemed = await invoke(user, deployment.quest_manager, "redeem", [
    nativeToScVal(user.publicKey(), { type: "address" }),
    nativeToScVal(merchant.publicKey(), { type: "address" }),
    nativeToScVal(spend, { type: "i128" }),
]);
const merchantBalance = await balanceOf(merchant.publicKey());
const userBalance = await balanceOf(user.publicKey());
console.log(`  ok     comercio ${wq(merchantBalance)}, usuario ${wq(userBalance)}, quemado ${wq(fee)}`);
console.log(`         https://stellar.expert/explorer/testnet/tx/${redeemed.hash}`);

console.log("\n3. Los tres ataques");
const results = [];
results.push(
    await expectRejected("prueba reusada", () =>
        invoke(user, deployment.quest_manager, "complete_quest", [
            nativeToScVal(user.publicKey(), { type: "address" }),
            nativeToScVal(QUEST_ID, { type: "u32" }),
            xdr.ScVal.scvBytes(nonce),
            xdr.ScVal.scvBytes(signature),
        ]),
    ),
);

const stolenNonce = randomBytes(32);
const stolenSignature = signVisit(key, QUEST_ID, stolenNonce, user.publicKey());
results.push(
    await expectRejected("prueba de otro usuario", () =>
        invoke(stranger, deployment.quest_manager, "complete_quest", [
            nativeToScVal(stranger.publicKey(), { type: "address" }),
            nativeToScVal(QUEST_ID, { type: "u32" }),
            xdr.ScVal.scvBytes(stolenNonce),
            xdr.ScVal.scvBytes(stolenSignature),
        ]),
    ),
);

const impostor = createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519_PREFIX, randomBytes(32)]),
    format: "der",
    type: "pkcs8",
});
const forgedNonce = randomBytes(32);
results.push(
    await expectRejected("firma de otra llave", () =>
        invoke(user, deployment.quest_manager, "complete_quest", [
            nativeToScVal(user.publicKey(), { type: "address" }),
            nativeToScVal(QUEST_ID, { type: "u32" }),
            xdr.ScVal.scvBytes(forgedNonce),
            xdr.ScVal.scvBytes(signVisit(impostor, QUEST_ID, forgedNonce, user.publicKey())),
        ]),
    ),
);

const allRejected = results.every(Boolean);
const balanceHolds = afterMint === questReward && merchantBalance === spend - fee;
console.log(`\n${allRejected && balanceHolds ? "TODO OK" : "HAY ALGO MAL"}`);
process.exit(allRejected && balanceHolds ? 0 : 1);
