#![no_std]
//! WQToken - the WanderQuest (WQ) fungible token as a Soroban contract.
//!
//! Implements `soroban_sdk::token::TokenInterface` (SEP-41) in full, so wallets,
//! explorers and other contracts handle WQ the same way they handle any Stellar
//! contract token. On top of the standard interface it adds an admin-gated
//! `mint`: in WanderQuest that admin is the QuestManager contract, so the ONLY
//! way new WQ comes into existence is through a cryptographically verified quest
//! completion.
//!
//! Uses 7 decimals to match Stellar's classic asset convention (1 WQ = 1e7 stroops).

use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, token, Address, Env, MuxedAddress, String,
};

const DECIMALS: u32 = 7;

// Storage lifetimes. Balances and the contract instance are kept alive for ~30
// days and bumped whenever they are touched: an archived contract stops
// answering, and an archived balance reads as zero until someone restores it.
const DAY_IN_LEDGERS: u32 = 17_280; // ~5s per ledger
const BALANCE_TTL_THRESHOLD: u32 = DAY_IN_LEDGERS * 20;
const BALANCE_TTL_BUMP: u32 = DAY_IN_LEDGERS * 30;
const INSTANCE_TTL_THRESHOLD: u32 = DAY_IN_LEDGERS * 20;
const INSTANCE_TTL_BUMP: u32 = DAY_IN_LEDGERS * 30;

#[derive(Clone)]
#[contracttype]
pub struct AllowanceKey {
    pub from: Address,
    pub spender: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub live_until_ledger: u32,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    TotalSupply,
    Balance(Address),
    Allowance(AllowanceKey),
}

#[contractevent(data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

/// SEP-41 transfer: `to_muxed_id` carries the multiplexing id when the
/// destination was given as an `M...` address, and is absent otherwise.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub to_muxed_id: Option<u64>,
}

#[contractevent(data_format = "vec")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approve {
    #[topic]
    pub from: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
    pub live_until_ledger: u32,
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

/// The parts of WQ that are not SEP-41: setup, minting and supply.
#[contractimpl]
impl WQToken {
    /// One-time setup. `admin` is the address allowed to mint (the QuestManager).
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);
        Self::bump_instance(&env);
    }

    /// Mint new WQ to `to`. Only the admin (QuestManager) may call this.
    pub fn mint(env: Env, to: Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        let admin = Self::require_admin(&env);
        admin.require_auth();
        Self::bump_instance(&env);

        let new_balance = Self::read_balance(&env, &to) + amount;
        Self::write_balance(&env, &to, new_balance);

        let supply: i128 = Self::total_supply(env.clone()) + amount;
        env.storage().instance().set(&DataKey::TotalSupply, &supply);

        Mint { to, amount }.publish(&env);
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

    // --- internal helpers ---

    fn require_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized")
    }

    fn bump_instance(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
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

    /// Moves `amount` out of `from` and into `to`, with the balance check both
    /// `transfer` and `transfer_from` need.
    fn move_balance(env: &Env, from: &Address, to: &Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        let from_balance = Self::read_balance(env, from);
        assert!(from_balance >= amount, "insufficient balance");

        Self::write_balance(env, from, from_balance - amount);
        let to_balance = Self::read_balance(env, to) + amount;
        Self::write_balance(env, to, to_balance);
    }

    /// Destroys `amount` held by `from` and shrinks total supply.
    fn destroy_balance(env: &Env, from: &Address, amount: i128) {
        assert!(amount > 0, "amount must be positive");
        let from_balance = Self::read_balance(env, from);
        assert!(from_balance >= amount, "insufficient balance");
        Self::write_balance(env, from, from_balance - amount);

        let supply: i128 = Self::total_supply(env.clone()) - amount;
        env.storage().instance().set(&DataKey::TotalSupply, &supply);
    }

    /// Reads a live allowance. An entry whose `live_until_ledger` has passed is
    /// worth zero, whether or not the host has archived it yet.
    fn read_allowance(env: &Env, from: &Address, spender: &Address) -> AllowanceValue {
        let key = DataKey::Allowance(AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        match env.storage().temporary().get::<_, AllowanceValue>(&key) {
            Some(allowance) if allowance.live_until_ledger >= env.ledger().sequence() => allowance,
            _ => AllowanceValue {
                amount: 0,
                live_until_ledger: 0,
            },
        }
    }

    fn write_allowance(
        env: &Env,
        from: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        let current = env.ledger().sequence();
        assert!(
            amount == 0 || live_until_ledger >= current,
            "expired allowance"
        );

        let key = DataKey::Allowance(AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        env.storage().temporary().set(
            &key,
            &AllowanceValue {
                amount,
                live_until_ledger,
            },
        );

        // Keep the entry alive exactly as long as the allowance it holds.
        if amount > 0 {
            let live_for = live_until_ledger - current;
            env.storage()
                .temporary()
                .extend_ttl(&key, live_for, live_for);
        }
    }

    /// Consumes `amount` of the allowance `spender` holds on `from`.
    fn spend_allowance(env: &Env, from: &Address, spender: &Address, amount: i128) {
        let allowance = Self::read_allowance(env, from, spender);
        assert!(allowance.amount >= amount, "insufficient allowance");
        Self::write_allowance(
            env,
            from,
            spender,
            allowance.amount - amount,
            allowance.live_until_ledger,
        );
    }
}

/// SEP-41, as declared by the SDK: implementing the trait is what guarantees the
/// function names, argument types and event shapes match the standard.
#[contractimpl]
impl token::TokenInterface for WQToken {
    fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        Self::read_allowance(&env, &from, &spender).amount
    }

    fn approve(env: Env, from: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        from.require_auth();
        Self::bump_instance(&env);

        Self::write_allowance(&env, &from, &spender, amount, live_until_ledger);

        Approve {
            from,
            spender,
            amount,
            live_until_ledger,
        }
        .publish(&env);
    }

    fn balance(env: Env, id: Address) -> i128 {
        Self::read_balance(&env, &id)
    }

    fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        from.require_auth();
        Self::bump_instance(&env);

        let recipient = to.address();
        Self::move_balance(&env, &from, &recipient, amount);

        Transfer {
            from,
            to: recipient,
            amount,
            to_muxed_id: to.id(),
        }
        .publish(&env);
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        Self::bump_instance(&env);

        Self::spend_allowance(&env, &from, &spender, amount);
        Self::move_balance(&env, &from, &to, amount);

        Transfer {
            from,
            to,
            amount,
            to_muxed_id: None,
        }
        .publish(&env);
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        Self::bump_instance(&env);

        Self::destroy_balance(&env, &from, amount);

        Burn { from, amount }.publish(&env);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        Self::bump_instance(&env);

        Self::spend_allowance(&env, &from, &spender, amount);
        Self::destroy_balance(&env, &from, amount);

        Burn { from, amount }.publish(&env);
    }

    fn decimals(_env: Env) -> u32 {
        DECIMALS
    }

    fn name(env: Env) -> String {
        String::from_str(&env, "WanderQuest")
    }

    fn symbol(env: Env) -> String {
        String::from_str(&env, "WQ")
    }
}

mod test;
