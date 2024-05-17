use std::collections::{HashMap, HashSet};

use coin_flip_v07 as ccf07;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Uint128,
};
use sg_std::NATIVE_DENOM;

use crate::{
    contract::migrate,
    error::ContractError,
    state::{ALLOWED_SEND_NFT, CONFIG, FEES, STREAK_REWARDS},
    testing::utils::setup::{MAX_BET, MIN_BANK_AMOUNT, MIN_BET, USDC_DENOM},
    types::{Config, DenomLimit, Fees, StreakReward, Wallets},
};

use super::utils::setup::{CREATOR_ADDR, RESERVE_ADDR, TEAM_ADDR};

#[test]
fn test_07_to_08() {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR_ADDR, &[]);

    ccf07::contract::instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        ccf07::msg::InstantiateMsg {
            admin: CREATOR_ADDR.into(),
            denoms: vec![NATIVE_DENOM.into()],
            wallets: ccf07::types::Wallets {
                team: TEAM_ADDR.into(),
                reserve: RESERVE_ADDR.into(),
            },
            fees: ccf07::types::Fees {
                team_bps: 1500,
                holders_bps: 7000,
                reserve_bps: 1500,
                flip_bps: 350,
            },
            bank_limit: None,
            min_bet_limit: None,
            max_bet_limit: None,
            flips_per_block_limit: None,
            sg721_addr: None,
        },
    )
    .unwrap();

    let old_config = ccf07::state::CONFIG.load(deps.as_ref().storage).unwrap();

    migrate(
        deps.as_mut(),
        mock_env(),
        crate::msg::MigrateMsg::FromV07 {
            nft_pool_max: 4,
            streak_nft_winning_amount: 5,
            streak_rewards: vec![
                StreakReward::new(2, Uint128::new(200000)),
                StreakReward::new(4, Uint128::new(400000)),
                StreakReward::new(5, Uint128::new(500000)),
            ],
            allowed_to_send_nft: vec![TEAM_ADDR.into(), CREATOR_ADDR.into()],
        },
    )
    .unwrap();

    let new_config = CONFIG.load(deps.as_ref().storage).unwrap();
    let mut new_denom_limits: HashMap<String, DenomLimit> = HashMap::new();

    new_denom_limits.insert(
        NATIVE_DENOM.to_string(),
        DenomLimit {
            min: MIN_BET,
            max: MAX_BET,
            bank: MIN_BANK_AMOUNT,
        },
    );

    assert_eq!(
        new_config,
        Config {
            admin: old_config.admin,
            denoms: HashSet::from_iter(old_config.denoms),
            denom_limits: new_denom_limits,
            flips_per_block_limit: old_config.flips_per_block_limit,
            wallets: Wallets {
                team: old_config.wallets.team,
                reserve: old_config.wallets.reserve
            },
            fees: Fees {
                team_bps: old_config.fees.team_bps,
                holders_bps: old_config.fees.holders_bps,
                reserve_bps: old_config.fees.reserve_bps,
                flip_bps: old_config.fees.flip_bps
            },
            sg721_addr: old_config.sg721_addr,
            is_paused: old_config.is_paused,
            nft_pool_max: 4,
            streak_nft_winning_amount: 5,
        }
    );

    let streak_rewards = STREAK_REWARDS.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        streak_rewards,
        vec![
            StreakReward::new(2, Uint128::new(200000)),
            StreakReward::new(4, Uint128::new(400000)),
            StreakReward::new(5, Uint128::new(500000)),
        ]
    );

    let allowed_to_send_nft = ALLOWED_SEND_NFT.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        allowed_to_send_nft,
        vec![Addr::unchecked(TEAM_ADDR), Addr::unchecked(CREATOR_ADDR)]
    );

    let fees = FEES
        .load(deps.as_ref().storage, NATIVE_DENOM.to_string())
        .unwrap();
    assert!(fees.is_zero());

    // Should error because usdc fees doesn't exists
    FEES.load(deps.as_ref().storage, USDC_DENOM.to_string())
        .unwrap_err();
}

#[test]
fn test_07_to_08_failing() {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR_ADDR, &[]);

    ccf07::contract::instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        ccf07::msg::InstantiateMsg {
            admin: CREATOR_ADDR.into(),
            denoms: vec![NATIVE_DENOM.into()],
            wallets: ccf07::types::Wallets {
                team: TEAM_ADDR.into(),
                reserve: RESERVE_ADDR.into(),
            },
            fees: ccf07::types::Fees {
                team_bps: 1500,
                holders_bps: 7000,
                reserve_bps: 1500,
                flip_bps: 350,
            },
            bank_limit: None,
            min_bet_limit: None,
            max_bet_limit: None,
            flips_per_block_limit: None,
            sg721_addr: None,
        },
    )
    .unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        crate::msg::MigrateMsg::FromV07 {
            nft_pool_max: 4,
            streak_nft_winning_amount: 5,
            streak_rewards: vec![
                StreakReward::new(2, Uint128::new(200000)),
                StreakReward::new(4, Uint128::new(400000)),
                StreakReward::new(5, Uint128::new(500000)),
            ],
            allowed_to_send_nft: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::EmptyAllowedToSendNft);

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        crate::msg::MigrateMsg::FromV07 {
            nft_pool_max: 4,
            streak_nft_winning_amount: 5,
            streak_rewards: vec![
                StreakReward::new(2, Uint128::new(200000)),
                StreakReward::new(4, Uint128::new(400000)),
            ],
            allowed_to_send_nft: vec![TEAM_ADDR.into(), CREATOR_ADDR.into()],
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::LowStreakAmount);

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        crate::msg::MigrateMsg::FromV07 {
            nft_pool_max: 4,
            streak_nft_winning_amount: 6,
            streak_rewards: vec![
                StreakReward::new(2, Uint128::new(200000)),
                StreakReward::new(4, Uint128::new(400000)),
                StreakReward::new(5, Uint128::new(500000)),
            ],
            allowed_to_send_nft: vec![TEAM_ADDR.into(), CREATOR_ADDR.into()],
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NftWinNotMatchLastStreakReward);
}
