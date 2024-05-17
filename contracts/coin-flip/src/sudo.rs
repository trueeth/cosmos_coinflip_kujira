use std::collections::{HashMap, HashSet};

use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Uint128, WasmMsg,
};
use sg_std::{Response, StargazeMsgWrapper};

use crate::error::ContractError;
use crate::helpers::ensure_admin;
use crate::msg::SudoMsg;
use crate::state::{ALLOWED_SEND_NFT, CONFIG, FEES, NFT_REWARDS, STREAK_REWARDS};
use crate::types::{Config, DenomLimit, Fees, FeesToPay, StreakReward};

pub fn handle_sudo_msg(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    config: Config,
    msg: SudoMsg,
) -> Result<Response, ContractError> {
    ensure_admin(&config, &info)?;

    match msg {
        SudoMsg::Distribute { denom } => distribute(deps, env, denom, &config),
        SudoMsg::AddNewDenom { denom, limits } => add_new_denom(deps, config, denom, limits),
        SudoMsg::RemoveDenoms { denoms } => remove_denoms(deps, config, denoms),
        SudoMsg::UpdateFees { fees } => update_fees(deps, config, fees),
        SudoMsg::UpdateBankLimit { denom, limit } => update_bank_limit(deps, config, denom, limit),
        SudoMsg::UpdateSg721 { addr } => update_sg721(deps, config, addr),
        SudoMsg::UpdatePause(is_paused) => update_pause(deps, config, is_paused),
        SudoMsg::UpdateStreak {
            nft_pool_max,
            streak_nft_winning_amount,
            streak_rewards,
            allowed_to_send_nft,
        } => update_streak_config(
            deps,
            config,
            nft_pool_max,
            streak_nft_winning_amount,
            streak_rewards,
            allowed_to_send_nft,
        ),
        SudoMsg::UpdateBetLimit {
            denom,
            min_bet,
            max_bet,
        } => update_bet_limit(deps, config, denom, min_bet, max_bet),
        SudoMsg::WithdrawNftFromPool { index, all } => {
            withdraw_nft_from_pool(deps, &config, index, all)
        }
        SudoMsg::SendExcessFunds { denom } => send_excess_funds(deps, env, &config, denom),
        SudoMsg::TransferNft { contract, token_id } => {
            transfer_nft(deps, &config, contract, token_id)
        }
    }
}

/// Update the bank limit in the config
pub fn update_bank_limit(
    deps: DepsMut,
    mut config: Config,
    denom: String,
    limit: Uint128,
) -> Result<Response, ContractError> {
    let mut bank_limit = config
        .denom_limits
        .get(&denom)
        .expect("denom limit should exist in the map")
        .clone();

    bank_limit.bank = limit;
    config.denom_limits.insert(denom, bank_limit);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_bank_limit"))
}

pub fn update_fees(
    deps: DepsMut,
    mut config: Config,
    fees: Fees,
) -> Result<Response, ContractError> {
    config.fees = fees;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_fees"))
}

pub fn update_sg721(
    deps: DepsMut,
    mut config: Config,
    addr: String,
) -> Result<Response, ContractError> {
    config.sg721_addr = Some(deps.api.addr_validate(&addr)?);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_sg721"))
}

pub fn update_bet_limit(
    deps: DepsMut,
    mut config: Config,
    denom: String,
    min: Uint128,
    max: Uint128,
) -> Result<Response, ContractError> {
    let mut denom_limit = config
        .denom_limits
        .get(&denom)
        .expect("denom limit should exist in the map")
        .clone();

    denom_limit.min = min;
    denom_limit.max = max;
    config.denom_limits.insert(denom, denom_limit);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_bet_limit"))
}

pub fn update_pause(
    deps: DepsMut,
    mut config: Config,
    is_paused: bool,
) -> Result<Response, ContractError> {
    config.is_paused = is_paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_pause"))
}

