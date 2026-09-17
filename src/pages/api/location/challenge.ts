import type { APIRoute } from "astro";
import { configuredQuests, issueChallenge } from "../../../lib/location-signer";

export const prerender = false;

/**
 * POST /api/location/challenge  { quest_id }
 *   -> { quest_id, nonce, expires_at }
 *
 * First half of the visit handshake. The nonce is single-use and short-lived, so
 * a QR photographed today is worth nothing tomorrow.
 */
export const POST: APIRoute = async ({ request }) => {
    let questId: unknown;
    try {
        ({ quest_id: questId } = await request.json());
    } catch {
        return json({ error: "se esperaba un cuerpo JSON" }, 400);
    }

    if (!Number.isInteger(questId)) {
        return json({ error: "quest_id tiene que ser un entero" }, 400);
    }

    try {
        const { nonce, expiresAt } = issueChallenge(questId as number);
        return json({ quest_id: questId, nonce, expires_at: new Date(expiresAt).toISOString() });
    } catch (error) {
        return json({ error: message(error), quests: configuredQuests() }, 404);
    }
};

const json = (body: unknown, status = 200) =>
    new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
