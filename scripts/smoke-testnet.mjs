// End-to-end proof against the deployed testnet contracts, signing locally.
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
// This one reads the location secret straight from .env. For the same walk going
// through the HTTP signing backend instead, see smoke-signer.mjs.

import { readFileSync } from "node:fs";
import { createPrivateKey, randomBytes, sign as edSign } from "node:crypto";
import { Address, Keypair, nativeToScVal, xdr } from "@stellar/stellar-sdk";
import { balanceOf, fundAccount, invoke, loadDeployment, wq } from "./lib/chain.mjs";

const repoRoot = new URL("..", import.meta.url);
const deployment = loadDeployment();

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

function signVisit(key, questId, nonce, address) {
    return edSign(null, visitMessage(questId, nonce, address), key);
}

const completeQuestArgs = (address, questId, nonce, signature) => [
    nativeToScVal(address, { type: "address" }),
    nativeToScVal(questId, { type: "u32" }),
    xdr.ScVal.scvBytes(nonce),
    xdr.ScVal.scvBytes(signature),
];

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

// --- the walk ---

const QUEST_ID = 1;
const questReward = BigInt(deployment.quests.find((q) => q.quest_id === QUEST_ID).reward);

console.log("red        testnet");
console.log(`token      ${deployment.wq_token}`);
console.log(`manager    ${deployment.quest_manager}`);

const user = Keypair.random();
const merchant = Keypair.random();
const stranger = Keypair.random();
await Promise.all([
    fundAccount(user.publicKey()),
    fundAccount(merchant.publicKey()),
    fundAccount(stranger.publicKey()),
]);
console.log(`usuario    ${user.publicKey()}`);
console.log(`comercio   ${merchant.publicKey()}`);

const key = locationKey(QUEST_ID);

console.log("\n1. Visita verificada");
const nonce = randomBytes(32);
const signature = signVisit(key, QUEST_ID, nonce, user.publicKey());
const minted = await invoke(
    user,
    deployment.quest_manager,
    "complete_quest",
    completeQuestArgs(user.publicKey(), QUEST_ID, nonce, signature),
);
const afterMint = await balanceOf(deployment.wq_token, user.publicKey());
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
const merchantBalance = await balanceOf(deployment.wq_token, merchant.publicKey());
const userBalance = await balanceOf(deployment.wq_token, user.publicKey());
console.log(`  ok     comercio ${wq(merchantBalance)}, usuario ${wq(userBalance)}, quemado ${wq(fee)}`);
console.log(`         https://stellar.expert/explorer/testnet/tx/${redeemed.hash}`);

console.log("\n3. Los tres ataques");
const results = [];
results.push(
    await expectRejected("prueba reusada", () =>
        invoke(
            user,
            deployment.quest_manager,
            "complete_quest",
            completeQuestArgs(user.publicKey(), QUEST_ID, nonce, signature),
        ),
    ),
);

const stolenNonce = randomBytes(32);
const stolenSignature = signVisit(key, QUEST_ID, stolenNonce, user.publicKey());
results.push(
    await expectRejected("prueba de otro usuario", () =>
        invoke(
            stranger,
            deployment.quest_manager,
            "complete_quest",
            completeQuestArgs(stranger.publicKey(), QUEST_ID, stolenNonce, stolenSignature),
        ),
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
        invoke(
            user,
            deployment.quest_manager,
            "complete_quest",
            completeQuestArgs(
                user.publicKey(),
                QUEST_ID,
                forgedNonce,
                signVisit(impostor, QUEST_ID, forgedNonce, user.publicKey()),
            ),
        ),
    ),
);

const allRejected = results.every(Boolean);
const balanceHolds = afterMint === questReward && merchantBalance === spend - fee;
console.log(`\n${allRejected && balanceHolds ? "TODO OK" : "HAY ALGO MAL"}`);
process.exit(allRejected && balanceHolds ? 0 : 1);