pub fn update_streak_config(
    deps: DepsMut,
    mut config: Config,
    nft_pool_max: Option<u32>,
    streak_nft_winning_amount: Option<u32>,
    streak_rewards: Option<Vec<StreakReward>>,
    allowed_to_send_nft: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    if let Some(nft_pool_max) = nft_pool_max {
        config.nft_pool_max = nft_pool_max;
    }

    if let Some(streak_nft_winning_amount) = streak_nft_winning_amount {
        config.streak_nft_winning_amount = streak_nft_winning_amount;
    }

    CONFIG.save(deps.storage, &config)?;

    if let Some(streak_rewards) = streak_rewards {
        STREAK_REWARDS.save(deps.storage, &streak_rewards)?;
    }

    if let Some(allowed_to_send_nft) = allowed_to_send_nft {
        let addrs = allowed_to_send_nft
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<Result<Vec<Addr>, _>>()?;
        ALLOWED_SEND_NFT.save(deps.storage, &addrs)?;
    }

    Ok(Response::default().add_attribute("method", "update_streak_config"))
}

pub fn add_new_denom(
    deps: DepsMut,
    mut config: Config,
    denom: String,
    limits: DenomLimit,
) -> Result<Response, ContractError> {
    FEES.update(deps.storage, denom.clone(), |denom| match denom {
        Some(_) => Err(ContractError::DenomAlreadyExists),
        None => Ok(Uint128::zero()),
    })?;

    config.denom_limits.insert(denom.clone(), limits);
    config.denoms.insert(denom);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "add_new_denom"))
}

pub fn remove_denoms(
    deps: DepsMut,
    mut config: Config,
    denoms: HashSet<String>,
) -> Result<Response, ContractError> {
    for denom in denoms {
        if !config.denoms.contains(&denom) {
            return Err(ContractError::DenomNotFound { denom });
        }

        let fees = FEES.load(deps.storage, denom.clone())?;
        if !fees.is_zero() {
            return Err(ContractError::DenomStillHaveFees(denom));
        }

        config.denom_limits.remove(&denom);
        config.denoms.remove(&denom);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "remove_denoms"))
}

pub fn distribute(
    deps: DepsMut,
    env: Env,
    denom: String,
    config: &Config,
) -> Result<Response, ContractError> {
    let total_fees = FEES.load(deps.storage, denom.clone()).unwrap_or_default();
    let bank_limit = config
        .denom_limits
        .get(&denom)
        .expect("denom limit should exist in the map")
        .bank;

    // We do distribution per a single denom
    let (
        sg721_addr,
        FeesToPay {
            team: team_fees_to_send,
            holders: holders_fees_to_send,
            reserve: reserve_fees,
        },
    ) = calculate_fees_to_pay(config, total_fees)?;

    let reserve_fees_to_send = verify_contract_balance(
        deps.as_ref(),
        env,
        denom.clone(),
        total_fees,
        reserve_fees,
        bank_limit,
    )?;

    // Handle holders fees
    let mut msgs: Vec<BankMsg> = vec![];
    let mut paid_to_holders = Uint128::zero();
    let mut total_shares = Decimal::zero();
    let mut fees_per_token = Decimal::zero();

    if !holders_fees_to_send.is_zero() {
        let (calculated_total_shares, holders_list) = get_holders_list(deps.as_ref(), sg721_addr)?;
        total_shares = calculated_total_shares;

        fees_per_token =
            Decimal::from_atomics(holders_fees_to_send, 0)?.checked_div(total_shares)?;

        for (addr, num) in holders_list {
            let amount = fees_per_token.checked_mul(num)?.to_uint_floor();

            if !amount.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: addr,
                    amount: coins(amount.into(), denom.clone()),
                });
            }

            paid_to_holders = paid_to_holders.checked_add(amount)?;
        }
    }

    // create subMsg send to team wallet
    msgs.push(BankMsg::Send {
        to_address: config.wallets.team.clone(),
        amount: coins(team_fees_to_send.into(), denom.clone()),
    });

    // Send to reserve
    if !reserve_fees_to_send.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: config.wallets.reserve.clone(),
            amount: coins(reserve_fees_to_send.into(), denom.clone()),
        });
    }

    // calculate remaining fees and save them to state
    let remaining_fees = total_fees
        .checked_sub(paid_to_holders)?
        .checked_sub(team_fees_to_send)?
        .checked_sub(reserve_fees)?;
    FEES.save(deps.storage, denom, &remaining_fees)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("total_fees", total_fees)
        .add_attribute("reserve_paid", reserve_fees)
        .add_attribute("team_paid", team_fees_to_send)
        .add_attribute("holders_paid", paid_to_holders)
        .add_attribute("fees_per_token", fees_per_token.to_string())
        .add_attribute("total_shares", total_shares.to_string()))
}

