use cosmwasm_std::{coin, Addr, Uint128};
use cw_multi_test::Executor;

use crate::{
    error::ContractError,
    testing::utils::{
        executes::{execute_send_nft_to_pool, execute_streak_claim, unwrap_execute},
        helpers::add_balance,
        queries::query_nft_owner,
        setup::{setup_with_nft_pool, CREATOR_ADDR, MIN_BET, NATIVE_DENOM, TEST_STREAK_REWARDS},
    },
    types::PickTypes,
};

use super::utils::{
    executes::{execute_do_flips, execute_start_flip},
    helpers::MIN_FUNDS,
    queries::{query_nft_pool, query_score},
    setup::{setup_base_contract, FLIPPER_ADDR},
};

#[test]
fn test_receive_nft() {
    let (mut app, contract_addr, nft_addr1, nft_addr2) = setup_with_nft_pool();

    let pool = query_nft_pool(&app, contract_addr.clone()).unwrap();

    assert_eq!(pool.len(), 4);
    assert_eq!(pool[0].contract_addr, nft_addr1);
    assert_eq!(pool[0].token_id, 1.to_string());
    assert_eq!(pool[1].contract_addr, nft_addr2);
    assert_eq!(pool[1].token_id, 2.to_string());

    // Try to send another NFT (can't cause reached the limit)
    let err = execute_send_nft_to_pool(
        &mut app,
        CREATOR_ADDR,
        contract_addr.clone(),
        nft_addr2,
        4.to_string(),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::MaxNFTStreakRewardsReached);

    // try send not from team or owner
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        nft_addr1.clone(),
        &cw721::Cw721ExecuteMsg::TransferNft {
            recipient: FLIPPER_ADDR.to_string(),
            token_id: "4".to_string(),
        },
        &[],
    ))
    .unwrap();

    let err = execute_send_nft_to_pool(
        &mut app,
        FLIPPER_ADDR,
        contract_addr,
        nft_addr1,
        4.to_string(),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::UnauthorizedToSendNft);
}

#[test]
fn test_streak_claim() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // We have streak of 3, try to claim (can't because no reward for 3 streaks)
    let err = execute_streak_claim(&mut app, FLIPPER_ADDR, contract_addr.clone()).unwrap_err();
    assert_eq!(err, ContractError::NotEligibleForStreakReward(3));

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // Verify the streak is 4
    let score = query_score(&app, contract_addr.clone(), FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 4);

    // Verify we got paid the 2nd reward
    let res = execute_streak_claim(&mut app, FLIPPER_ADDR, contract_addr.clone()).unwrap();
    assert_eq!(
        res.events[1].attributes[3].value,
        coin(
            TEST_STREAK_REWARDS[1].reward.into(),
            NATIVE_DENOM.to_string()
        )
        .to_string()
    );

    // Verify we resset the score after claiming
    let score = query_score(&app, contract_addr, FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 0);
}

#[test]
fn test_streak_claim_win_streak() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // Verify the streak is 4
    let score = query_score(&app, contract_addr.clone(), FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 4);

    // Verify we got paid the 2nd reward
    let res = execute_streak_claim(&mut app, FLIPPER_ADDR, contract_addr.clone()).unwrap();
    assert_eq!(
        res.events[1].attributes[3].value,
        coin(
            TEST_STREAK_REWARDS[1].reward.into(),
            NATIVE_DENOM.to_string()
        )
        .to_string()
    );

    // Verify we resset the score after claiming
    let score = query_score(&app, contract_addr, FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 0);
}

#[test]
fn test_streak_nft_rewards() {
    let (mut app, contract_addr, _, _) = setup_with_nft_pool();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();

    let res = execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    let nft_info: Vec<&str> = res
        .events
        .iter()
        .find(|event| event.ty == "wasm-streak-claim")
        .unwrap()
        .attributes[3]
        .value
        .split('/')
        .collect();
    let nft_contract = Addr::unchecked(nft_info[0]);
    let token_id = nft_info[1];

    let owner: cw721::OwnerOfResponse =
        query_nft_owner(&app, nft_contract, token_id.to_string()).unwrap();
    assert_eq!(owner.owner, Addr::unchecked(FLIPPER_ADDR));

    // Verify we resset the score after claiming
    let score = query_score(&app, contract_addr, FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 0);
}

#[test]
fn test_streak_nft_rewards_win_streak() {
    let (mut app, contract_addr, _, _) = setup_with_nft_pool();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();

    let res = execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    let nft_info: Vec<&str> = res
        .events
        .iter()
        .find(|event| event.ty == "wasm-streak-claim")
        .unwrap()
        .attributes[3]
        .value
        .split('/')
        .collect();
    let nft_contract = Addr::unchecked(nft_info[0]);
    let token_id = nft_info[1];

    let owner: cw721::OwnerOfResponse =
        query_nft_owner(&app, nft_contract, token_id.to_string()).unwrap();
    assert_eq!(owner.owner, Addr::unchecked(FLIPPER_ADDR));

    // Verify we resset the score after claiming
    let score = query_score(&app, contract_addr, FLIPPER_ADDR).unwrap();
    assert_eq!(score.streak.amount, 0);
}

#[test]
fn test_no_nft_pool() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();

    let old_balance = app
        .wrap()
        .query_balance(FLIPPER_ADDR, NATIVE_DENOM)
        .unwrap();
    let res = execute_do_flips(&mut app, contract_addr).unwrap();

    let rewards = res
        .events
        .iter()
        .find(|event| event.ty == "wasm-streak-claim")
        .unwrap()
        .attributes[3]
        .value
        .clone();
    assert_eq!(
        rewards,
        coin(
            TEST_STREAK_REWARDS[TEST_STREAK_REWARDS.len() - 1]
                .reward
                .u128(),
            NATIVE_DENOM.to_string()
        )
        .to_string()
    );

    //Verify balance actually changed for the flipper
    let new_balance = app
        .wrap()
        .query_balance(FLIPPER_ADDR, NATIVE_DENOM)
        .unwrap();
    assert_eq!(
        new_balance.amount,
        old_balance.amount + TEST_STREAK_REWARDS[TEST_STREAK_REWARDS.len() - 1].reward
    );
}

#[test]
fn test_no_nft_pool_win_streak() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();

    let old_balance = app
        .wrap()
        .query_balance(FLIPPER_ADDR, NATIVE_DENOM)
        .unwrap();
    let res = execute_do_flips(&mut app, contract_addr).unwrap();

    let rewards = res
        .events
        .iter()
        .find(|event| event.ty == "wasm-streak-claim")
        .unwrap()
        .attributes[3]
        .value
        .clone();
    assert_eq!(
        rewards,
        coin(
            TEST_STREAK_REWARDS[TEST_STREAK_REWARDS.len() - 1]
                .reward
                .u128(),
            NATIVE_DENOM.to_string()
        )
        .to_string()
    );

    //Verify balance actually changed for the flipper
    let new_balance = app
        .wrap()
        .query_balance(FLIPPER_ADDR, NATIVE_DENOM)
        .unwrap();
    assert_eq!(
        new_balance.amount,
        old_balance.amount
            + TEST_STREAK_REWARDS[TEST_STREAK_REWARDS.len() - 1].reward
            + MIN_BET * Uint128::new(2) // add win amount
    );
}

#[test]
fn test_no_streak_claim() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        NATIVE_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let err = execute_streak_claim(&mut app, FLIPPER_ADDR, contract_addr).unwrap_err();
    assert_eq!(err, ContractError::LowStreak(TEST_STREAK_REWARDS[0].streak));
}
