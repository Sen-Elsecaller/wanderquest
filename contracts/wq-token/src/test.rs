#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

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

    client.transfer(&alice, &bob, &30);
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
    client.transfer(&alice, &bob, &50);
}

#[test]
#[should_panic(expected = "already initialized")]
fn double_initialize_panics() {
    let (env, client, _admin) = setup();
    let other = Address::generate(&env);
    client.initialize(&other);
}
