//! This module contains all the core implementation of each method.
//!
//! All the interesting code, a.k.a. business logic, happens in this module.
//! Type checks and unit conversion happen before calling into the core
//! implementations, so the code in this module can focus on the logical
//! components.
//!
//! Access permission checks are done in this module as well. This makes it easy
//! to check that any method that changes internal state does have access check
//! in place.

use crate::error::Error;
use crate::{Teller, TellerExt};
use near_sdk::{env, near_bindgen, AccountId, Balance, Gas, GasWeight};

type Result<T> = std::result::Result<T, Error>;

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
}

impl Teller {
    pub(crate) fn pay_impl(&mut self, yocto: Balance, receiver: &AccountId) -> Result<()> {
        Self::check_access()?;
        self.try_lock(yocto)?;

        let index: u64 = env::promise_batch_create(receiver);
        env::promise_batch_action_transfer(index, yocto);

        Ok(())
    }

    pub(crate) fn lock_impl(&mut self, n: Balance) -> Result<()> {
        Self::check_access()?;
        self.try_lock(n)?;
        Ok(())
    }

    pub(crate) fn stake_impl(&mut self, yocto: Balance, staking_pool: &AccountId) -> Result<()> {
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

    pub(crate) fn unstake_impl(&mut self, staking_pool: &AccountId) -> Result<()> {
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

    pub(crate) fn withdraw_impl(&mut self, staking_pool: &AccountId) -> Result<()> {
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

    /// Only allow functions to be called directly, not via cross function call.
    ///
    /// This is very important to check, as otherwise anyone could call into
    /// teller's methods without any access permission checks!
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
