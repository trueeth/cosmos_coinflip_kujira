use cosmwasm_std::{Addr, Coin, StdError, Uint128};
use cw721::OwnerOfResponse;

use crate::{
    msg::{DryDistributionResponse, QueryMsg},
    types::{Config, Flip, FlipScore, NftReward},
};

use super::setup::BaseApp;

pub fn query_config(app: &BaseApp, contract_addr: Addr) -> Result<Config, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetConfig {})
}

pub fn query_fees(app: &BaseApp, contract_addr: Addr, denom: &str) -> Result<Uint128, StdError> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::GetFeesAmount {
            denom: denom.to_string(),
        },
    )
}

pub fn query_all_fees(app: &BaseApp, contract_addr: Addr) -> Result<Vec<Coin>, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetAllFeesAmount {})
}

pub fn query_last_flips(app: &BaseApp, contract_addr: Addr) -> Result<Vec<Flip>, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetLast5 {})
}

pub fn query_score(
    app: &BaseApp,
    contract_addr: Addr,
    address: &str,
) -> Result<FlipScore, StdError> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::GetScore {
            address: address.to_string(),
        },
    )
}

pub fn query_dry_distribution(
    app: &BaseApp,
    contract_addr: Addr,
    denom: &str,
) -> Result<DryDistributionResponse, StdError> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::DryDistribution {
            denom: denom.to_string(),
        },
    )
}

pub fn query_nft_pool(app: &BaseApp, contract_addr: Addr) -> Result<Vec<NftReward>, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetNftPool {})
}

pub fn query_should_flip(app: &BaseApp, contract_addr: Addr) -> Result<bool, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::ShouldDoFlips {})
}

pub fn query_nft_owner(
    app: &BaseApp,
    contract_addr: Addr,
    token_id: String,
) -> Result<OwnerOfResponse, StdError> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &cw721::Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        },
    )
}