pub fn withdraw_nft_from_pool(
    deps: DepsMut,
    config: &Config,
    index: Option<u32>,
    all: Option<bool>,
) -> Result<Response, ContractError> {
    let nft_attr: String;
    let mut msgs: Vec<CosmosMsg<StargazeMsgWrapper>> = vec![];

    if all.is_some() {
        // loop over all NFTs in pool, and send them to team wallet.
        let nft_rewards = NFT_REWARDS.load(deps.storage)?;
        let nft_len = nft_rewards.len();

        nft_rewards.into_iter().for_each(|nft| {
            msgs.push(
                WasmMsg::Execute {
                    contract_addr: nft.contract_addr.to_string(),
                    msg: to_json_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                        recipient: config.wallets.team.clone(),
                        token_id: nft.token_id,
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            );
        });

        NFT_REWARDS.save(deps.storage, &vec![])?;

        nft_attr = format!("All NFTs withdrawn from pool ({nft_len})");
    } else if let Some(index) = index {
        let mut nft_rewards = NFT_REWARDS.load(deps.storage)?;

        if index >= nft_rewards.len() as u32 {
            return Err(ContractError::NftIndexOutOfRange);
        }
        // Get NFT by index from our pool and send it to the team wallet.
        let nft_to_send = nft_rewards[index as usize].clone();

        msgs.push(
            WasmMsg::Execute {
                contract_addr: nft_to_send.contract_addr.to_string(),
                msg: to_json_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                    recipient: config.wallets.team.clone(),
                    token_id: nft_to_send.token_id.clone(),
                })?,
                funds: vec![],
            }
            .into(),
        );

        nft_rewards.remove(index as usize);
        NFT_REWARDS.save(deps.storage, &nft_rewards)?;

        nft_attr = format!(
            "withdraw: {} / {}",
            nft_to_send.contract_addr, nft_to_send.token_id
        );
    } else {
        // Return error because none of the options are provided.
        return Err(ContractError::EmptyWithdrawParams);
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_nft_from_pool")
        .add_attribute("withdraw", nft_attr))
}

pub fn send_excess_funds(
    deps: DepsMut,
    env: Env,
    config: &Config,
    denom: String,
) -> Result<Response, ContractError> {
    let Coin {
        amount: bank_amount,
        denom,
    } = deps.querier.query_balance(env.contract.address, denom)?;

    let fees_amount = FEES.load(deps.storage, denom.clone())?;
    let bank_balance = bank_amount - fees_amount;
    let bank_limit = config
        .denom_limits
        .get(&denom)
        .expect("denom limit should exist in the map")
        .bank;

    if bank_balance > bank_limit {
        let to_send = bank_balance - bank_limit;

        let msg = BankMsg::Send {
            to_address: config.wallets.reserve.clone(),
            amount: coins(to_send.into(), denom.clone()),
        };
        Ok(Response::default()
            .add_message(msg)
            .add_attribute("action", "send_excess_funds")
            .add_attribute("amount", format!("{to_send}{denom}")))
    } else {
        Err(ContractError::NoExcessFunds)
    }
}

