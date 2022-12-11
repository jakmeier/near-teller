use crate::error::Error;
use crate::{env, AccountId, Balance, Near, Teller};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::{testing_env, VMContext};

#[test]
fn test_balance() {
    let app = install();
    assert_eq!(app.hot(), 0);

    fast_forward(10, 13);

    let expected = seconds_to_yocto(13);
    assert_eq!(expected, app.hot());
}

#[test]
fn test_pay() {
    let mut app = install();
    let giga = 1_000_000_000; // to avoid Near fractions
    fast_forward(10 * giga, 13 * giga);
    let tokens = seconds_to_near(giga);

    for _ in 0..5 {
        app.pay(tokens, "max.near".parse().unwrap());
    }

    let expected = seconds_to_yocto(8 * giga);
    assert_eq!(expected, app.hot());

    for _ in 0..8 {
        app.pay(tokens, "max.near".parse().unwrap());
    }
    let expected = 0;
    assert_eq!(expected, app.hot());
}

#[test]
fn test_pay_yocto() {
    let mut app = install();
    fast_forward(10, 13);
    let tokens = seconds_to_yocto(1);
    let token_string = format!("{tokens}");

    for _ in 0..5 {
        app.pay_yocto(&token_string, "max.near".parse().unwrap());
    }

    let expected = seconds_to_yocto(8);
    assert_eq!(expected, app.hot());

    for _ in 0..8 {
        app.pay_yocto(&token_string, "max.near".parse().unwrap());
    }
    let expected = 0;
    assert_eq!(expected, app.hot());
}

#[test]
#[should_panic]
fn test_pay_too_much() {
    let mut app = install();
    fast_forward(50, 65);
    let tokens = seconds_to_near(65) + 1;
    app.pay(tokens, "max.near".parse().unwrap());
}

#[test]
#[should_panic]
fn test_pay_foreign_account() {
    let mut app = install();
    fast_forward(10, 13);
    set_predecessor_account("max.near", false);
    let tokens = seconds_to_near(2);
    app.pay(tokens, "max.near".parse().unwrap());
}

#[test]
fn test_lock() {
    let mut app = install();
    let giga = 1_000_000_000; // to avoid Near fractions
    fast_forward(10 * giga, 13 * giga);
    let tokens = seconds_to_near(1 * giga);

    for _ in 0..5 {
        app.lock(tokens);
    }

    let expected = seconds_to_yocto(8 * giga);
    assert_eq!(expected, app.hot());

    for _ in 0..8 {
        app.lock(tokens);
    }
    let expected = 0;
    assert_eq!(expected, app.hot());
}

#[test]
fn test_lock_yocto() {
    let mut app = install();
    fast_forward(10, 13);
    let tokens = seconds_to_yocto(1);
    let token_string = format!("{tokens}");

    for _ in 0..5 {
        app.lock_yocto(&token_string);
    }

    let expected = seconds_to_yocto(8);
    assert_eq!(expected, app.hot());

    for _ in 0..8 {
        app.lock_yocto(&token_string);
    }
    let expected = 0;
    assert_eq!(expected, app.hot());
}

#[test]
#[should_panic]
fn test_lock_too_much() {
    let mut app = install();
    fast_forward(50, 65);
    let tokens = seconds_to_near(65) + 1;
    app.lock(tokens);
}

#[test]
#[should_panic]
fn test_lock_foreign_account() {
    let mut app = install();
    fast_forward(10, 13);
    set_predecessor_account("max.near", false);
    let tokens = seconds_to_yocto(2);
    app.lock(yocto_to_near(tokens));
}

#[test]
fn test_scenario() {
    let mut app = install();
    let receiver = "max.near".parse().unwrap();

    fast_forward(100, 100);
    let tokens = seconds_to_yocto(30);
    app.pay_impl(tokens, &receiver).expect("access should work");
    app.assert_hot(70);
    app.pay_impl(tokens, &receiver).expect("access should work");
    app.assert_hot(40);

    fast_forward(100, 100);
    set_predecessor_account("max.near", false);
    app.assert_hot(140);
    let err = app.pay_impl(tokens, &receiver).expect_err("should fail");
    assert_eq!(err, Error::ForeignAccountNotAllowed);
    app.assert_hot(140);
    let err = app
        .pay_impl(seconds_to_yocto(500), &receiver)
        .expect_err("should fail");
    assert_eq!(err, Error::ForeignAccountNotAllowed);

    set_predecessor_account("teller.near", false);
    app.pay_impl(tokens, &receiver).expect("access should work");
    app.assert_hot(110);

    let err = app
        .pay_impl(seconds_to_yocto(500), &receiver)
        .expect_err("should fail");
    assert_eq!(err, Error::NotEnoughHot);
    app.assert_hot(110);

    app.pay_impl(seconds_to_yocto(110), &receiver)
        .expect("access should work");
    app.assert_hot(0);
}

fn get_context(is_view: bool) -> VMContext {
    let account_id: AccountId = "teller.near".parse().unwrap();
    VMContextBuilder::new()
        .signer_account_id(account_id.clone())
        .current_account_id(account_id.clone())
        .predecessor_account_id(account_id)
        .account_balance(13000 * 10u128.pow(24))
        .is_view(is_view)
        .build()
}

fn install() -> Teller {
    let context = get_context(false);
    testing_env!(context.clone());
    Teller::init()
}

fn seconds_to_yocto(seconds: u64) -> u128 {
    seconds as u128 * super::CONFIG.nano_near_per_second * 10u128.pow(15)
}

fn yocto_to_near(yocto: Balance) -> Near {
    (yocto / 10u128.pow(24)) as u32
}

fn seconds_to_near(seconds: u64) -> Near {
    yocto_to_near(seconds_to_yocto(seconds))
}

fn fast_forward(blocks: u64, seconds: u64) {
    let is_view = false;
    let mut context = get_context(is_view);
    context.block_timestamp = env::block_timestamp() + seconds * 1_000_000_000;
    context.block_index = env::block_height() + blocks;
    testing_env!(context);
}

fn set_predecessor_account(account_id: &str, is_view: bool) {
    let mut context = get_context(is_view);
    context.block_timestamp = env::block_timestamp();
    context.block_index = env::block_height();
    context.predecessor_account_id = account_id.parse().unwrap();
    testing_env!(context);
}

impl Teller {
    #[track_caller]
    fn assert_hot(&self, seconds: u64) {
        assert_eq!(self.hot(), seconds_to_yocto(seconds));
    }
}
