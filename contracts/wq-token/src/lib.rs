#![no_std]
//! WQToken - the WanderQuest (WQ) fungible token as a Soroban contract.
//!
//! Minimal, self-contained fungible token: metadata, balances, transfer, mint,
//! and burn. `mint` is gated to an `admin` address - in WanderQuest that admin
//! is the QuestManager contract, so the ONLY way new WQ comes into existence is
//! through a cryptographically verified quest completion.
//!
//! Uses 7 decimals to match Stellar's classic asset convention (1 WQ = 1e7 stroops).
//!
//! Event shapes follow SEP-41: topics `["transfer", from, to]`, `["burn", from]`
//! and `["mint", to]`, each carrying `amount: i128` as data.

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, String};

const DECIMALS: u32 = 7;

// Persistent balances: keep entries alive for ~30 days, bump when near expiry.
const DAY_IN_LEDGERS: u32 = 17_280; // ~5s per ledger
const BALANCE_TTL_THRESHOLD: u32 = DAY_IN_LEDGERS * 20;
const BALANCE_TTL_BUMP: u32 = DAY_IN_LEDGERS * 30;

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    TotalSupply,
    Balance(Address),
}

#[contractevent(data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Burn {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contract]
pub struct WQToken;

#[contractimpl]
impl WQToken {
    /// One-time setup. `admin` is the address allowed to mint (the QuestManager).
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);
    }

    /// Mint new WQ to `to`. Only the admin (QuestManager) may call this.
    pub fn mint(env: Env, to: Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        let admin = Self::require_admin(&env);
        admin.require_auth();

        let new_balance = Self::read_balance(&env, &to) + amount;
        Self::write_balance(&env, &to, new_balance);

        let supply: i128 = Self::total_supply(env.clone()) + amount;
        env.storage().instance().set(&DataKey::TotalSupply, &supply);

        Mint { to, amount }.publish(&env);
    }

    /// Transfer WQ from `from` to `to`. Requires `from`'s authorization.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        from.require_auth();

        let from_balance = Self::read_balance(&env, &from);
        assert!(from_balance >= amount, "insufficient balance");

        Self::write_balance(&env, &from, from_balance - amount);
        let to_balance = Self::read_balance(&env, &to) + amount;
        Self::write_balance(&env, &to, to_balance);

        Transfer { from, to, amount }.publish(&env);
    }

    /// Burn WQ from `from`. Requires `from`'s authorization. Reduces total supply.
    pub fn burn(env: Env, from: Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        from.require_auth();

        let from_balance = Self::read_balance(&env, &from);
        assert!(from_balance >= amount, "insufficient balance");
        Self::write_balance(&env, &from, from_balance - amount);

        let supply: i128 = Self::total_supply(env.clone()) - amount;
        env.storage().instance().set(&DataKey::TotalSupply, &supply);

        Burn { from, amount }.publish(&env);
    }

    // --- read-only views ---

    pub fn balance(env: Env, id: Address) -> i128 {
        Self::read_balance(&env, &id)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        Self::require_admin(&env)
    }

    pub fn decimals(_env: Env) -> u32 {
        DECIMALS
    }

    pub fn name(env: Env) -> String {
        String::from_str(&env, "WanderQuest")
    }

    pub fn symbol(env: Env) -> String {
        String::from_str(&env, "WQ")
    }

    // --- internal helpers ---

    fn require_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized")
    }

    fn read_balance(env: &Env, id: &Address) -> i128 {
        let key = DataKey::Balance(id.clone());
        if let Some(b) = env.storage().persistent().get::<DataKey, i128>(&key) {
            env.storage()
                .persistent()
                .extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_TTL_BUMP);
            b
        } else {
            0
        }
    }

    fn write_balance(env: &Env, id: &Address, amount: i128) {
        let key = DataKey::Balance(id.clone());
        env.storage().persistent().set(&key, &amount);
        env.storage()
            .persistent()
            .extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_TTL_BUMP);
    }
}

mod test;
