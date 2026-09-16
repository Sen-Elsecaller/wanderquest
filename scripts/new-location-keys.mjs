// Generates the ed25519 key pair of a quest location.
//
// The secret half is what proves a visit: the signing backend holds it and signs
// `quest_id || nonce || user_address` for someone who is physically there. It
// never goes in the QR, in a passive tag, or in this repo - whoever holds it can
// mint WQ without leaving the house.
//
//   node scripts/new-location-keys.mjs 3
//
// Prints one JSON object per key. Public halves go on-chain via `register_quest`;
// secret halves belong in .env, which is gitignored.

import { generateKeyPairSync } from "node:crypto";

const count = Number.parseInt(process.argv[2] ?? "1", 10);
if (!Number.isInteger(count) || count < 1) {
    console.error("usage: node scripts/new-location-keys.mjs [count]");
    process.exit(1);
}

const keys = [];
for (let questId = 1; questId <= count; questId++) {
    const { publicKey, privateKey } = generateKeyPairSync("ed25519");
    const raw = (jwk, field) => Buffer.from(jwk[field], "base64url").toString("hex");
    keys.push({
        quest_id: questId,
        public_hex: raw(publicKey.export({ format: "jwk" }), "x"),
        secret_hex: raw(privateKey.export({ format: "jwk" }), "d"),
    });
}

console.log(JSON.stringify(keys, null, 2));
