#![no_std]
//! QuestManager — the brain of WanderQuest's "cost per verified visit" (CPVV) model.
//!
//! A quest has a physical location. Each location holds a secret ed25519 key
//! (embedded in its QR / handed to its NFC tag / held by the sponsor's backend).
//! When a user physically reaches the location and scans it, an off-chain signer
//! produces an ed25519 signature over `quest_id || nonce`. The user submits that
//! signature here, and this contract VERIFIES IT ON-CHAIN before minting WQ.
//!
//! So new WQ can only be created by proving possession of a location's secret —
//! i.e. by actually being there. QuestManager is the admin of the WQToken, so it
//! is the sole minter.
//!
//! Redemption (`redeem`) moves WQ from a user to a merchant and burns a 5%
//! recirculation fee, matching the WanderQuest economic model.
//!
//! NOTE (MVP scope): a signed proof is bound to the quest, not to a specific user,
//! so whoever submits a fresh proof first claims it. Production fix: include the
//! user's key in the signed message. The nonce registry below still prevents any
//! proof from being replayed twice.

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env,
};
use wq_token::WQTokenClient;

// Fee, in basis points, burned on redemption (5%).
const REDEEM_BURN_BPS: i128 = 500;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Owner,
    Token,
    Quest(u32),
    UsedNonce(u32, BytesN<32>),
}

#[derive(Clone)]
#[contracttype]
pub struct Quest {
    pub location_pubkey: BytesN<32>,
    pub reward: i128,
    pub active: bool,
}

#[contract]
pub struct QuestManager;

#[contractimpl]
impl QuestManager {
    /// One-time setup. `owner` may register quests; `token` is the WQToken this
    /// manager administers (WQToken must be initialized with this contract's
    /// address as its admin).
    pub fn initialize(env: Env, owner: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Owner) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    /// Register (or overwrite) a quest and the ed25519 public key of its location.
    pub fn register_quest(
        env: Env,
        quest_id: u32,
        location_pubkey: BytesN<32>,
        reward: i128,
    ) {
        assert!(reward > 0, "reward must be positive");
        Self::require_owner(&env).require_auth();
        let quest = Quest {
            location_pubkey,
            reward,
            active: true,
        };
        env.storage().persistent().set(&DataKey::Quest(quest_id), &quest);
    }

    pub fn set_quest_active(env: Env, quest_id: u32, active: bool) {
        Self::require_owner(&env).require_auth();
        let mut quest = Self::get_quest(env.clone(), quest_id);
        quest.active = active;
        env.storage().persistent().set(&DataKey::Quest(quest_id), &quest);
    }

    /// Complete a quest: verify the location's signature over `quest_id || nonce`,
    /// then mint the quest reward in WQ to `user`. Reverts if the quest is
    /// inactive, the nonce was already used, or the signature is invalid.
    pub fn complete_quest(
        env: Env,
        user: Address,
        quest_id: u32,
        nonce: BytesN<32>,
        signature: BytesN<64>,
    ) {
        user.require_auth();

        let quest = Self::get_quest(env.clone(), quest_id);
        assert!(quest.active, "quest is not active");

        let nonce_key = DataKey::UsedNonce(quest_id, nonce.clone());
        assert!(
            !env.storage().persistent().has(&nonce_key),
            "proof already used"
        );

        // Reconstruct the signed message: quest_id (big-endian) || nonce.
        let mut message = Bytes::new(&env);
        message.extend_from_array(&quest_id.to_be_bytes());
        message.extend_from_array(&nonce.to_array());

        // On-chain ed25519 verification — panics if the signature is invalid.
        env.crypto()
            .ed25519_verify(&quest.location_pubkey, &message, &signature);

        // Mark the proof spent so it can never be replayed.
        env.storage().persistent().set(&nonce_key, &true);

        // Mint the reward. QuestManager is the token admin, so it authorizes
        // this sub-invocation on its own behalf automatically.
        let token = Self::token_client(&env);
        token.mint(&user, &quest.reward);

        env.events()
            .publish((symbol_short!("completed"), user), quest_id);
    }

    /// Spend WQ at a merchant. Transfers 95% to the merchant and burns 5% as the
    /// recirculation fee. Requires the user's authorization.
    pub fn redeem(env: Env, user: Address, merchant: Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        user.require_auth();

        let burn_amount = amount * REDEEM_BURN_BPS / 10_000;
        let pay_amount = amount - burn_amount;

        let token = Self::token_client(&env);
        token.transfer(&user, &merchant, &pay_amount);
        if burn_amount > 0 {
            token.burn(&user, &burn_amount);
        }

        env.events()
            .publish((symbol_short!("redeem"), user, merchant), amount);
    }

    // --- views ---

    pub fn get_quest(env: Env, quest_id: u32) -> Quest {
        env.storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("quest not found")
    }

    pub fn token(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized")
    }

    pub fn owner(env: Env) -> Address {
        Self::require_owner(&env)
    }

    // --- helpers ---

    fn require_owner(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Owner)
            .expect("not initialized")
    }

    fn token_client(env: &Env) -> WQTokenClient {
        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized");
        WQTokenClient::new(env, &token)
    }
}

mod test;
