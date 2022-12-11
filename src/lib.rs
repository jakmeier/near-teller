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

mod error;
mod implementation;
#[cfg(test)]
mod unit_tests;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Balance};

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

// Public API of the contract.
//
// Everything in here operates in an untrusted environment and we must not
// update Teller's state from here. Always delegate to the `implementation`
// module for that.
//
// The code in here is basically just parsing the input arguments and converting
// them to convenient types.
#[near_bindgen]
impl Teller {
    /// Send Near tokens to an account. Only whole Near values are supported.
    pub fn pay(&mut self, n: Near, a: AccountId) {
        let yocto = n as u128 * 10u128.pow(24);
        let receiver = &a;
        if let Err(e) = self.pay_impl(yocto, receiver) {
            e.panic()
        }
    }

    /// Send Near tokens to an account. Amount is specified in yocto Near.
    pub fn pay_yocto(&mut self, yocto: &String, a: AccountId) {
        let yocto: u128 = yocto.parse().expect("could not parse input yocto");
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

    /// Make Near tokens unavailable for retrieval from hot wallet. Amount is specified in yocto Near.
    pub fn lock_yocto(&mut self, yocto: &String) {
        let yocto: u128 = yocto.parse().expect("could not parse input yocto");
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
        let staking_pool = select_staking_pool(i as usize);
        let yocto = n as u128 * 10u128.pow(24);

        if let Err(e) = self.stake_impl(yocto, &staking_pool) {
            e.panic()
        }
    }

    /// Stake with validator[i].Amount is specified in yocto Near.
    pub fn stake_yocto(&mut self, i: u32, yocto: &String) {
        let staking_pool = select_staking_pool(i as usize);
        let yocto: u128 = yocto.parse().expect("could not parse input yocto");

        if let Err(e) = self.stake_impl(yocto, &staking_pool) {
            e.panic()
        }
    }

    /// Unstake and withdraw all balance staked with validator[i].
    pub fn unstake(&mut self, i: u32) {
        let staking_pool = select_staking_pool(i as usize);

        if let Err(e) = self.unstake_impl(&staking_pool) {
            e.panic()
        }
    }

    /// Withdraw all balance staked with validator[i].
    pub fn withdraw(&mut self, i: u32) {
        let staking_pool = select_staking_pool(i as usize);

        if let Err(e) = self.withdraw_impl(&staking_pool) {
            e.panic()
        }
    }
}

fn select_staking_pool(i: usize) -> AccountId {
    // safety: rust will panic on out-of-bound access
    let staking_pool_str = CONFIG.staking_pools[i];
    let Ok(staking_pool) = staking_pool_str.parse() else {
        env::panic_str("invalid pre-installed account");
    };
    staking_pool
}
