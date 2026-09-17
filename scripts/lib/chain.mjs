// Plumbing shared by the scripts that talk to testnet: fund an account, invoke a
// contract and wait for the result, read a balance.

import { readFileSync } from "node:fs";
import {
    Contract,
    Networks,
    TransactionBuilder,
    nativeToScVal,
    rpc,
    scValToNative,
} from "@stellar/stellar-sdk";

export const RPC_URL = "https://soroban-testnet.stellar.org";
export const FRIENDBOT = "https://friendbot.stellar.org";

export const server = new rpc.Server(RPC_URL);

const repoRoot = new URL("../../", import.meta.url);

export function loadDeployment(network = "testnet") {
    return JSON.parse(readFileSync(new URL(`deployments/${network}.json`, repoRoot), "utf8"));
}

export async function fundAccount(publicKey) {
    const response = await fetch(`${FRIENDBOT}?addr=${publicKey}`);
    // 400 means the account already exists, which is fine.
    if (!response.ok && response.status !== 400) {
        throw new Error(`friendbot fallo: ${response.status}`);
    }
}

/** Builds, simulates, signs and submits one contract call, and waits for it. */
export async function invoke(keypair, contractId, method, args) {
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
    if (sent.status === "ERROR") {
        throw new Error(`envio rechazado: ${JSON.stringify(sent.errorResult)}`);
    }

    let result = await server.getTransaction(sent.hash);
    while (result.status === "NOT_FOUND") {
        await new Promise((resolve) => setTimeout(resolve, 1000));
        result = await server.getTransaction(sent.hash);
    }
    if (result.status !== "SUCCESS") throw new Error(`transaccion fallida: ${result.status}`);

    return { hash: sent.hash, value: result.returnValue ? scValToNative(result.returnValue) : null };
}

/** Reads a WQ balance by simulation - no transaction, no fee. */
export async function balanceOf(tokenId, address) {
    const simulated = await server.simulateTransaction(
        new TransactionBuilder(await server.getAccount(address), {
            fee: "100000",
            networkPassphrase: Networks.TESTNET,
        })
            .addOperation(new Contract(tokenId).call("balance", nativeToScVal(address, { type: "address" })))
            .setTimeout(30)
            .build(),
    );
    return BigInt(scValToNative(simulated.result.retval));
}

export const wq = (stroops) => `${Number(stroops) / 1e7} WQ`;
