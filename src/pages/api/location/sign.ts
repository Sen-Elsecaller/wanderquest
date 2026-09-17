import type { APIRoute } from "astro";
import { Address } from "@stellar/stellar-sdk";
import { signVisit } from "../../../lib/location-signer";

export const prerender = false;

/**
 * POST /api/location/sign  { quest_id, nonce, address }
 *   -> { quest_id, nonce, address, signature }
 *
 * Second half of the handshake. The address goes into the signed message, which
 * is what makes the proof useless to anyone else: the server encodes it here
 * rather than trusting the client to send the right bytes.
 */
export const POST: APIRoute = async ({ request }) => {
    let body: { quest_id?: unknown; nonce?: unknown; address?: unknown };
    try {
        body = await request.json();
    } catch {
        return json({ error: "se esperaba un cuerpo JSON" }, 400);
    }

    const { quest_id: questId, nonce, address } = body;
    if (!Number.isInteger(questId)) return json({ error: "quest_id tiene que ser un entero" }, 400);
    if (typeof nonce !== "string" || !/^[0-9a-f]{64}$/.test(nonce)) {
        return json({ error: "nonce tiene que ser 64 caracteres hex" }, 400);
    }
    if (typeof address !== "string") return json({ error: "falta address" }, 400);

    let addressXdr: Buffer;
    try {
        addressXdr = Address.fromString(address).toScVal().toXDR();
    } catch {
        return json({ error: "address no es una direccion Stellar valida" }, 400);
    }

    try {
        const signature = signVisit(questId as number, nonce, addressXdr);
        return json({ quest_id: questId, nonce, address, signature });
    } catch (error) {
        // A spent, expired or unknown nonce is a rejected claim, not a crash.
        return json({ error: error instanceof Error ? error.message : String(error) }, 409);
    }
};

const json = (body: unknown, status = 200) =>
    new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
