# NEAR Teller

NEAR Teller keeps the bulk of your tokens safe behind a full access key which
you should store in a cold wallet. You can then add a function access key that
allows unlimited staking and limited withdrawals from a hot wallet.

## Risks

- This code was not audited.
- This code was not written by a professional smart contract developer.
- No claims about correctness of the code are made.
- Using this contract improperly may result in permanent loss of access to your assets.

Recommendation: The complete contract code is only about 100 lines of code, most
of which is trivial. Audit it for yourself, adapt it where you see fit, and only
use it if you fully understand it. If you are not comfortable doing that on your
own, get someone professional who can do that for you.

## Usage

1. Configure, compile and deploy this contract.
2. Add a function call key to your account that has itself as the receiver.
3. Store you full access key away safely in a cold wallet.
4. Use the function call key to manage tokens sent to your account.
    - Stake arbitrary amounts of tokens.
    - Retrieve a limited amount of tokens. The amount increases at a constant rate over time.

### Contract Methods

- `hot()` is a view call that returns the balance in yocto Near currently available to access from a hot wallet.
- `pay(n: Near, a: AccountId)` sends tokens to an account and reduces the amount accessible from your hot wallet.
- `lock(n: Near)` reduces the amount accessible from your hot wallet.
- `stake(i: u32, n: Near)`: stakes tokens with a staking pool without changing the amount accessible from your hot wallet.

### Config

Set parameters in [`config.ron`](./src/config.ron):

```rust
Config {
    // Set how many yocto NEAR per second should be available through function calls.
    nano_near_per_second: 100_000_000_000_000_000,
    // pick staking pools you trust
    // https://explorer.near.org/nodes/validators
    staking_pools: [
        "YOUR-FAVOURITE-VALIADTOR-0.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-1.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-2.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-3.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-4.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-5.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-6.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-7.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-8.poolv1.near",
        "YOUR-FAVOURITE-VALIADTOR-9.poolv1.near",
    ],
}
```

Then compile it using `make res/near_teller.wasm`.

The configuration options are now fixed inside the WASM.
They can only be changed by recompiling and redeploying.

### Limitations

This contract, on purpose, does now allow:
- more granular access than whole NEAR
- restaking / unstaking without full access key
- staking with staking pools not listed in config
- changing configs without redeploying the entire contract

It would be easy to add such functionality in a fork, feel free to do so. But
this repository contains a minimal contract by design.

## License

Distributed with both the MIT license and the Apache License (Version 2.0) at your choice.

Basically, do whit this code anything you like. **But at your own risk.**

## Disclaimers

This is a private project by me, jakmeier, to learn and build a simple but useful
smart contract. It is not endorsed by any companies or organizations that I am
or was associated with.

This is the first real smart contract I implemented. Hence I do not have the
necessary experience or expertise to write contracts that move larger amounts of
money than you are willing to lose. Use with discretion.
