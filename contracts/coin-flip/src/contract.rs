use std::collections::{HashMap, HashSet};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ensure_eq, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Uint128};
use cw2::set_contract_version;
use sg_std::Response;

use coin_flip_v07 as ccf_v07;

use crate::error::ContractError;
use crate::helpers::ensure_not_paused;
use crate::msg::{
    ExecuteMsg, FlipExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StreakExecuteMsg,
};
use crate::state::{
    ALLOWED_SEND_NFT, CONFIG, FEES, FLIPS, NFT_REWARDS, STREAK_REWARDS, TODO_FLIPS,
};
use crate::types::{Config, DenomLimit, Fees, Wallets};

use crate::sudo::handle_sudo_msg;

// version info for migration info
const CONTRACT_NAME: &str = "cosmos-coin-flip";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Verify the wallets are correct.
    deps.api.addr_validate(&msg.wallets.team)?;
    deps.api.addr_validate(&msg.wallets.reserve)?;

    let sg721_addr = match msg.sg721_addr {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };

    let mut denom_limits: HashMap<String, DenomLimit> = HashMap::with_capacity(msg.denoms.len());

    for (denom, min, max, bank) in msg.denom_limits {
        denom_limits.insert(denom, DenomLimit { min, max, bank });
    }

    // Save config
    CONFIG.save(
        deps.storage,
        &Config {
            admin: info.sender.to_string(),
            denoms: msg.denoms.clone(),
            denom_limits,
            flips_per_block_limit: msg.flips_per_block_limit.unwrap_or(10),
            wallets: Wallets {
                team: msg.wallets.team,
                reserve: msg.wallets.reserve,
            },
            fees: msg.fees,
            sg721_addr,
            is_paused: false,
            streak_nft_winning_amount: msg.streak_nft_winning_amount,
            nft_pool_max: msg.nft_pool_max,
        },
    )?;

    // Init fees to be 0
    for denom in msg.denoms {
        FEES.save(deps.storage, denom, &Uint128::zero())?;
    }

    FLIPS.save(deps.storage, &vec![])?;
    TODO_FLIPS.save(deps.storage, &vec![])?;

    STREAK_REWARDS.save(deps.storage, &msg.streak_rewards)?;
    let allowed_send_nft_addrs = msg
        .allowed_to_send_nft
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect::<StdResult<Vec<_>>>()?;
    ALLOWED_SEND_NFT.save(deps.storage, &allowed_send_nft_addrs)?;
    NFT_REWARDS.save(deps.storage, &vec![])?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match msg {
        ExecuteMsg::ReceiveNft(cw721::Cw721ReceiveMsg {
            sender,
            token_id,
            msg: _,
        }) => streak_execute::receive_nft(deps, info, &config, sender, token_id),
        ExecuteMsg::Streak(StreakExecuteMsg::Claim {}) => {
            ensure_not_paused(&config)?;
            streak_execute::execute_claim(deps, info)
        }
        ExecuteMsg::Flip(FlipExecuteMsg::StartFlip { pick, amount }) => {
            ensure_not_paused(&config)?;
            flip_execute::execute_start_flip(deps, env, info, &config, pick, amount)
        }
        ExecuteMsg::Flip(FlipExecuteMsg::DoFlips {}) => {
            ensure_not_paused(&config)?;
            flip_execute::execute_do_flips(deps, env, &config)
        }
        ExecuteMsg::Sudo(sudo_msg) => handle_sudo_msg(deps, info, env, config, sudo_msg),
    }
}

mod flip_execute {
    use std::collections::HashMap;

    use cw_utils::must_pay;

    use cosmwasm_std::{coin, ensure, to_json_binary, BankMsg, CosmosMsg, Event, Uint128, WasmMsg};
    use sg_std::StargazeMsgWrapper;
    use sha256::Sha256Digest;

    use crate::helpers::ensure_correct_funds;
    use crate::state::{get_next_flip_id, FEES, FLIPS, FLIP_ID, NFT_REWARDS, SCORES};
    use crate::types::{Flip, FlipScore, PickTypes, TodoFlip};

