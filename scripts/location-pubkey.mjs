// Derives the ed25519 public key of a location from its secret seed.
//
//   node scripts/location-pubkey.mjs <64-hex-char seed>
//
// The deploy script uses this so the secret in .env stays the single source of
// truth: there is no public-key list to drift out of sync with it.

import { createPrivateKey, createPublicKey } from "node:crypto";

const seedHex = (process.argv[2] ?? "").trim();
if (!/^[0-9a-fA-F]{64}$/.test(seedHex)) {
    console.error("usage: node scripts/location-pubkey.mjs <64 hex chars>");
    process.exit(1);
}

// Wrap the raw seed in the fixed PKCS8 envelope Node expects for Ed25519.
const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const der = Buffer.concat([PKCS8_ED25519_PREFIX, Buffer.from(seedHex, "hex")]);
const privateKey = createPrivateKey({ key: der, format: "der", type: "pkcs8" });
const jwk = createPublicKey(privateKey).export({ format: "jwk" });

process.stdout.write(Buffer.from(jwk.x, "base64url").toString("hex"));
