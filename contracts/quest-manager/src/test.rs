#![cfg(test)]
//! The security properties of QuestManager, exercised as tests.
//!
//! Every signature here is a real ed25519 signature produced by `ed25519-dalek`
//! over the exact bytes the contract rebuilds internally, so these tests fail if
//! the signed message format ever drifts.

use super::*;
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{
    testutils::{Address as _, Events, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, IntoVal,
};
use wq_token::{WQToken, WQTokenClient};

const QUEST_ID: u32 = 1;
const REWARD: i128 = 50_000_000; // 5 WQ at 7 decimals

struct Fixture<'a> {
    env: Env,
    manager_id: Address,
    manager: QuestManagerClient<'a>,
    token_id: Address,
    token: WQTokenClient<'a>,
    owner: Address,
    location: SigningKey,
}

/// A manager wired as the token's admin, with one active quest registered.
fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let token_id = env.register(WQToken, ());
    let manager_id = env.register(QuestManager, ());
    let token = WQTokenClient::new(&env, &token_id);
    let manager = QuestManagerClient::new(&env, &manager_id);

    let owner = Address::generate(&env);
    token.initialize(&manager_id);
    manager.initialize(&owner, &token_id);

    let location = location_key(7);
    manager.register_quest(&QUEST_ID, &public_key(&env, &location), &REWARD);

    Fixture {
        env,
        manager_id,
        manager,
        token_id,
        token,
        owner,
        location,
    }
}

/// Deterministic location key, so a test can name the key it wants.
fn location_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn public_key(env: &Env, key: &SigningKey) -> BytesN<32> {
    BytesN::from_array(env, &key.verifying_key().to_bytes())
}

fn nonce(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Produces the signature a location's backend would hand to `user`, over
/// `quest_id || nonce || user`. Mirrors `QuestManager::visit_message`.
fn sign_visit(
    env: &Env,
    key: &SigningKey,
    quest_id: u32,
    nonce: &BytesN<32>,
    user: &Address,
) -> BytesN<64> {
    let mut message = Bytes::new(env);
    message.extend_from_array(&quest_id.to_be_bytes());
    message.extend_from_array(&nonce.to_array());
    message.append(&user.clone().to_xdr(env));

    let signature = key.sign(message.to_buffer::<256>().as_slice());
    BytesN::from_array(env, &signature.to_bytes())
}

// --- the happy path ---

#[test]
fn verified_visit_mints_the_reward() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let n = nonce(&f.env, 1);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);

    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);

    // Read the events first: `all()` only holds those of the last invocation,
    // and every balance query below is itself an invocation.
    let events = f.env.events().all();
    assert_eq!(
        events.filter_by_contract(&f.manager_id).events().len(),
        1,
        "one QuestCompleted event"
    );
    assert_eq!(
        events.filter_by_contract(&f.token_id).events().len(),
        1,
        "one Mint event"
    );

    assert_eq!(f.token.balance(&alice), REWARD);
    assert_eq!(f.token.total_supply(), REWARD);
}

#[test]
fn the_manager_is_the_only_minter() {
    let f = setup();
    assert_eq!(f.token.admin(), f.manager_id);
}

// --- the three attacks ---

#[test]
fn a_proof_issued_to_another_user_is_rejected() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let mallory = Address::generate(&f.env);
    let n = nonce(&f.env, 2);

    // Signed for alice, submitted by mallory: the address is inside the message,
    // so verification fails. This is the fix for a bearer proof.
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);
    let stolen = f
        .manager
        .try_complete_quest(&mallory, &QUEST_ID, &n, &signature);

    assert!(stolen.is_err(), "a stolen proof must not mint");
    assert_eq!(f.token.balance(&mallory), 0);
    assert_eq!(f.token.total_supply(), 0);
}

#[test]
#[should_panic(expected = "proof already used")]
fn a_replayed_proof_is_rejected() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let n = nonce(&f.env, 3);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);

    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);
    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);
}

#[test]
fn a_signature_from_the_wrong_key_is_rejected() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let n = nonce(&f.env, 4);

    // Correct message, wrong signer: someone who never held the location's key.
    let impostor = location_key(9);
    let signature = sign_visit(&f.env, &impostor, QUEST_ID, &n, &alice);
    let forged = f
        .manager
        .try_complete_quest(&alice, &QUEST_ID, &n, &signature);

    assert!(forged.is_err(), "a forged proof must not mint");
    assert_eq!(f.token.balance(&alice), 0);
}

