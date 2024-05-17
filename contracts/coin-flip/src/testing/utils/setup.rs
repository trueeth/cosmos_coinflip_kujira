use std::collections::HashSet;

use cosmwasm_std::{
    coin, coins, testing::MockApi, Addr, BlockInfo, Empty, MemoryStorage, Timestamp, Uint128,
};
use cw_multi_test::{
    App, BankKeeper, BasicAppBuilder, Contract, ContractWrapper, Executor, FailingModule,
    WasmKeeper,
};

use sg721::{CollectionInfo, RoyaltyInfoResponse};
use sg_std::StargazeMsgWrapper;

use crate::{
    msg::InstantiateMsg,
    types::{DenomLimit, Fees, StreakReward, Wallets},
};

use super::{
    executes::{execute_send_nft_to_pool, sudo_add_new_denom, sudo_update_sg721, unwrap_execute},
    helpers::{add_balance, mint_nfts},
};

pub type BaseApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<StargazeMsgWrapper, Empty, Empty>,
    WasmKeeper<StargazeMsgWrapper, Empty>,
>;

pub const FLIPPER_ADDR: &str = "some_flipper";
pub const FLIPPER_ADDR2: &str = "some_flipper2";
pub const CREATOR_ADDR: &str = "creator";
pub const NATIVE_DENOM: &str = "ustars";
pub const USDC_DENOM: &str = "uusdc";
pub const TEST_STREAK_REWARDS: [StreakReward; 3] = [
    StreakReward::new(2, Uint128::new(100000)),
    StreakReward::new(4, Uint128::new(200000)),
    StreakReward::new(5, Uint128::new(300000)),
];

/// Min bet people are allow to bet
pub const MIN_BET: Uint128 = Uint128::new(5_000_000);
/// Max bet people are allow to bet
pub const MAX_BET: Uint128 = Uint128::new(25_000_000);
/// Minimum amount of tokens we need to have in the contract
pub const MIN_BANK_AMOUNT: Uint128 = Uint128::new(30_000_000_000);

//Wallets
pub const TEAM_ADDR: &str = "team_wallet";
pub const RESERVE_ADDR: &str = "reserve_wallet";

pub const PLUS_NANOS: u64 = 654321;

pub fn nft_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

pub fn flip_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_nanos(PLUS_NANOS);
    block.height += 1;
}

/// Basic setup for unit test on a single contract
pub fn setup_base_contract() -> (BaseApp, Addr) {
    let mut app: BaseApp = BasicAppBuilder::<StargazeMsgWrapper, Empty>::new_custom()
        .with_block(BlockInfo {
            height: 1,
            time: Timestamp::from_seconds(123456789),
            chain_id: "stargaze-1".to_string(),
        })
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(FLIPPER_ADDR),
                    vec![
                        coin(999999999999999, NATIVE_DENOM),
                        coin(999999999999999, USDC_DENOM),
                        coin(999999999999999, "random"),
                    ],
                )
                .unwrap();

            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(FLIPPER_ADDR2),
                    coins(999999999999999, NATIVE_DENOM),
                )
                .unwrap();
        });

    let code_id = app.store_code(flip_contract());

    let denoms = vec![NATIVE_DENOM.to_string()];
    let init_msg = &InstantiateMsg {
        denoms: HashSet::from_iter(denoms),
        wallets: Wallets {
            team: TEAM_ADDR.to_string(),
            reserve: RESERVE_ADDR.to_string(),
        },
        fees: Fees {
            team_bps: 1500,
            holders_bps: 7000,
            reserve_bps: 1500,
            flip_bps: 350,
        },
        denom_limits: vec![(NATIVE_DENOM.to_string(), MIN_BET, MAX_BET, MIN_BANK_AMOUNT)],
        flips_per_block_limit: None,
        sg721_addr: None,
        nft_pool_max: 4,
        streak_nft_winning_amount: 5,
        streak_rewards: TEST_STREAK_REWARDS.into(),
        allowed_to_send_nft: vec![TEAM_ADDR.to_string(), CREATOR_ADDR.to_string()],
    };

    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            init_msg,
            &[],
            "flip contract",
            Some(CREATOR_ADDR.to_string()),
        )
        .unwrap();

    add_balance(&mut app, contract_addr.clone(), 30000000000);

    (app, contract_addr)
}

