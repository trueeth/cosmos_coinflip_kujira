use std::convert::Infallible;

use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),

    #[error("{0}")]
    Infallible(#[from] Infallible),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Fees to be paid is 0")]
    NoFeesToPay,

    #[error("Fees amount to distribute is more then the contract balance")]
    NotEnoughFundsToPayFees,

    #[error("The sent funds holds wrong amount.")]
    WrongPaidAmount,

    #[error("We do not support this denom = {denom}")]
    WrongDenom { denom: String },

    #[error("We only support 1 denom at a time.")]
    WrongFundsAmount,

    #[error("You already started a flip, please wait for it to finish.")]
    AlreadyStartedFlip,

    #[error("Block limit reached, please try again in few seconds")]
    BlockLimitReached,

    #[error("NFT contract is not set.")]
    Sg721NotSet,

    #[error("Contract doesn't have enough funds to pay for the bet: denom = {0}")]
    ContractMissingFunds(String),

    #[error("You cannot bet above our limit = {max_limit}")]
    OverTheLimitBet { max_limit: String },

    #[error("You cannot bet under our limit = {min_limit}")]
    UnderTheLimitBet { min_limit: String },

    #[error("Operation is paused at this moment! Please try again later.")]
    Paused,

    #[error("There are no flips to do")]
    NoFlipsToDo,

    #[error("There are no flips to do this block")]
    NoFlipsToDoThisBlock,

    // Streak errors
    #[error("This address is not allowed to send NFTs to the contract")]
    UnauthorizedToSendNft,

    #[error("NFTs rewards pool is full")]
    MaxNFTStreakRewardsReached,

    #[error("Expecting an 'index' or 'all' parameter")]
    EmptyWithdrawParams,

    #[error("Index does not exists in the NFT rewards pool")]
    NftIndexOutOfRange,

    #[error("Minimum streak is {0}")]
    LowStreak(u32),

    #[error("Streak ({0}) not eligible for reward")]
    NotEligibleForStreakReward(u32),

    #[error("At least 1 address must be provided to allowed_send_nft")]
    EmptyAllowedToSendNft,

    #[error("Can't transfer NFT, it's in the pool")]
    NftInPool,

    #[error("No excess funds to send")]
    NoExcessFunds,

    #[error("Streak rewards must be higher then 3")]
    LowStreakAmount,

    #[error("NFT winning streak amount must match the last streak reward streak amount")]
    NftWinNotMatchLastStreakReward,

    #[error("Bet Limits doesn't exists for this denom: {denom}")]
    NoBetLimits { denom: String },

    #[error("Denom is not in the list of denoms: {denom}")]
    DenomNotFound { denom: String },

    #[error("Denom Is already exists on the contract")]
    DenomAlreadyExists,

    #[error("Denom still have fees that are not distributed, denom : {0}")]
    DenomStillHaveFees(String),
}
