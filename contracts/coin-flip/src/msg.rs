use std::collections::HashSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};

use crate::types::{
    Config, DenomLimit, Fees, Flip, FlipScore, NftReward, PickTypes, StreakReward, Wallets,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub denoms: HashSet<String>,
    pub wallets: Wallets,
    pub fees: Fees,
    /// Limit of each denom, (denom, min_bet, max_bet, bank_limit)
    pub denom_limits: Vec<(String, Uint128, Uint128, Uint128)>,
    pub flips_per_block_limit: Option<u64>,
    pub sg721_addr: Option<String>,

    // streak
    pub nft_pool_max: u32,
    pub streak_nft_winning_amount: u32,
    pub streak_rewards: Vec<StreakReward>,
    pub allowed_to_send_nft: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive NFTs to add to our NFTs pool of streak mini game
    ReceiveNft(cw721::Cw721ReceiveMsg),
    /// Streak mini game msgs
    Streak(StreakExecuteMsg),
    /// Flip msgs
    Flip(FlipExecuteMsg),
    /// Only call-able by admin (mutlisig)
    Sudo(SudoMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(Config)]
    GetConfig {},
    /// Get fees total
    #[returns(Uint128)]
    GetFeesAmount { denom: String },
    #[returns(Vec<Coin>)]
    GetAllFeesAmount {},
    /// Get last 10 flips
    #[returns(Vec<Flip>)]
    GetLast5 {},
    /// Get score of wallet
    #[returns(FlipScore)]
    GetScore { address: String },
    /// let us know if we should execute the do flips msg or not
    /// this is to prevent sending unnecessary txs
    #[returns(bool)]
    ShouldDoFlips {},
    /// Do dry ditribution to see results
    #[returns(DryDistributionResponse)]
    DryDistribution { denom: String },
    /// Get the NFT pool
    #[returns(Vec<NftReward>)]
    GetNftPool {},
}

#[cw_serde]
pub enum StreakExecuteMsg {
    /// Claim streak reward if the sender streak matches one of the rewards
    /// e.g - streak reward is at 8 streaks, if sender have 8 streaks, he can claim the reward
    /// if sender have 9 streaks, he can't claim rewards for 8 streak,
    ///
    /// Claim only highest reward, so if sender have 10 streak,
    /// and streak rewards are at 8 and 10, only 10th will be claimed,
    /// and streak will be reset.
    ///
    /// NFT claims will be automatically claimed on 12th streak (highest reward)
    /// but if NFT pool is empty, last reward will be automatically sent
    Claim {},
}

#[cw_serde]
pub enum FlipExecuteMsg {
    /// Register the flip
    StartFlip { pick: PickTypes, amount: Uint128 },
    /// Does the actual flip
    DoFlips {},
}

#[cw_serde]
pub enum SudoMsg {
    /// Distribute the collected fees so far
    Distribute {
        denom: String,
    },
    /// Add new denom to allow flipping with
    AddNewDenom {
        denom: String,
        limits: DenomLimit,
    },
    /// Remove denoms
    RemoveDenoms {
        denoms: HashSet<String>,
    },
    /// Update fees
    UpdateFees {
        fees: Fees,
    },
    // Update the collection address
    UpdateSg721 {
        addr: String,
    },
    /// Update the bank limit
    UpdateBankLimit {
        denom: String,
        limit: Uint128,
    },
    /// Update the bet limit (min and max)
    UpdateBetLimit {
        denom: String,
        min_bet: Uint128,
        max_bet: Uint128,
    },
    /// Pause the contract in case of emergency
    UpdatePause(bool),
    /// Update streak related config stuff
    UpdateStreak {
        nft_pool_max: Option<u32>,
        streak_nft_winning_amount: Option<u32>,
        streak_rewards: Option<Vec<StreakReward>>,
        allowed_to_send_nft: Option<Vec<String>>,
    },
    /// Withdraw all or a single NFT from the pool
    /// only to the team wallet
    WithdrawNftFromPool {
        index: Option<u32>,
        all: Option<bool>,
    },
    /// Send excess funds to the reserve wallet
    /// This function checks the bank limit - fees and send the excess to the reserve wallet
    SendExcessFunds {
        denom: String,
    },
    /// This transfer the NFT to the team wallet
    /// This function will check that the NFT is not in the pool
    /// In case NFT will be transfered by mistake we will be able to handle it
    TransferNft {
        contract: String,
        token_id: String,
    },
}

#[cw_serde]
pub enum MigrateMsg {
    Basic {},
    FromV07 {
        nft_pool_max: u32,
        streak_nft_winning_amount: u32,
        streak_rewards: Vec<StreakReward>,
        allowed_to_send_nft: Vec<String>,
    },
}

#[cw_serde]
pub struct DryDistributionResponse {
    pub total_fees: Uint128,
    pub team_total_fee: Uint128,
    pub reserve_total_fee: Uint128,
    pub holders_total_fee: Uint128,
    pub holders_total_shares: Decimal,
    pub fees_per_token: Decimal,
    pub pay_to_holders: Uint128,
    pub number_of_holders: u64,
}
