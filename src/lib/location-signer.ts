/**
 * The signing backend of a quest location. Server only - importing this from
 * anything that reaches the browser would ship the secrets with it.
 *
 * A location's ed25519 secret key is what turns "I was there" into WQ: whoever
 * holds it can mint that quest's reward from anywhere. So it lives here, in
 * process memory, read from `.env`, and never travels - not into the QR, not
 * into a passive tag, not into the repo.
 *
 * The flow this serves is challenge-response:
 *
 *   1. The QR at the location carries only `quest_id`.
 *   2. The app asks for a challenge and gets a single-use nonce that expires.
 *   3. The app sends back the nonce plus the address claiming the reward, and
 *      gets a signature over `quest_id || nonce || address`.
 *
 * What this demo does NOT prove is presence: it signs for whoever asks with a
 * live nonce. In a real deployment this code runs on a device at the location
 * and that device decides who gets a signature. The contract's guarantee is
 * narrower and honest - possession of a signature from the location's signer.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createPrivateKey, randomBytes, sign as edSign, type KeyObject } from "node:crypto";

/** How long a challenge stays claimable. */
const NONCE_TTL_MS = 5 * 60 * 1000;

/** The fixed PKCS8 envelope Node needs around a raw ed25519 seed. */
const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");

type Challenge = { questId: number; expiresAt: number };

/** Issued and not yet spent. Process memory is enough for a demo signer. */
const openChallenges = new Map<string, Challenge>();

let secretsByQuest: Map<number, string> | null = null;

/**
 * Location secrets, from the environment first and `.env` second. The file
 * fallback is what makes `astro dev` work without exporting anything by hand.
 */
function loadSecrets(): Map<number, string> {
    if (secretsByQuest) return secretsByQuest;

    const secrets = new Map<number, string>();
    const remember = (name: string, value: string) => {
        const match = /^WQ_LOCATION_SECRET_(\d+)$/.exec(name);
        if (match && /^[0-9a-fA-F]{64}$/.test(value)) {
            secrets.set(Number.parseInt(match[1], 10), value.toLowerCase());
        }
    };

    for (const [name, value] of Object.entries(process.env)) {
        if (value) remember(name, value);
    }

    // Relative to the working directory, not to this module: the build bundles
    // this file into dist/server and any path relative to it stops resolving.
    try {
        for (const line of readFileSync(resolve(process.cwd(), ".env"), "utf8").split("\n")) {
            const [name, ...rest] = line.trim().split("=");
            if (name && rest.length) remember(name, rest.join("=").trim());
        }
    } catch {
        // No .env file: whatever the environment carried is all there is.
    }

    secretsByQuest = secrets;
    return secrets;
}

function signingKey(questId: number): KeyObject {
    const seed = loadSecrets().get(questId);
    if (!seed) throw new Error(`la quest ${questId} no tiene llave de ubicacion configurada`);
    return createPrivateKey({
        key: Buffer.concat([PKCS8_ED25519_PREFIX, Buffer.from(seed, "hex")]),
        format: "der",
        type: "pkcs8",
    });
}

/** Quest ids this signer can actually sign for. */
export function configuredQuests(): number[] {
    return [...loadSecrets().keys()].sort((a, b) => a - b);
}

/** Issues a single-use nonce for `questId`. */
export function issueChallenge(questId: number): { nonce: string; expiresAt: number } {
    if (!loadSecrets().has(questId)) {
        throw new Error(`la quest ${questId} no tiene llave de ubicacion configurada`);
    }

    dropExpired();
    const nonce = randomBytes(32).toString("hex");
    const expiresAt = Date.now() + NONCE_TTL_MS;
    openChallenges.set(nonce, { questId, expiresAt });
    return { nonce, expiresAt };
}

/**
 * Signs `quest_id || nonce || address` and spends the nonce. These are the exact
 * bytes `QuestManager::visit_message` rebuilds on-chain, so the address has to be
 * the one that will submit the transaction.
 */
export function signVisit(questId: number, nonce: string, addressXdr: Buffer): string {
    const challenge = openChallenges.get(nonce);
    if (!challenge) throw new Error("nonce desconocido o ya usado");
    if (challenge.questId !== questId) throw new Error("el nonce es de otra quest");
    if (challenge.expiresAt < Date.now()) {
        openChallenges.delete(nonce);
        throw new Error("el nonce vencio");
    }

    // Spend it before signing: a proof is issued once, whatever happens next.
    openChallenges.delete(nonce);

    const questIdBytes = Buffer.alloc(4);
    questIdBytes.writeUInt32BE(questId);
    const message = Buffer.concat([questIdBytes, Buffer.from(nonce, "hex"), addressXdr]);

    return edSign(null, message, signingKey(questId)).toString("hex");
}

function dropExpired() {
    const now = Date.now();
    for (const [nonce, challenge] of openChallenges) {
        if (challenge.expiresAt < now) openChallenges.delete(nonce);
    }
}
