#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, MuxedAddress,
};

fn setup() -> (Env, WQTokenClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WQToken, ());
    let client = WQTokenClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn metadata() {
    let (env, client, _admin) = setup();
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.symbol(), String::from_str(&env, "WQ"));
    assert_eq!(client.name(), String::from_str(&env, "WanderQuest"));
}

#[test]
fn mint_transfer_burn() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.mint(&alice, &100);
    assert_eq!(client.balance(&alice), 100);
    assert_eq!(client.total_supply(), 100);

    client.transfer(&alice, MuxedAddress::from(bob.clone()), &30);
    assert_eq!(client.balance(&alice), 70);
    assert_eq!(client.balance(&bob), 30);

    client.burn(&bob, &10);
    assert_eq!(client.balance(&bob), 20);
    assert_eq!(client.total_supply(), 90);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_over_balance_panics() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    client.mint(&alice, &10);
    client.transfer(&alice, MuxedAddress::from(bob), &50);
}

#[test]
#[should_panic(expected = "already initialized")]
fn double_initialize_panics() {
    let (env, client, _admin) = setup();
    let other = Address::generate(&env);
    client.initialize(&other);
}

// --- SEP-41 allowances ---

#[test]
fn approve_then_transfer_from_spends_the_allowance() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);
    let bob = Address::generate(&env);
    client.mint(&alice, &100);

    let expires = env.ledger().sequence() + 1_000;
    client.approve(&alice, &spender, &40, &expires);
    assert_eq!(client.allowance(&alice, &spender), 40);

    client.transfer_from(&spender, &alice, &bob, &25);

    assert_eq!(client.balance(&bob), 25);
    assert_eq!(client.balance(&alice), 75);
    assert_eq!(
        client.allowance(&alice, &spender),
        15,
        "the allowance shrinks by what was spent"
    );
}

#[test]
#[should_panic(expected = "insufficient allowance")]
fn transfer_from_beyond_the_allowance_panics() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);
    let bob = Address::generate(&env);
    client.mint(&alice, &100);

    let expires = env.ledger().sequence() + 1_000;
    client.approve(&alice, &spender, &10, &expires);
    client.transfer_from(&spender, &alice, &bob, &11);
}

#[test]
fn burn_from_spends_the_allowance_and_shrinks_supply() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);
    client.mint(&alice, &100);

    let expires = env.ledger().sequence() + 1_000;
    client.approve(&alice, &spender, &30, &expires);
    client.burn_from(&spender, &alice, &20);

    assert_eq!(client.balance(&alice), 80);
    assert_eq!(client.total_supply(), 80);
    assert_eq!(client.allowance(&alice, &spender), 10);
}

#[test]
fn an_allowance_is_worth_nothing_past_its_ledger() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);
    client.mint(&alice, &100);

    let expires = env.ledger().sequence() + 5;
    client.approve(&alice, &spender, &50, &expires);
    assert_eq!(client.allowance(&alice, &spender), 50);

    env.ledger().set_sequence_number(expires + 1);
    assert_eq!(client.allowance(&alice, &spender), 0);
}

#[test]
#[should_panic(expected = "expired allowance")]
fn approving_into_the_past_panics() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);

    env.ledger().set_sequence_number(100);
    client.approve(&alice, &spender, &50, &99);
}

#[test]
fn a_zero_approval_may_expire_in_the_past() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    let spender = Address::generate(&env);

    // Clearing an allowance must not be blocked by the expiry check, which is
    // what lets a holder revoke one.
    env.ledger().set_sequence_number(100);
    client.approve(&alice, &spender, &0, &0);
    assert_eq!(client.allowance(&alice, &spender), 0);
}

// --- muxed destinations ---

#[test]
fn a_transfer_to_a_muxed_address_credits_the_underlying_account() {
    let (env, client, _admin) = setup();
    let alice = Address::generate(&env);
    client.mint(&alice, &100);

    // Documented pair: this M address carries id 420 over that G account.
    let muxed = MuxedAddress::from_str(
        &env,
        "MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAABUTGI4",
    );
    let account = Address::from_str(
        &env,
        "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ",
    );
    assert_eq!(muxed.id(), Some(420));

    client.transfer(&alice, &muxed, &40);

    assert_eq!(client.balance(&account), 40);
    assert_eq!(client.balance(&alice), 60);
}