    use super::*;

    pub(crate) fn execute_start_flip(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        config: &Config,
        pick: PickTypes,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let mut todo_flips = TODO_FLIPS.load(deps.storage)?;

        // Make sure the user doesn't have flip waiting already
        ensure!(
            todo_flips.iter().all(|x| x.wallet != info.sender),
            ContractError::AlreadyStartedFlip
        );

        // Make sure we only have 10 waiting flips max
        ensure!(
            (todo_flips.len() as u64) < config.flips_per_block_limit,
            ContractError::BlockLimitReached
        );

        // Verify we only have one coin sent
        if info.funds.len() != 1 {
            return Err(ContractError::WrongFundsAmount);
        }

        let funds = info.funds[0].clone();
        // Verify the sent funds is in supported denom.
        let denom = if config.denoms.iter().any(|x| x == &funds.denom) {
            funds.denom
        } else {
            return Err(ContractError::WrongDenom { denom: funds.denom });
        };

        // Make sure that the sent amount is within our limits
        let Some(bet_limits) = config.denom_limits.get(&denom) else {
          return Err(ContractError::NoBetLimits { denom});
        };

        ensure!(
            amount <= bet_limits.max,
            ContractError::OverTheLimitBet {
                max_limit: (bet_limits.max / Uint128::new(1000000)).to_string()
            }
        );
        ensure!(
            amount >= bet_limits.min,
            ContractError::UnderTheLimitBet {
                min_limit: (bet_limits.min / Uint128::new(1000000)).to_string()
            }
        );

        // Make sure the paid amount is correct (funds sent is the amount + fee)
        let fee_amount = ensure_correct_funds(funds.amount, amount, config.fees.flip_bps)?;
        let should_pay_amount = amount.checked_add(fee_amount)?;
        let paid_amount = must_pay(&info, &denom)?;

        ensure_eq!(
            should_pay_amount,
            paid_amount,
            ContractError::WrongPaidAmount
        );

        // Make sure we have funds to pay for the flip
        let mut fees = FEES.load(deps.storage, denom.clone())?;
        let balance = deps
            .querier
            .query_balance(&env.contract.address, denom.clone())?;
        ensure!(
            balance.amount - fees >= amount * Uint128::new(2),
            ContractError::ContractMissingFunds(denom)
        );

        // Save fees
        fees = fees.checked_add(fee_amount)?;
        FEES.save(deps.storage, denom.clone(), &fees)?;

        let id = get_next_flip_id(deps.storage);
        FLIP_ID.save(deps.storage, &id)?;

        // Everything is correct, save this to_do_flip
        todo_flips.push(TodoFlip {
            id,
            wallet: info.sender,
            amount: coin(amount.u128(), denom),
            pick,
            block: env.block.height,
            timestamp: env.block.time,
        });
        TODO_FLIPS.save(deps.storage, &todo_flips)?;

        Ok(Response::default()
            .add_event(Event::new("start_flip").add_attribute("id", id.to_string())))
    }

