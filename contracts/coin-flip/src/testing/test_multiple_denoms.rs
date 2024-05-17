use std::collections::HashSet;

use cosmwasm_std::Addr;

use crate::{
    error::ContractError,
    testing::utils::{
        executes::{sudo_add_new_denom, sudo_distribute, sudo_remove_denoms},
        setup::{MIN_BANK_AMOUNT, NATIVE_DENOM},
    },
    types::{DenomLimit, PickTypes},
};

use super::utils::{
    executes::{execute_do_flips, execute_start_flip},
    helpers::MIN_FUNDS,
    queries::{query_all_fees, query_config, query_fees},
    setup::{setup_base_contract, setup_with_multiple_denoms, FLIPPER_ADDR, MIN_BET, USDC_DENOM},
};

#[test]
fn test_update_denoms() {
    let (mut app, contract_addr) = setup_base_contract();

    let mut to_remove = HashSet::new();
    to_remove.insert(USDC_DENOM.to_string());

    sudo_add_new_denom(
        &mut app,
        contract_addr.clone(),
        USDC_DENOM,
        DenomLimit {
            min: MIN_BET,
            max: MIN_BET,
            bank: MIN_BANK_AMOUNT,
        },
    )
    .unwrap();

    let config = query_config(&app, contract_addr.clone()).unwrap();
    assert_eq!(config.denoms.len(), 2);
    assert!(config.denoms.contains(NATIVE_DENOM));
    assert!(config.denoms.contains(USDC_DENOM));

    // remove usdc
    sudo_remove_denoms(&mut app, contract_addr.clone(), to_remove).unwrap();

    let config = query_config(&app, contract_addr).unwrap();
    assert_eq!(config.denoms.len(), 1);
    assert!(config.denoms.contains(NATIVE_DENOM));
}

#[test]
fn test_happy_path() {
    let (mut app, contract_addr) = setup_with_multiple_denoms();

    // Make sure fees are zero before we start
    let native_fees = query_fees(&app, contract_addr.clone(), NATIVE_DENOM).unwrap();
    let usdc_fees = query_fees(&app, contract_addr.clone(), USDC_DENOM).unwrap();

    assert!(native_fees.is_zero());
    assert!(usdc_fees.is_zero());

    // Do some flips with each token
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
        USDC_DENOM,
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // Make sure the fees are not zero now
    let native_fees = query_fees(&app, contract_addr.clone(), NATIVE_DENOM).unwrap();
    let usdc_fees = query_fees(&app, contract_addr.clone(), USDC_DENOM).unwrap();
    let all_fees = query_all_fees(&app, contract_addr.clone()).unwrap();

    assert!(!native_fees.is_zero());
    assert!(!usdc_fees.is_zero());
    assert!(all_fees.len() == 2);

    sudo_distribute(&mut app, contract_addr.clone(), NATIVE_DENOM).unwrap();

    // We only distributed native, usdc should not be empty
    let native_fees = query_fees(&app, contract_addr.clone(), NATIVE_DENOM).unwrap();
    let usdc_fees = query_fees(&app, contract_addr.clone(), USDC_DENOM).unwrap();

    assert!(native_fees.is_zero());
    assert!(!usdc_fees.is_zero());

    sudo_distribute(&mut app, contract_addr.clone(), USDC_DENOM).unwrap();

    // We distributed all fees, so they both should be zero now.
    let native_fees = query_fees(&app, contract_addr.clone(), NATIVE_DENOM).unwrap();
    let usdc_fees = query_fees(&app, contract_addr, USDC_DENOM).unwrap();

    assert!(native_fees.is_zero());
    assert!(usdc_fees.is_zero());
}

#[test]
fn test_non_existing_denom() {
    let (mut app, contract_addr) = setup_with_multiple_denoms();

    execute_start_flip(
        &mut app,
        contract_addr,
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        "random",
        MIN_FUNDS,
    )
    .unwrap_err();
}

#[test]
fn test_add_existing_denom() {
    let (mut app, contract_addr) = setup_with_multiple_denoms();

    let err = sudo_add_new_denom(
        &mut app,
        contract_addr,
        USDC_DENOM,
        DenomLimit {
            min: MIN_BET,
            max: MIN_BET,
            bank: MIN_BANK_AMOUNT,
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::DenomAlreadyExists)
}

#[test]
fn test_remove_non_existing_denom() {
    let (mut app, contract_addr) = setup_with_multiple_denoms();

    let mut to_remove = HashSet::new();
    to_remove.insert("random".to_string());

    let err = sudo_remove_denoms(&mut app, contract_addr, to_remove).unwrap_err();
    assert_eq!(
        err,
        ContractError::DenomNotFound {
            denom: "random".to_string()
        }
    )
}

#[test]
fn test_remove_denom_with_fees() {
    let (mut app, contract_addr) = setup_with_multiple_denoms();

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

    let mut to_remove = HashSet::new();
    to_remove.insert(NATIVE_DENOM.to_string());

    let err = sudo_remove_denoms(&mut app, contract_addr, to_remove).unwrap_err();
    assert_eq!(
        err,
        ContractError::DenomStillHaveFees(NATIVE_DENOM.to_string())
    )
}
