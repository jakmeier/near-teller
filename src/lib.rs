//! A token guarding contract for an account with a cold full access key and hot function call access keys.
//!
//! The features are simple and few by design. Configuration changes are only possible by redeploying.
//!
//! Possible actions are:
//!
//! 1. Pay: Send `arg.N` tokens to `arg.account`.
//! 2. Lock: Forgo `arg.N` tokens that can no longer be retrieved by 1.
//! 3. Stake: Call `deposit_and_stake` on `CONFIG.staking_pools[arg.staking_pool]` and attach `arg.N` tokens.
//! 4. Unstake: Call `unstake_all` on `CONFIG.staking_pools[arg.staking_pool]`.
//! 5. Unstake: Call `withdraw_all` on `CONFIG.staking_pools[arg.staking_pool]`.
//!
//! Pay and lock are limited by how many tokens are unlocked for hot wallet access.
//! Staking is unlimited. (Besides the external limit of actual tokens in the account.)
//!
//! Rationale:
//!
//! - No dynamic staking: Calling a method with the name `deposit_and_stake` on an arbitrary account makes it possible to retrieve all tokens with hot key.
//! - No dynamic rate change: Necessary allowance computation makes code more complicated.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Balance, Gas, GasWeight};

type Result<T> = std::result::Result<T, Error>;
type Near = u32;

struct Config {
    nano_near_per_second: u128,
    staking_pools: [&'static str; 10],
}

const CONFIG: Config = include!("config.ron");

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, near_sdk::PanicOnDefault)]
pub struct Teller {
    /// Initial timestamp (ns) from which the allowance is computed from.
    t0: u64,
    /// yocto NEAR either retrieved or forgone.
    locked: u128,
}

#[near_bindgen]
impl Teller {
    /// Called after deployment, if redeployed, delete account first.
    #[init]
    pub fn init() -> Self {
        Self {
            t0: env::block_timestamp(),
            locked: 0,
        }
    }

    /// Send Near tokens to an account. Only whole Near values are supported.
    pub fn pay(&mut self, n: Near, a: AccountId) {
        let yocto = n as u128 * 10u128.pow(24);
        let receiver = &a;
        if let Err(e) = self.pay_impl(yocto, receiver) {
            e.panic()
        }
    }

    /// Make Near tokens unavailable for retrieval from hot wallet. Only whole Near values.
    pub fn lock(&mut self, n: Near) {
        let yocto = n as u128 * 10u128.pow(24);
        if let Err(e) = self.lock_impl(yocto) {
            e.panic()
        }
    }

    /// Available balance in yocto Near.
    pub fn hot(&self) -> Balance {
        let ns = env::block_timestamp() - self.t0;
        // nano = e-9, yocto = e-24
        // ns * nNEAR/s = n^2NEAR = NEAR * e-18
        // need to multiply with e+6 to return in yocto
        let available_ever = ns as u128 * CONFIG.nano_near_per_second * 10u128.pow(6);
        available_ever - self.locked
    }

    /// Stake with validator[i].
    pub fn stake(&mut self, i: u32, n: Near) {
        let staking_pool = staking_pool(i as usize);
        let yocto = n as u128 * 10u128.pow(24);

        if let Err(e) = self.stake_impl(yocto, &staking_pool) {
            e.panic()
        }
    }

    /// Unstake and withdraw all balance staked with validator[i].
    pub fn unstake(&mut self, i: u32) {
        let staking_pool = staking_pool(i as usize);

        if let Err(e) = self.unstake_impl(&staking_pool) {
            e.panic()
        }
    }

    /// Withdraw all balance staked with validator[i].
    pub fn withdraw(&mut self, i: u32) {
        let staking_pool = staking_pool(i as usize);

        if let Err(e) = self.withdraw_impl(&staking_pool) {
            e.panic()
        }
    }
}

impl Teller {
    fn pay_impl(&mut self, yocto: Balance, receiver: &AccountId) -> Result<()> {
        Self::check_access()?;
        self.try_lock(yocto)?;

        let index: u64 = env::promise_batch_create(receiver);
        env::promise_batch_action_transfer(index, yocto);

        Ok(())
    }

    fn lock_impl(&mut self, n: Balance) -> Result<()> {
        Self::check_access()?;
        self.try_lock(n)?;
        Ok(())
    }

    fn stake_impl(&mut self, yocto: Balance, staking_pool: &AccountId) -> Result<()> {
        Self::check_access()?;
        let index: u64 = env::promise_batch_create(&staking_pool);
        env::promise_batch_action_function_call_weight(
            index,
            "deposit_and_stake",
            &[],
            yocto,
            Gas(0),
            GasWeight(1),
        );
        Ok(())
    }

    fn unstake_impl(&mut self, staking_pool: &AccountId) -> Result<()> {
        Self::check_access()?;
        let index: u64 = env::promise_batch_create(&staking_pool);
        let attached_balance = 0;
        env::promise_batch_action_function_call_weight(
            index,
            "unstake_all",
            &[],
            attached_balance,
            Gas(0),
            GasWeight(1),
        );
        Ok(())
    }

    fn withdraw_impl(&mut self, staking_pool: &AccountId) -> Result<()> {
        Self::check_access()?;
        let index: u64 = env::promise_batch_create(&staking_pool);
        let attached_balance = 0;
        env::promise_batch_action_function_call_weight(
            index,
            "withdraw_all",
            &[],
            attached_balance,
            Gas(0),
            GasWeight(1),
        );
        Ok(())
    }

    fn check_access() -> Result<()> {
        if env::current_account_id() == env::predecessor_account_id() {
            Ok(())
        } else {
            Err(Error::ForeignAccountNotAllowed)
        }
    }

    fn try_lock(&mut self, yocto: Balance) -> Result<()> {
        if self.hot() < yocto {
            Err(Error::NotEnoughHot)
        } else {
            self.locked += yocto;
            Ok(())
        }
    }
}

fn staking_pool(i: usize) -> AccountId {
    // safety: rust will panic on out-of-bound access
    let staking_pool_str = CONFIG.staking_pools[i];
    let Ok(staking_pool) = staking_pool_str.parse() else {
        env::panic_str("invalid pre-installed account");
    };
    staking_pool
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
enum Error {
    NotEnoughHot,
    ForeignAccountNotAllowed,
}

impl Error {
    fn as_str(&self) -> &'static str {
        match self {
            Error::NotEnoughHot => "not enough hot tokens",
            Error::ForeignAccountNotAllowed => "must be called by contract account",
        }
    }

    fn panic(&self) -> ! {
        env::panic_str(self.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