    pub(crate) fn execute_do_flips(
        deps: DepsMut,
        env: Env,
        config: &Config,
    ) -> Result<Response, ContractError> {
        let todo_flips = TODO_FLIPS.load(deps.storage)?;
        // Make sure we have flips
        ensure!(!todo_flips.is_empty(), ContractError::NoFlipsToDo);

        let mut save_todo_flips: Vec<TodoFlip> = vec![];
        let mut flip_denoms: HashMap<String, Uint128> = HashMap::with_capacity(1);

        // Make sure we have flips to do
        let filtered_todo_flips: Vec<TodoFlip> = todo_flips
            .into_iter()
            .filter(|todo_flip| {
                if todo_flip.block >= env.block.height {
                    save_todo_flips.push(todo_flip.clone());
                    return false;
                }

                match flip_denoms.get(todo_flip.amount.denom.as_str()) {
                    Some(amount) => flip_denoms.insert(
                        todo_flip.amount.denom.clone(),
                        *amount + todo_flip.amount.amount,
                    ),
                    None => {
                        flip_denoms.insert(todo_flip.amount.denom.clone(), todo_flip.amount.amount)
                    }
                };

                true
            })
            .collect();

        // Make sure that we have flips to do, else error
        ensure!(
            !filtered_todo_flips.is_empty(),
            ContractError::NoFlipsToDoThisBlock
        );

        // Save the todo flips that are not ready to be flipped yet
        TODO_FLIPS.save(deps.storage, &save_todo_flips)?;

        // Make sure we have funds to pay for all the flips
        for (denom, total_amount) in flip_denoms {
            let fees = FEES.load(deps.storage, denom.clone())?;

            let contract_balance = deps
                .querier
                .query_balance(&env.contract.address, denom.clone())?;

            ensure!(
                contract_balance.amount - fees >= total_amount * Uint128::new(2),
                ContractError::ContractMissingFunds(denom)
            );
        }

        let mut msgs: Vec<CosmosMsg<StargazeMsgWrapper>> = vec![];
        let mut response = Response::default();
        let rand = get_random(&env);
        let mut last_flips = FLIPS.load(deps.storage)?;

        for todo_flip in filtered_todo_flips {
            // Get flip result (won or lost)
            let flip_result = do_a_flip(&todo_flip, rand);

            // Handle score (needed the streak info in Flip)
            let mut score = match SCORES.load(deps.storage, &todo_flip.wallet) {
                Ok(mut score) => score.update(flip_result, env.clone()),
                Err(_) => FlipScore::new(flip_result, env.clone()),
            };

            // Create new flip
            let flip = Flip {
                wallet: todo_flip.wallet.clone(),
                amount: todo_flip.amount.clone(),
                result: flip_result,
                streak: score.streak.clone(),
                timestamp: env.block.time,
            };

            // check if flipper did enough streak to win NFT (12)
            if is_streak_nft_winner(config, &score) {
                let mut nft_pool = NFT_REWARDS.load(deps.storage)?;
                let mut streak_event = Event::new("streak-claim")
                    .add_attribute("flipper", todo_flip.wallet.to_string())
                    .add_attribute("flip_id", todo_flip.id.to_string());

                if nft_pool.is_empty() {
                    let rewards = STREAK_REWARDS.load(deps.storage)?;
                    let to_send = rewards[rewards.len() - 1].clone();

                    msgs.push(
                        BankMsg::Send {
                            to_address: todo_flip.wallet.to_string(),
                            amount: vec![coin(to_send.reward.u128(), "ustars".to_string())],
                        }
                        .into(),
                    );

                    //add claim amount to event
                    streak_event = streak_event
                        .clone()
                        .add_attribute("claim", format!("{}ustars", to_send.reward.u128()));
                } else {
                    // get the random index of the NFT to send
                    let winning_nft_index = rand % nft_pool.len() as u64;
                    let nft_to_send = nft_pool[winning_nft_index as usize].clone();

                    // Send the NFT to the winner
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: nft_to_send.contract_addr.to_string(),
                            msg: to_json_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                                recipient: todo_flip.wallet.to_string().clone(),
                                token_id: nft_to_send.token_id.clone(),
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );

                    nft_pool.remove(winning_nft_index as usize);
                    NFT_REWARDS.save(deps.storage, &nft_pool)?;

                    //add claim amount to event
                    streak_event = streak_event.clone().add_attribute(
                        "claim",
                        format!("{}/{}", nft_to_send.contract_addr, nft_to_send.token_id),
                    );
                }

                // Reset the streak score of the flipper
                score.streak.reset();

                // Add event to response
                response = response.add_event(streak_event);
            }

            // Save the score
            SCORES.save(deps.storage, &todo_flip.wallet, &score)?;

            // Update last flips vector
            if last_flips.len() >= 5 {
                last_flips.remove(0);
            }
            last_flips.push(flip);

            // Send funds if they won
            if flip_result {
                let pay = todo_flip.amount.amount * Uint128::new(2); // double the amount
                msgs.push(
                    BankMsg::Send {
                        to_address: todo_flip.wallet.to_string(),
                        amount: vec![coin(pay.u128(), todo_flip.amount.denom.clone())],
                    }
                    .into(),
                );
            }

            response = response.clone().add_event(
                Event::new("flip")
                    .add_attribute("flipper", todo_flip.wallet)
                    .add_attribute("flip_id", todo_flip.id.to_string())
                    .add_attribute("flip_amount", todo_flip.amount.to_string())
                    .add_attribute("flip_pick", format!("{:?}", todo_flip.pick))
                    .add_attribute("result", if flip_result { "won" } else { "lost" }),
            );
        }

