use serde_json::json;
use workspaces::result::{ExecutionFinalResult, ViewResultDetails};
use workspaces::types::{KeyType, SecretKey};
use workspaces::{AccountId, Contract, DevNetwork, Worker};

async fn init(worker: &Worker<impl DevNetwork>) -> anyhow::Result<Contract> {
    let teller_contract = worker
        .dev_deploy(include_bytes!("../res/near_teller.wasm"))
        .await?;

    let res = teller_contract.call("init").max_gas().transact().await?;
    assert!(res.is_success(), "{res:?}");

    return Ok(teller_contract);
}

/// optional: clean up dev-account contract
///
/// The main motivation is to cleanup the testnet on-chain state. We don't want
/// it to grow for no reason, so let's just cleanup as good practice. When
/// deleting an account, send tokens to my account because why not.
async fn cleanup_account(contract: Contract) {
    _ = contract
        .delete_contract(&"jakmeier.testnet".parse().unwrap())
        .await;
}

/// Read the receiver ID from a cross contract call result.
fn cross_contract_call_receiver(result: &ExecutionFinalResult) -> AccountId {
    let receipts = result.receipt_outcomes();
    assert!(receipts.len() > 2, "{result:?}");
    let stake_call_receipt = &receipts[1];
    stake_call_receipt.executor_id.clone()
}

/// On a staking pool contract, look up the staked balance for an account.
async fn view_staked_account_balance(
    worker: &Worker<impl DevNetwork>,
    pool: AccountId,
    account_id: &AccountId,
) -> ViewResultDetails {
    let arg = json!({ "account_id": account_id });
    view_call(worker, pool, arg, "get_account_staked_balance").await
}

/// On a staking pool contract, look up the unstaked balance for an account.
async fn view_unstaked_account_balance(
    worker: &Worker<impl DevNetwork>,
    pool: AccountId,
    account_id: &AccountId,
) -> ViewResultDetails {
    let arg = json!({ "account_id": account_id });
    view_call(worker, pool, arg, "get_account_unstaked_balance").await
}

async fn view_call(
    worker: &Worker<impl DevNetwork>,
    pool: AccountId,
    arg: serde_json::Value,
    method: &str,
) -> ViewResultDetails {
    // create dummy `Contract` for pool to make a view call on
    let pool = Contract::from_secret_key(
        pool,
        SecretKey::from_seed(KeyType::ED25519, "no-key-require-for-view-call"),
        &worker,
    );

    pool.view(method, arg.to_string().into_bytes())
        .await
        .expect("view call failed")
}

#[tokio::test]
async fn test_init() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let contract = init(&worker).await?;

    let res = contract.call("hot").view().await?;
    let res_str = std::str::from_utf8(&res.result)?;
    let yocto: u128 = res_str.parse()?;
    // between init and this call, there must have been multiple blocks, so
    // there should be something available
    assert!(yocto > 0, "{res:?}");

    Ok(())
}

#[tokio::test]
async fn test_stake() -> anyhow::Result<()> {
    let worker = workspaces::testnet_archival().await?;
    let contract = init(&worker).await?;
    let contract_account_id = contract.id().clone();

    // do the stake call under test
    let stake_res = contract
        .call("stake")
        .args_json(json!({
            "i": 0,
            "n": 1,
        }))
        .max_gas()
        .transact()
        .await?;
    assert!(stake_res.is_success(), "{stake_res:?}");

    // lookup account that the staking function call went to
    let stake_pool_id = cross_contract_call_receiver(&stake_res);

    // check that staked value is what was sent before - 3 yocto NEAR
    let view_res = view_staked_account_balance(&worker, stake_pool_id, &contract_account_id).await;
    assert_eq!(
        view_res.result, b"\"999999999999999999999997\"",
        "{stake_res:?}\n {view_res:?}"
    );

    cleanup_account(contract).await;
    Ok(())
}

#[tokio::test]
async fn test_stake_unstake() -> anyhow::Result<()> {
    let worker = workspaces::testnet_archival().await?;
    let contract = init(&worker).await?;
    let contract_account_id = contract.id().clone();
    let pool_id = 0;

    // prepare with a stake call
    let stake_res = contract
        .call("stake")
        .args_json(json!({
            "i": pool_id,
            "n": 1,
        }))
        .max_gas()
        .transact()
        .await?;
    assert!(stake_res.is_success(), "{stake_res:?}");

    // do the unstake call
    let unstake_res = contract
        .call("unstake")
        .args_json(json!({
            "i": pool_id,
        }))
        .max_gas()
        .transact()
        .await?;
    assert!(unstake_res.is_success(), "{unstake_res:?}");

    // lookup account that the unstaking function call went to
    let stake_pool_id = cross_contract_call_receiver(&unstake_res);

    // ensure nothing is staked after unstaking
    let view_res =
        view_staked_account_balance(&worker, stake_pool_id.clone(), &contract_account_id).await;
    assert_eq!(view_res.result, b"\"0\"", "{unstake_res:?}\n {view_res:?}");

    // ensure unstaked balance is still available
    let view_res =
        view_unstaked_account_balance(&worker, stake_pool_id.clone(), &contract_account_id).await;
    assert_eq!(
        // I don't understand why it is + 1 yocto NEAR but that's what I
        // observed and I expect it to be consistent.
        view_res.result,
        b"\"1000000000000000000000001\"",
        "{unstake_res:?}\n {view_res:?}"
    );

    cleanup_account(contract).await;
    Ok(())
}