pub fn transfer_nft(
    deps: DepsMut,
    config: &Config,
    contract: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let nft_pool = NFT_REWARDS.load(deps.storage)?;

    if nft_pool
        .iter()
        .any(|nft| nft.contract_addr == contract && nft.token_id == token_id)
    {
        return Err(ContractError::NftInPool);
    }

    let msg = WasmMsg::Execute {
        contract_addr: contract,
        msg: to_json_binary(&cw721::Cw721ExecuteMsg::TransferNft {
            recipient: config.wallets.team.clone(),
            token_id,
        })?,
        funds: vec![],
    };
    Ok(Response::default().add_message(msg))
}

pub fn calculate_fees_to_pay(
    config: &Config,
    total_fees: Uint128,
) -> Result<(Addr, FeesToPay), ContractError> {
    // If fees are lower then the minimum bet amount, means we don't fees to pay (no flips happened)
    if total_fees.u128() <= 1000_u128 {
        return Err(ContractError::NoFeesToPay {});
    }

    // If we have sg721_addr, it means we have a collection we need to distribute to
    // the holders. If not, we distribute to the team and reserve 50/50.
    if let Some(sg721_addr) = config.sg721_addr.clone() {
        Ok((sg721_addr, config.fees.calculate(total_fees)))
    } else {
        let half = total_fees.checked_div(Uint128::new(2))?;
        Ok((
            Addr::unchecked("sg721"),
            FeesToPay {
                team: half,
                holders: Uint128::zero(),
                reserve: half,
            },
        ))
    }
}

pub fn verify_contract_balance(
    deps: Deps,
    env: Env,
    denom: String,
    total_fees: Uint128,
    reserve_fees: Uint128,
    bank_limit: Uint128,
) -> Result<Uint128, ContractError> {
    let mut reserve_fees_to_send = reserve_fees;
    let contract_balance = deps.querier.query_balance(env.contract.address, denom)?;
    let bank_balance = contract_balance
        .amount
        .checked_sub(total_fees)
        .map_err(|_| ContractError::NotEnoughFundsToPayFees)?;

    if bank_balance < bank_limit {
        // How much we need to reach to the minimum bank amount.
        let reserve_diff = bank_limit.checked_sub(bank_balance)?;

        if reserve_diff > reserve_fees_to_send {
            // If we need more then we have, we send nothing.
            reserve_fees_to_send = Uint128::zero();
        } else {
            // If we need less then we have, we send the difference.
            reserve_fees_to_send.checked_sub(reserve_diff)?;
        }
    }
    Ok(reserve_fees_to_send)
}

pub fn get_holders_list(
    deps: Deps,
    sg721_addr: Addr,
) -> Result<(Decimal, HashMap<String, Decimal>), ContractError> {
    let mut total_shares = Decimal::zero();
    let mut holders_list: HashMap<String, Decimal> = HashMap::new();

    for num in 1..=777 {
        // Get the owner of the token.
        let owner_addr = deps.querier.query_wasm_smart::<cw721::OwnerOfResponse>(
            sg721_addr.clone(),
            &cw721::Cw721QueryMsg::OwnerOf {
                token_id: num.to_string(),
                include_expired: None,
            },
        );

        if let Ok(res) = owner_addr {
            let rewards_share = get_share(num)?;
            total_shares = total_shares.checked_add(rewards_share)?;

            holders_list
                .entry(res.owner)
                .and_modify(|e| *e += rewards_share)
                .or_insert(rewards_share);
        }
    }
    Ok((total_shares, holders_list))
}

pub fn get_share(num: u32) -> Result<Decimal, ContractError> {
    if (650..=727).contains(&num) {
        // 1.5
        Decimal::one()
            .checked_add(Decimal::from_atomics(Uint128::from(5_u128), 1)?)
            .map_err(ContractError::OverflowErr)
    } else if num >= 728 {
        Decimal::from_atomics(Uint128::from(2_u128), 0).map_err(ContractError::DecimalRangeExceeded)
    // 2
    } else {
        Ok(Decimal::one()) // 1
    }
}
