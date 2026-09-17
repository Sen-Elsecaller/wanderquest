// The same visit as smoke-testnet.mjs, but going through the HTTP signing
// backend instead of reading the location secret directly.
//
//   npm run build && node scripts/smoke-signer.mjs
//
// This is the path the app will walk in F3: challenge -> sign -> transaction.
// It starts the built server itself, so it also proves the routes survive the
// build, and checks that the backend refuses to reuse a nonce or to sign for a
// quest it has no key for.

import { spawn } from "node:child_process";
import { Keypair, nativeToScVal, xdr } from "@stellar/stellar-sdk";
import { balanceOf, fundAccount, invoke, loadDeployment, wq } from "./lib/chain.mjs";

const PORT = 4321;
const BASE = `http://127.0.0.1:${PORT}`;
const QUEST_ID = 1;

const deployment = loadDeployment();
const entry = new URL("../dist/server/entry.mjs", import.meta.url);

async function post(path, body) {
    const response = await fetch(`${BASE}${path}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
    });
    return { status: response.status, body: await response.json() };
}

async function waitForServer(attempts = 40) {
    for (let i = 0; i < attempts; i++) {
        try {
            await fetch(BASE, { method: "HEAD" });
            return;
        } catch {
            await new Promise((resolve) => setTimeout(resolve, 250));
        }
    }
    throw new Error("el servidor no levanto");
}

const server = spawn(process.execPath, [entry.pathname.slice(1)], {
    env: { ...process.env, HOST: "127.0.0.1", PORT: String(PORT) },
    stdio: ["ignore", "ignore", "inherit"],
});

let failures = 0;
const check = (ok, label, detail = "") => {
    console.log(`  ${ok ? "ok    " : "FALLO "} ${label}${detail ? ` (${detail})` : ""}`);
    if (!ok) failures++;
};

try {
    await waitForServer();
    console.log(`servidor   ${BASE}`);
    console.log(`manager    ${deployment.quest_manager}`);

    const user = Keypair.random();
    await fundAccount(user.publicKey());
    console.log(`usuario    ${user.publicKey()}\n`);

    console.log("1. Handshake con el firmante");
    const challenge = await post("/api/location/challenge", { quest_id: QUEST_ID });
    check(challenge.status === 200 && /^[0-9a-f]{64}$/.test(challenge.body.nonce), "challenge entrega un nonce");

    const signed = await post("/api/location/sign", {
        quest_id: QUEST_ID,
        nonce: challenge.body.nonce,
        address: user.publicKey(),
    });
    check(signed.status === 200 && /^[0-9a-f]{128}$/.test(signed.body.signature), "sign entrega una firma");

    console.log("\n2. La firma sirve on-chain");
    const minted = await invoke(user, deployment.quest_manager, "complete_quest", [
        nativeToScVal(user.publicKey(), { type: "address" }),
        nativeToScVal(QUEST_ID, { type: "u32" }),
        xdr.ScVal.scvBytes(Buffer.from(challenge.body.nonce, "hex")),
        xdr.ScVal.scvBytes(Buffer.from(signed.body.signature, "hex")),
    ]);
    const balance = await balanceOf(deployment.wq_token, user.publicKey());
    const reward = BigInt(deployment.quests.find((q) => q.quest_id === QUEST_ID).reward);
    check(balance === reward, `acunados ${wq(balance)}`, minted.hash);

    console.log("\n3. Lo que el firmante rechaza");
    const reused = await post("/api/location/sign", {
        quest_id: QUEST_ID,
        nonce: challenge.body.nonce,
        address: user.publicKey(),
    });
    check(reused.status === 409, "nonce ya gastado", reused.body.error);

    const unknownQuest = await post("/api/location/challenge", { quest_id: 999 });
    check(unknownQuest.status === 404, "quest sin llave configurada", unknownQuest.body.error);

    const badAddress = await post("/api/location/sign", {
        quest_id: QUEST_ID,
        nonce: (await post("/api/location/challenge", { quest_id: QUEST_ID })).body.nonce,
        address: "no-soy-una-address",
    });
    check(badAddress.status === 400, "address invalida", badAddress.body.error);
} finally {
    server.kill();
}

console.log(`\n${failures === 0 ? "TODO OK" : `${failures} FALLARON`}`);
process.exit(failures === 0 ? 0 : 1);