#[test]
fn a_proof_for_another_quest_is_rejected() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let n = nonce(&f.env, 5);

    // The quest id is inside the message too, so a proof cannot be moved between
    // quests to claim a larger reward.
    let signature = sign_visit(&f.env, &f.location, QUEST_ID + 99, &n, &alice);
    let moved = f
        .manager
        .try_complete_quest(&alice, &QUEST_ID, &n, &signature);

    assert!(moved.is_err(), "a proof for another quest must not mint");
}

// --- quest lifecycle and ownership ---

#[test]
#[should_panic(expected = "quest is not active")]
fn an_inactive_quest_mints_nothing() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let n = nonce(&f.env, 6);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);

    f.manager.set_quest_active(&QUEST_ID, &false);
    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);
}

#[test]
fn only_the_owner_registers_quests() {
    let f = setup();
    let mallory = Address::generate(&f.env);
    let key = public_key(&f.env, &location_key(11));

    // Auth is granted to mallory, not to the owner the contract asks for.
    let rejected = f
        .manager
        .mock_auths(&[MockAuth {
            address: &mallory,
            invoke: &MockAuthInvoke {
                contract: &f.manager_id,
                fn_name: "register_quest",
                args: (2u32, key.clone(), REWARD).into_val(&f.env),
                sub_invokes: &[],
            },
        }])
        .try_register_quest(&2u32, &key, &REWARD);

    assert!(rejected.is_err(), "a stranger must not register quests");
    assert_eq!(f.manager.owner(), f.owner);
}

// --- redemption ---

#[test]
fn redeem_pays_the_merchant_and_burns_the_fee() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);
    let n = nonce(&f.env, 7);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);
    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);

    let spend = 10_000_000i128; // 1 WQ
    let fee = spend * 5 / 100;
    f.manager.redeem(&alice, &merchant, &spend);

    assert_eq!(f.token.balance(&merchant), spend - fee);
    assert_eq!(f.token.balance(&alice), REWARD - spend);
    assert_eq!(
        f.token.total_supply(),
        REWARD - fee,
        "the 5% fee leaves circulation"
    );
}

#[test]
fn redeem_needs_the_user_to_authorize_all_three_calls() {
    // The user's authorization must cover redeem itself plus the token's transfer
    // and burn: that is the tree the frontend has to build and sign.
    let f = setup();
    let alice = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);
    let n = nonce(&f.env, 8);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);
    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);

    let spend = 10_000_000i128;
    let fee = spend * 5 / 100;
    let transfer = MockAuthInvoke {
        contract: &f.token_id,
        fn_name: "transfer",
        args: (alice.clone(), merchant.clone(), spend - fee).into_val(&f.env),
        sub_invokes: &[],
    };
    let burn = MockAuthInvoke {
        contract: &f.token_id,
        fn_name: "burn",
        args: (alice.clone(), fee).into_val(&f.env),
        sub_invokes: &[],
    };

    f.manager
        .mock_auths(&[MockAuth {
            address: &alice,
            invoke: &MockAuthInvoke {
                contract: &f.manager_id,
                fn_name: "redeem",
                args: (alice.clone(), merchant.clone(), spend).into_val(&f.env),
                sub_invokes: &[transfer, burn],
            },
        }])
        .redeem(&alice, &merchant, &spend);

    assert_eq!(f.token.balance(&merchant), spend - fee);
}

#[test]
fn redeem_fails_when_the_burn_node_is_missing() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);
    let n = nonce(&f.env, 9);
    let signature = sign_visit(&f.env, &f.location, QUEST_ID, &n, &alice);
    f.manager.complete_quest(&alice, &QUEST_ID, &n, &signature);

    let spend = 10_000_000i128;
    let fee = spend * 5 / 100;
    let transfer = MockAuthInvoke {
        contract: &f.token_id,
        fn_name: "transfer",
        args: (alice.clone(), merchant.clone(), spend - fee).into_val(&f.env),
        sub_invokes: &[],
    };

    let incomplete = f
        .manager
        .mock_auths(&[MockAuth {
            address: &alice,
            invoke: &MockAuthInvoke {
                contract: &f.manager_id,
                fn_name: "redeem",
                args: (alice.clone(), merchant.clone(), spend).into_val(&f.env),
                sub_invokes: &[transfer],
            },
        }])
        .try_redeem(&alice, &merchant, &spend);

    assert!(
        incomplete.is_err(),
        "an unsigned burn must abort the redeem"
    );
    assert_eq!(f.token.balance(&merchant), 0);
}