pub fn setup_contract() -> (BaseApp, Addr) {
    let (mut app, contract_addr) = setup_base_contract();
    let nft_code_id = app.store_code(nft_contract());

    let nft_addr = app
        .instantiate_contract(
            nft_code_id,
            contract_addr.clone(),
            &sg721::InstantiateMsg {
                name: "Test NFT".to_string(),
                symbol: "TEST".to_string(),
                minter: contract_addr.to_string(),
                collection_info: CollectionInfo::<RoyaltyInfoResponse> {
                    description: "Test NFT".to_string(),
                    image: "https://example.net".to_string(),
                    creator: CREATOR_ADDR.to_string(),
                    external_link: None,
                    explicit_content: Some(false),
                    start_trading_time: None,
                    royalty_info: None,
                },
            },
            &[],
            "flip contract",
            None,
        )
        .unwrap();

    mint_nfts(&mut app, nft_addr.clone(), contract_addr.clone(), 777);

    sudo_update_sg721(&mut app, contract_addr.clone(), nft_addr.to_string()).unwrap();

    (app, contract_addr)
}

pub fn setup_nft_contracts(app: &mut BaseApp, contract_addr: Addr) -> (Addr, Addr) {
    // Create two NFT contracts
    let nft_code_id = app.store_code(nft_contract());
    let nft_addr1 = app
        .instantiate_contract(
            nft_code_id,
            contract_addr.clone(),
            &sg721::InstantiateMsg {
                name: "Test NFT".to_string(),
                symbol: "TEST".to_string(),
                minter: Addr::unchecked(CREATOR_ADDR).into(),
                collection_info: CollectionInfo::<RoyaltyInfoResponse> {
                    description: "Test NFT".to_string(),
                    image: "https://example.net".to_string(),
                    creator: CREATOR_ADDR.to_string(),
                    external_link: None,
                    explicit_content: Some(false),
                    start_trading_time: None,
                    royalty_info: None,
                },
            },
            &[],
            "nft1 contract",
            None,
        )
        .unwrap();

    let nft_addr2 = app
        .instantiate_contract(
            nft_code_id,
            contract_addr,
            &sg721::InstantiateMsg {
                name: "Test NFT2".to_string(),
                symbol: "TEST2".to_string(),
                minter: Addr::unchecked(CREATOR_ADDR).into(),
                collection_info: CollectionInfo::<RoyaltyInfoResponse> {
                    description: "Test NFT2".to_string(),
                    image: "https://example.net".to_string(),
                    creator: CREATOR_ADDR.to_string(),
                    external_link: None,
                    explicit_content: Some(false),
                    start_trading_time: None,
                    royalty_info: None,
                },
            },
            &[],
            "nft2 contract",
            None,
        )
        .unwrap();

    // mint 5 nfts for each collection
    for i in 1..=5 {
        unwrap_execute(app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            nft_addr1.clone(),
            &sg721::ExecuteMsg::Mint::<Empty, Empty>(cw721_base::MintMsg {
                token_id: i.to_string(),
                owner: CREATOR_ADDR.to_string(),
                token_uri: Some("ipfs://sdfsdf.com".to_string()),
                extension: Empty {},
            }),
            &[],
        ))
        .unwrap();

        unwrap_execute(app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            nft_addr2.clone(),
            &sg721::ExecuteMsg::Mint::<Empty, Empty>(cw721_base::MintMsg {
                token_id: i.to_string(),
                owner: CREATOR_ADDR.to_string(),
                token_uri: Some("ipfs://sdfsdf.com".to_string()),
                extension: Empty {},
            }),
            &[],
        ))
        .unwrap();
    }

    (nft_addr1, nft_addr2)
}

pub fn setup_with_nft_pool() -> (BaseApp, Addr, Addr, Addr) {
    let (mut app, contract_addr) = setup_base_contract();
    let (nft_addr1, nft_addr2) = setup_nft_contracts(&mut app, contract_addr.clone());

    execute_send_nft_to_pool(
        &mut app,
        CREATOR_ADDR,
        contract_addr.clone(),
        nft_addr1.clone(),
        1.to_string(),
    )
    .unwrap();
    execute_send_nft_to_pool(
        &mut app,
        CREATOR_ADDR,
        contract_addr.clone(),
        nft_addr2.clone(),
        2.to_string(),
    )
    .unwrap();
    execute_send_nft_to_pool(
        &mut app,
        CREATOR_ADDR,
        contract_addr.clone(),
        nft_addr2.clone(),
        1.to_string(),
    )
    .unwrap();
    execute_send_nft_to_pool(
        &mut app,
        CREATOR_ADDR,
        contract_addr.clone(),
        nft_addr1.clone(),
        5.to_string(),
    )
    .unwrap();

    (app, contract_addr, nft_addr1, nft_addr2)
}

pub fn setup_with_multiple_denoms() -> (BaseApp, Addr) {
    let (mut app, contract_addr) = setup_base_contract();

    sudo_add_new_denom(
        &mut app,
        contract_addr.clone(),
        USDC_DENOM,
        DenomLimit {
            min: MIN_BET,
            max: MAX_BET,
            bank: MIN_BANK_AMOUNT,
        },
    )
    .unwrap();

    (app, contract_addr)
}