        FLIPS.save(deps.storage, &last_flips)?;

        Ok(response
            .add_attribute("flip_action", "do_flips")
            .add_messages(msgs))
    }

    fn get_random(env: &Env) -> u64 {
        let tx_index = if let Some(tx) = &env.transaction {
            tx.index
        } else {
            0
        };

        let sha256 = Sha256Digest::digest(format!(
            "{}{}{}",
            tx_index,
            env.block.height,
            env.block.time.nanos(),
        ));

        sha256.as_bytes().iter().fold(0, |acc, x| acc + *x as u64)
    }

    fn do_a_flip(todo_flip: &TodoFlip, rand: u64) -> bool {
        let er = todo_flip
            .wallet
            .as_bytes()
            .iter()
            .fold(0, |acc, x| acc + *x as u64);

        let flip_result = (rand + er) % 2 == 0;

        // if picked heads and flip_result is true, he won
        let won_heads = todo_flip.pick == PickTypes::Heads && flip_result;
        // if picked tails and flip_result is false, he won
        let won_tails = todo_flip.pick == PickTypes::Tails && !flip_result;

        // Return true if one of them is true (won) else return false (lost)
        won_heads || won_tails
    }

    fn is_streak_nft_winner(config: &Config, score: &FlipScore) -> bool {
        score.streak.amount == config.streak_nft_winning_amount
    }
}

mod streak_execute {
    use cosmwasm_std::{coins, BankMsg, Event};

    use crate::{
        state::{ALLOWED_SEND_NFT, NFT_REWARDS, SCORES, STREAK_REWARDS},
        types::NftReward,
    };

    use super::*;

    pub(crate) fn receive_nft(
        deps: DepsMut,
        info: MessageInfo,
        config: &Config,
        sender: String,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // Verify the sender can send NFTs to the contract
        let allowed_to_send_nfts = ALLOWED_SEND_NFT.load(deps.storage)?;

        if !allowed_to_send_nfts.contains(&deps.api.addr_validate(&sender)?) {
            return Err(ContractError::UnauthorizedToSendNft);
        }

        // Verify our NFT rewards pool is not full
        let mut nft_rewards = NFT_REWARDS.load(deps.storage)?;

        if nft_rewards.len() as u32 >= config.nft_pool_max {
            return Err(ContractError::MaxNFTStreakRewardsReached);
        }

        // info.sender is the NFT contract address, token_id is the NFT id
        // Add the NFT to the pool of NFTs
        nft_rewards.push(NftReward::new(info.sender.clone(), token_id.clone()));
        NFT_REWARDS.save(deps.storage, &nft_rewards)?;

        Ok(Response::default()
            .add_attribute("action", "add_nft")
            .add_attribute("nft_contract", info.sender)
            .add_attribute("token_id", token_id)
            .add_attribute("nft_pool_size", nft_rewards.len().to_string()))
    }

