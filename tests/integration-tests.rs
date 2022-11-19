use serde_json::json;
use workspaces::types::{KeyType, SecretKey};
use workspaces::{Contract, DevNetwork, Worker};

async fn init(worker: &Worker<impl DevNetwork>) -> anyhow::Result<Contract> {
    let teller_contract = worker
        .dev_deploy(include_bytes!("../res/near_teller.wasm"))
        .await?;

    let res = teller_contract.call("init").max_gas().transact().await?;
    assert!(res.is_success(), "{res:?}");

    return Ok(teller_contract);
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
    let receipts = stake_res.receipt_outcomes();
    assert!(receipts.len() > 2, "{stake_res:?}");
    let stake_call_receipt = &receipts[1];
    let stake_pool_id = stake_call_receipt.executor_id.clone();

    // optional: clean up contract and send tokens to my account, (owning more
    // testnet tokens is never wrong)
    _ = contract
        .delete_contract(&"jakmeier.testnet".parse().unwrap())
        .await;

    // create dummy `Contract` for pool to make a view call on
    let pool = Contract::from_secret_key(
        stake_pool_id,
        SecretKey::from_seed(KeyType::ED25519, "no-key-require-for-view-call"),
        &worker,
    );

    // check that staked value is what was sent before - 3 yocto NEAR
    let arg = json!({ "account_id": contract_account_id });
    let view_res = pool
        .view("get_account_staked_balance", arg.to_string().into_bytes())
        .await?;
    assert_eq!(
        view_res.result, b"\"999999999999999999999997\"",
        "{stake_res:?}\n {view_res:?}"
    );
    Ok(())
}
