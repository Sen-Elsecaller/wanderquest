#![no_std]
//! QuestManager - the brain of WanderQuest's "cost per verified visit" (CPVV) model.
//!
//! Each quest has a physical location, and each location has an ed25519 key pair.
//! The secret half lives in the location's signing backend; only the public half is
//! stored here. When a user physically reaches the location and scans its QR, the
//! backend signs `quest_id || nonce || user_address` and hands the signature back.
//! The user submits it here, and this contract VERIFIES IT ON-CHAIN before minting WQ.
//!
//! Two properties fall out of that message:
//!
//! - **Only presence mints.** New WQ exists only if someone proved possession of a
//!   signature from a location's signer. QuestManager is the admin of WQToken, so it
//!   is the sole minter.
//! - **A proof is not bearer paper.** The user's address is inside the signed bytes,
//!   so a leaked or forwarded proof is worthless to anyone else. The nonce registry
//!   stops the rightful owner from spending the same proof twice.
//!
//! Redemption (`redeem`) moves WQ from a user to a merchant and burns a 5%
//! recirculation fee, matching the WanderQuest economic model.

use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, xdr::ToXdr, Address,
    Bytes, BytesN, Env,
};

/// The slice of WQToken this contract calls. Declared as a trait rather than by
/// depending on the `wq-token` crate: a crate dependency links the token's own
/// contract exports into this Wasm, where they collide with this contract's
/// (`initialize` has a different arity in each) and one of them is dropped.
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn mint(env: Env, to: Address, amount: i128);
    fn transfer(env: Env, from: Address, to: Address, amount: i128);
    fn burn(env: Env, from: Address, amount: i128);
}

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

/// Emitted when a verified visit mints its reward.
#[contractevent(data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestCompleted {
    #[topic]
    pub user: Address,
    #[topic]
    pub quest_id: u32,
    pub reward: i128,
}

/// Emitted when a user spends WQ at a merchant.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Redeemed {
    #[topic]
    pub user: Address,
    #[topic]
    pub merchant: Address,
    pub amount: i128,
    pub burned: i128,
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
    pub fn register_quest(env: Env, quest_id: u32, location_pubkey: BytesN<32>, reward: i128) {
        assert!(reward > 0, "reward must be positive");
        Self::require_owner(&env).require_auth();
        let quest = Quest {
            location_pubkey,
            reward,
            active: true,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Quest(quest_id), &quest);
    }

    pub fn set_quest_active(env: Env, quest_id: u32, active: bool) {
        Self::require_owner(&env).require_auth();
        let mut quest = Self::get_quest(env.clone(), quest_id);
        quest.active = active;
        env.storage()
            .persistent()
            .set(&DataKey::Quest(quest_id), &quest);
    }

    /// Complete a quest: verify the location's signature over
    /// `quest_id || nonce || user`, then mint the quest reward in WQ to `user`.
    /// Reverts if the quest is inactive, the nonce was already used, or the
    /// signature does not match this exact message.
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

        // On-chain ed25519 verification - panics if the signature is invalid.
        let message = Self::visit_message(&env, quest_id, &nonce, &user);
        env.crypto()
            .ed25519_verify(&quest.location_pubkey, &message, &signature);

        // Mark the proof spent so it can never be replayed.
        env.storage().persistent().set(&nonce_key, &true);

        // Mint the reward. QuestManager is the token admin, so it authorizes
        // this sub-invocation on its own behalf automatically.
        let token = Self::token_client(&env);
        token.mint(&user, &quest.reward);

        QuestCompleted {
            user,
            quest_id,
            reward: quest.reward,
        }
        .publish(&env);
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

        Redeemed {
            user,
            merchant,
            amount,
            burned: burn_amount,
        }
        .publish(&env);
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

    /// The exact bytes a location's signer must sign: `quest_id` big-endian,
    /// then the 32-byte nonce, then the XDR of the claiming user's address.
    /// Binding the address is what keeps a proof from being transferable.
    fn visit_message(env: &Env, quest_id: u32, nonce: &BytesN<32>, user: &Address) -> Bytes {
        let mut message = Bytes::new(env);
        message.extend_from_array(&quest_id.to_be_bytes());
        message.extend_from_array(&nonce.to_array());
        message.append(&user.clone().to_xdr(env));
        message
    }

    fn require_owner(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Owner)
            .expect("not initialized")
    }

    fn token_client(env: &Env) -> TokenClient<'_> {
        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized");
        TokenClient::new(env, &token)
    }
}

mod test;
