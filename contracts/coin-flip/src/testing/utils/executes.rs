use std::collections::HashSet;

use cosmwasm_std::{coins, to_json_binary, Addr, Empty, Uint128};
use cw_multi_test::{AppResponse, Executor};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, FlipExecuteMsg, StreakExecuteMsg, SudoMsg},
    types::{DenomLimit, Fees, PickTypes, StreakReward},
};

use super::setup::{next_block, BaseApp, CREATOR_ADDR, FLIPPER_ADDR};

pub(crate) fn unwrap_execute(
    res: Result<AppResponse, anyhow::Error>,
) -> Result<AppResponse, ContractError> {
    match res {
        Ok(res) => Ok(res),
        Err(e) => Err(e.downcast().unwrap()),
    }
}

pub fn execute_start_flip(
    app: &mut BaseApp,
    contract_addr: Addr,
    pick: PickTypes,
    flip_amount: Uint128,
    flipper: Addr,
    denom: &str,
    funds: Uint128,
) -> Result<AppResponse, ContractError> {
    let funds = coins(funds.u128(), denom);
    unwrap_execute(app.execute_contract(
        flipper,
        contract_addr,
        &ExecuteMsg::Flip(FlipExecuteMsg::StartFlip {
            pick,
            amount: flip_amount,
        }),
        &funds,
    ))
}

pub fn execute_do_flips(
    app: &mut BaseApp,
    contract_addr: Addr,
) -> Result<AppResponse, ContractError> {
    app.update_block(next_block);
    unwrap_execute(app.execute_contract(
        Addr::unchecked(FLIPPER_ADDR),
        contract_addr,
        &ExecuteMsg::Flip(FlipExecuteMsg::DoFlips {}),
        &[],
    ))
}

pub fn sudo_update_fees(
    app: &mut BaseApp,
    contract_addr: Addr,
    fees: Fees,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateFees { fees }),
        &[],
    ))
}

pub fn sudo_add_new_denom(
    app: &mut BaseApp,
    contract_addr: Addr,
    denom: &str,
    limits: DenomLimit,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::AddNewDenom {
            denom: denom.to_string(),
            limits,
        }),
        &[],
    ))
}

pub fn sudo_remove_denoms(
    app: &mut BaseApp,
    contract_addr: Addr,
    denoms: HashSet<String>,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::RemoveDenoms { denoms }),
        &[],
    ))
}

pub fn sudo_update_bet_limit(
    app: &mut BaseApp,
    contract_addr: Addr,
    denom: &str,
    min_bet: Uint128,
    max_bet: Uint128,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateBetLimit {
            denom: denom.to_string(),
            min_bet,
            max_bet,
        }),
        &[],
    ))
}

pub fn sudo_update_pause(
    app: &mut BaseApp,
    contract_addr: Addr,
    pause: bool,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdatePause(pause)),
        &[],
    ))
}

pub fn sudo_update_sg721(
    app: &mut BaseApp,
    contract_addr: Addr,
    addr: String,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateSg721 { addr }),
        &[],
    ))
}

pub fn sudo_update_bank_limit(
    app: &mut BaseApp,
    contract_addr: Addr,
    denom: &str,
    limit: Uint128,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateBankLimit {
            denom: denom.to_string(),
            limit,
        }),
        &[],
    ))
}

pub fn sudo_update_streak_config(
    app: &mut BaseApp,
    contract_addr: Addr,
    nft_pool_max: Option<u32>,
    streak_nft_winning_amount: Option<u32>,
    streak_rewards: Option<Vec<StreakReward>>,
    allowed_to_send_nft: Option<Vec<String>>,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateStreak {
            nft_pool_max,
            streak_nft_winning_amount,
            streak_rewards,
            allowed_to_send_nft,
        }),
        &[],
    ))
}

pub fn sudo_distribute(
    app: &mut BaseApp,
    contract_addr: Addr,
    denom: &str,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::Distribute {
            denom: denom.to_string(),
        }),
        &[],
    ))
}

pub fn sudo_withdraw_nft_from_pool(
    app: &mut BaseApp,
    contract_addr: Addr,
    index: Option<u32>,
    all: Option<bool>,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::WithdrawNftFromPool { index, all }),
        &[],
    ))
}

pub fn sudo_withdraw_excess(
    app: &mut BaseApp,
    contract_addr: Addr,
    denom: &str,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::SendExcessFunds {
            denom: denom.into(),
        }),
        &[],
    ))
}

pub fn sudo_transfer_nft(
    app: &mut BaseApp,
    contract_addr: Addr,
    nft_contract: &str,
    token_id: &str,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::TransferNft {
            contract: nft_contract.into(),
            token_id: token_id.into(),
        }),
        &[],
    ))
}

pub fn execute_send_nft_to_pool(
    app: &mut BaseApp,
    sender: &str,
    contract_addr: Addr,
    nft_contract: Addr,
    token_id: String,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(sender),
        nft_contract,
        &cw721::Cw721ExecuteMsg::SendNft {
            contract: contract_addr.into(),
            token_id,
            msg: to_json_binary(&Empty {}).unwrap(),
        },
        &[],
    ))
}

pub fn execute_streak_claim(
    app: &mut BaseApp,
    sender: &str,
    contract_addr: Addr,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(sender),
        contract_addr,
        &ExecuteMsg::Streak(StreakExecuteMsg::Claim {}),
        &[],
    ))
}
