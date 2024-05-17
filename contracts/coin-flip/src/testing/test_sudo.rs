use std::vec;

use cosmwasm_std::{to_json_binary, Addr, Uint128, WasmMsg};
use cw_multi_test::Executor;
use sg_std::NATIVE_DENOM;

use crate::{
    error::ContractError,
    testing::utils::{
        executes::{sudo_transfer_nft, sudo_withdraw_nft_from_pool},
        queries::query_nft_owner,
        setup::{CREATOR_ADDR, MIN_BANK_AMOUNT, TEAM_ADDR},
    },
    types::StreakReward,
};

use super::utils::{
    executes::{execute_do_flips, sudo_update_streak_config, sudo_withdraw_excess},
    helpers::{add_10_todo_flips, add_balance},
    queries::{query_fees, query_nft_pool},
    setup::{setup_base_contract, setup_with_nft_pool, RESERVE_ADDR},
};

#[test]
fn test_withdraw_nft_from_pool() {
    let (mut app, contract_addr, _, _) = setup_with_nft_pool();
    let pool = query_nft_pool(&app, contract_addr.clone()).unwrap();

    assert_eq!(pool.len(), 4);

    // Make sure the owner of the NFTs is the coin flip contract
    for nft in &pool {
        let owner = query_nft_owner(&app, nft.contract_addr.clone(), nft.token_id.clone()).unwrap();
        assert_eq!(owner.owner, contract_addr)
    }

    // Try withdraw NFT out of index
    let err =
        sudo_withdraw_nft_from_pool(&mut app, contract_addr.clone(), Some(100), None).unwrap_err();
    assert_eq!(err, ContractError::NftIndexOutOfRange);

    // Must include either an index, or the all flag
    let err = sudo_withdraw_nft_from_pool(&mut app, contract_addr.clone(), None, None).unwrap_err();
    assert_eq!(err, ContractError::EmptyWithdrawParams);

    // Lets withdraw first NFT and make sure the owner is the team wallet
    sudo_withdraw_nft_from_pool(&mut app, contract_addr.clone(), Some(0), None).unwrap();

    let owner = query_nft_owner(
        &app,
        pool[0].contract_addr.clone(),
        pool[0].token_id.clone(),
    )
    .unwrap();
    assert_eq!(owner.owner, TEAM_ADDR);

    // Lets withdraw all NFTs and make sure the owner is the team wallet
    let pool = query_nft_pool(&app, contract_addr.clone()).unwrap();
    assert_eq!(pool.len(), 3);

    sudo_withdraw_nft_from_pool(&mut app, contract_addr.clone(), None, Some(true)).unwrap();

    for nft in &pool {
        let owner = query_nft_owner(&app, nft.contract_addr.clone(), nft.token_id.clone()).unwrap();
        assert_eq!(owner.owner, TEAM_ADDR)
    }

    // Make sure the pool is empty
    let pool = query_nft_pool(&app, contract_addr).unwrap();
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_withdraw_excess_funds() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 35000000000);

    // do some flips
    add_10_todo_flips(&mut app, contract_addr.clone());
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let fees_amount = query_fees(&app, contract_addr.clone(), NATIVE_DENOM).unwrap();
    let old_contract_balance = app
        .wrap()
        .query_balance(contract_addr.clone(), NATIVE_DENOM)
        .unwrap();
    let old_reserve_balance = app
        .wrap()
        .query_balance(RESERVE_ADDR, NATIVE_DENOM)
        .unwrap();
    let excess_funds = old_contract_balance.amount - fees_amount - MIN_BANK_AMOUNT;

    // Withdraw excess funds
    sudo_withdraw_excess(&mut app, contract_addr.clone(), NATIVE_DENOM).unwrap();

    let new_contract_balance = app
        .wrap()
        .query_balance(contract_addr.clone(), NATIVE_DENOM)
        .unwrap();
    let new_reserve_balance = app
        .wrap()
        .query_balance(RESERVE_ADDR, NATIVE_DENOM)
        .unwrap();

    // Make sure the contract balance is correct (old balance - excess funds)
    assert_eq!(
        new_contract_balance.amount,
        old_contract_balance.amount - excess_funds
    );

    // Make sure the reserve now holds the excess funds
    assert_eq!(
        new_reserve_balance.amount,
        old_reserve_balance.amount + excess_funds
    );

    // Try withdraw when there are no excess funds
    let err = sudo_withdraw_excess(&mut app, contract_addr, NATIVE_DENOM).unwrap_err();
    assert_eq!(err, ContractError::NoExcessFunds)
}

#[test]
fn test_transfer_nft() {
    let (mut app, contract_addr, nft_contract1, _) = setup_with_nft_pool();
    let pool = query_nft_pool(&app, contract_addr.clone()).unwrap();
    assert_eq!(pool.len(), 4);

    // Try to transfer an NFt that is in the pool
    let err = sudo_transfer_nft(
        &mut app,
        contract_addr.clone(),
        pool[0].contract_addr.as_str(),
        pool[0].token_id.as_str(),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::NftInPool);

    // Try transfer NFT that was sent by mistake
    let token_id = "3";
    app.execute(
        Addr::unchecked(CREATOR_ADDR),
        WasmMsg::Execute {
            contract_addr: nft_contract1.to_string(),
            msg: to_json_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                recipient: contract_addr.to_string(),
                token_id: token_id.into(),
            })
            .unwrap(),
            funds: vec![],
        }
        .into(),
    )
    .unwrap();

    let owner = query_nft_owner(&app, nft_contract1.clone(), token_id.into()).unwrap();
    assert_eq!(owner.owner, contract_addr);

    sudo_transfer_nft(&mut app, contract_addr, nft_contract1.as_str(), token_id).unwrap();

    let owner = query_nft_owner(&app, nft_contract1.clone(), token_id.into()).unwrap();
    assert_eq!(owner.owner, TEAM_ADDR);
}

#[test]
fn test_update_streak() {
    let (mut app, contract_addr) = setup_base_contract();

    sudo_update_streak_config(
        &mut app,
        contract_addr,
        Some(10),
        Some(100),
        Some(vec![
            StreakReward::new(1, Uint128::new(1000)),
            StreakReward::new(1, Uint128::new(2000)),
        ]),
        Some(vec![TEAM_ADDR.into(), RESERVE_ADDR.into()]),
    )
    .unwrap();
}