    pub(crate) fn execute_claim(
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        let streak_rewards = STREAK_REWARDS.load(deps.storage)?;
        let mut score = SCORES.load(deps.storage, &info.sender)?;

        // make sure score is higher or equal to the lowest streak reward
        if score.streak.amount < streak_rewards[0].streak {
            return Err(ContractError::LowStreak(streak_rewards[0].streak));
        }

        // find the matching reward to claim
        match streak_rewards
            .iter()
            .find(|r| r.streak == score.streak.amount)
        {
            Some(reward) => {
                score.streak.reset();
                SCORES.save(deps.storage, &info.sender, &score)?;

                Ok(Response::default()
                    .add_event(
                        Event::new("streak-claim")
                            .add_attribute("flipper", info.sender.to_string())
                            .add_attribute("streak", reward.streak.to_string())
                            .add_attribute("claim", format!("{}ustars", reward.reward)),
                    )
                    .add_message(BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: coins(reward.reward.u128(), "ustars"),
                    }))
            }
            None => Err(ContractError::NotEligibleForStreakReward(
                score.streak.amount,
            )),
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetLast5 {} => query::get_last_5(deps),
        QueryMsg::GetFeesAmount { denom } => query::get_fees(deps, denom),
        QueryMsg::GetAllFeesAmount {} => query::get_all_fees(deps),
        QueryMsg::GetScore { address } => query::get_score(deps, address),
        QueryMsg::GetConfig {} => query::get_config(deps),
        QueryMsg::ShouldDoFlips {} => query::should_do_flips(deps, env),
        QueryMsg::DryDistribution { denom } => query::dry_distribution(deps, env, denom),
        QueryMsg::GetNftPool {} => query::get_nft_pool(deps),
    }
}

mod query {
    use cosmwasm_std::{
        to_json_binary, Binary, Coin, Decimal, Deps, Env, StdError, StdResult, Uint128,
    };

    use crate::{
        msg::DryDistributionResponse,
        state::{CONFIG, FEES, FLIPS, NFT_REWARDS, SCORES, TODO_FLIPS},
        sudo::{calculate_fees_to_pay, get_holders_list, verify_contract_balance},
        types::{FeesToPay, NftReward},
    };

    pub fn get_fees(deps: Deps, denom: String) -> StdResult<Binary> {
        to_json_binary(&FEES.load(deps.storage, denom)?)
    }

    pub fn get_all_fees(deps: Deps) -> StdResult<Binary> {
        let all_fees = FEES
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|res| {
                let (denom, amount) = res?;

                Ok(Coin { denom, amount })
            })
            .collect::<Result<Vec<Coin>, StdError>>()?;

