use std::collections::{HashMap, HashSet};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Env, Timestamp, Uint128};

use crate::helpers::bps_to_decimal;

#[cw_serde]
pub struct Config {
    pub admin: String,
    pub denoms: HashSet<String>,
    pub denom_limits: HashMap<String, DenomLimit>,
    pub flips_per_block_limit: u64,
    pub wallets: Wallets,
    pub fees: Fees,
    pub sg721_addr: Option<Addr>,
    pub is_paused: bool,

    pub nft_pool_max: u32,
    pub streak_nft_winning_amount: u32,
}

#[cw_serde]
pub struct DenomLimit {
    pub min: Uint128,
    pub max: Uint128,
    pub bank: Uint128,
}

#[cw_serde]
pub struct StreakReward {
    pub streak: u32,
    pub reward: Uint128,
}

impl StreakReward {
    pub const fn new(streak: u32, reward: Uint128) -> StreakReward {
        StreakReward { streak, reward }
    }
}

#[cw_serde]
pub struct NftReward {
    pub token_id: String,
    pub contract_addr: Addr,
}

impl NftReward {
    pub fn new(contract_addr: Addr, token_id: String) -> NftReward {
        NftReward {
            token_id,
            contract_addr,
        }
    }
}

#[cw_serde]
pub enum PickTypes {
    Heads,
    Tails,
}

#[cw_serde]
pub struct Wallets {
    pub team: String,
    pub reserve: String,
}

#[cw_serde]
pub struct Fees {
    pub team_bps: u64,
    pub holders_bps: u64,
    pub reserve_bps: u64,
    pub flip_bps: u64,
}

impl Fees {
    pub fn calculate(&self, total_fees: Uint128) -> FeesToPay {
        let total_fees = Decimal::from_atomics(total_fees, 0).unwrap();
        let team_perc = bps_to_decimal(self.team_bps);
        let holders_perc = bps_to_decimal(self.holders_bps);
        let reserve_perc = bps_to_decimal(self.reserve_bps);

        let team_decimal_to_pay = total_fees.checked_mul(team_perc).unwrap();
        let holders_decimal_to_pay = total_fees.checked_mul(holders_perc).unwrap();
        let reserve_decimal_to_pay = total_fees.checked_mul(reserve_perc).unwrap();

        FeesToPay {
            team: self.to_uint_floor(team_decimal_to_pay),
            holders: self.to_uint_floor(holders_decimal_to_pay),
            reserve: self.to_uint_floor(reserve_decimal_to_pay),
        }
    }

    pub fn to_uint_floor(&self, to_pay: Decimal) -> Uint128 {
        let decimal_fractional = Uint128::from(
            10_u128
                .checked_pow(to_pay.decimal_places())
                .unwrap_or(1_000_000_000_000_000_000u128),
        );
        let full_num = to_pay.floor().atomics();
        full_num.checked_div(decimal_fractional).unwrap()
    }
}

#[cw_serde]
pub struct FeesToPay {
    pub team: Uint128,
    pub holders: Uint128,
    pub reserve: Uint128,
}

#[cw_serde]
pub struct Flip {
    pub wallet: Addr,
    pub amount: Coin,
    pub result: bool,
    pub streak: Streak,
    pub timestamp: Timestamp,
}

#[cw_serde]
pub struct FlipScore {
    pub streak: Streak,
    pub last_flip: Timestamp,
}

impl FlipScore {
    pub fn new(result: bool, env: Env) -> Self {
        FlipScore {
            streak: Streak::new(result),
            last_flip: env.block.time,
        }
    }

    pub fn update(&mut self, result: bool, env: Env) -> Self {
        self.streak.update(result);
        self.last_flip = env.block.time;
        self.clone()
    }
}

#[cw_serde]
pub struct Streak {
    pub amount: u32,
    pub result: bool,
}

impl Streak {
    pub fn new(result: bool) -> Self {
        Streak { amount: 1, result }
    }
    pub fn update(&mut self, result: bool) {
        if result == self.result {
            self.amount += 1;
        } else {
            self.amount = 1;
            self.result = result;
        }
    }

    pub fn reset(&mut self) {
        self.amount = 0;
    }
}

#[cw_serde]
pub struct TodoFlip {
    pub id: u64,
    pub wallet: Addr,
    pub amount: Coin,
    pub pick: PickTypes,
    pub block: u64,
    pub timestamp: Timestamp,
}