        to_json_binary(&all_fees)
    }

    pub fn get_config(deps: Deps) -> StdResult<Binary> {
        to_json_binary(&CONFIG.load(deps.storage)?)
    }

    pub fn get_score(deps: Deps, address: String) -> StdResult<Binary> {
        let address = deps.api.addr_validate(&address)?;
        to_json_binary(&SCORES.load(deps.storage, &address)?)
    }

    pub fn should_do_flips(deps: Deps, env: Env) -> StdResult<Binary> {
        let todo_flips = TODO_FLIPS.load(deps.storage)?;

        let res = todo_flips
            .iter()
            .any(|todo_flip| env.block.height > todo_flip.block);
        to_json_binary(&res)
    }

    pub fn get_last_5(deps: Deps) -> StdResult<Binary> {
        let flips = FLIPS.load(deps.storage)?;

        to_json_binary(&flips)
    }

    pub fn dry_distribution(deps: Deps, env: Env, denom: String) -> StdResult<Binary> {
        let config = CONFIG.load(deps.storage)?;
        let total_fees = FEES.load(deps.storage, denom.clone()).unwrap_or_default();
        let bank_limit = config
            .denom_limits
            .get(&denom)
            .expect("denom limit should exist in the map")
            .bank;

        let (
            sg721_addr,
            FeesToPay {
                team: team_fees_to_send,
                holders: holders_fees_to_send,
                reserve: reserve_fees,
            },
        ) = calculate_fees_to_pay(&config, total_fees)
            .map_err(|x| StdError::generic_err(x.to_string()))?;

        let reserve_fees_to_send =
            verify_contract_balance(deps, env, denom, total_fees, reserve_fees, bank_limit)
                .map_err(|x| StdError::generic_err(x.to_string()))?;

        let mut paid_to_holders = Uint128::zero();
        let mut total_shares = Decimal::zero();
        let mut fees_per_token = Decimal::zero();
        let mut number_of_holders: u64 = 0;

        if !holders_fees_to_send.is_zero() {
            let (calculated_total_shares, holders_list) = get_holders_list(deps, sg721_addr)
                .map_err(|x| StdError::generic_err(x.to_string()))?;
            total_shares = calculated_total_shares;
            number_of_holders = holders_list.len() as u64;

            fees_per_token = Decimal::from_atomics(holders_fees_to_send, 0)
                .map_err(|x| StdError::generic_err(x.to_string()))?
                .checked_div(total_shares)
                .map_err(|x| StdError::generic_err(x.to_string()))?;

            for (_, num) in holders_list {
                let amount = fees_per_token.checked_mul(num)?.to_uint_floor();

                if !amount.is_zero() {
                    paid_to_holders = paid_to_holders.checked_add(amount)?;
                }
            }
        }

        to_json_binary(&DryDistributionResponse {
            total_fees,
            team_total_fee: team_fees_to_send,
            reserve_total_fee: reserve_fees_to_send,
            holders_total_fee: holders_fees_to_send,
            holders_total_shares: total_shares,
            fees_per_token,
            pay_to_holders: paid_to_holders,
            number_of_holders,
        })
    }

    pub fn get_nft_pool(deps: Deps) -> StdResult<Binary> {
        let nft_pool: Vec<NftReward> = NFT_REWARDS.load(deps.storage)?;

        to_json_binary(&nft_pool)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg {
        MigrateMsg::Basic {} => {}
        MigrateMsg::FromV07 {
            nft_pool_max,
            streak_nft_winning_amount,
            streak_rewards,
            allowed_to_send_nft,
        } => {
            set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

            if allowed_to_send_nft.is_empty() {
                return Err(ContractError::EmptyAllowedToSendNft);
            }

            if streak_rewards.len() < 3 {
                return Err(ContractError::LowStreakAmount);
            }

            if streak_rewards[streak_rewards.len() - 1].streak != streak_nft_winning_amount {
                return Err(ContractError::NftWinNotMatchLastStreakReward);
            }

            // convert old config to new config
            let old_config = ccf_v07::state::CONFIG.load(deps.storage)?;
            let old_denom = old_config.denoms[0].clone();

            let mut denom_limits: HashMap<String, DenomLimit> = HashMap::new();
            denom_limits.insert(
                old_denom.clone(),
                DenomLimit {
                    min: old_config.min_bet_limit,
                    max: old_config.max_bet_limit,
                    bank: old_config.bank_limit,
                },
            );

            CONFIG.save(
                deps.storage,
                &Config {
                    admin: old_config.admin,
                    denoms: HashSet::from_iter(old_config.denoms),
                    denom_limits,
                    flips_per_block_limit: old_config.flips_per_block_limit,
                    wallets: Wallets {
                        team: old_config.wallets.team,
                        reserve: old_config.wallets.reserve,
                    },
                    fees: Fees {
                        team_bps: old_config.fees.team_bps,
                        holders_bps: old_config.fees.holders_bps,
                        reserve_bps: old_config.fees.reserve_bps,
                        flip_bps: old_config.fees.flip_bps,
                    },
                    sg721_addr: old_config.sg721_addr,
                    is_paused: old_config.is_paused,
                    // New fields
                    nft_pool_max,
                    streak_nft_winning_amount,
                },
            )?;

            STREAK_REWARDS.save(deps.storage, &streak_rewards)?;
            let allowed_send_nft_addrs = allowed_to_send_nft
                .into_iter()
                .map(|addr| deps.api.addr_validate(&addr))
                .collect::<StdResult<Vec<_>>>()?;
            ALLOWED_SEND_NFT.save(deps.storage, &allowed_send_nft_addrs)?;
            NFT_REWARDS.save(deps.storage, &vec![])?;

            // Update fee storage
            let old_fees = ccf_v07::state::FEES.load(deps.storage)?;
            ccf_v07::state::FEES.remove(deps.storage);
            FEES.save(deps.storage, old_denom, &old_fees)?;
        }
    };

    Ok(Response::default())
}
