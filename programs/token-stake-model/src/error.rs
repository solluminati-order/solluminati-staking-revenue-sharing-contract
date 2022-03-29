use anchor_lang::{prelude::*};

#[error_code]
pub enum StakeError {
    #[msg("Invalid Merkle proof.")]
    InvalidProof,
    #[msg("You must have some SOL")]
    NoEnoughSol,
    #[msg("This is not available from epoch 2")]
    StartEpoch2,
    #[msg("Epoch days are wrong")]
    EpochWrongDays,
    #[msg("The 52 epoches are ended")]
    EpochEnd,
    #[msg("The staking is already initialized")]
    EpochAlreadyStarted,
    #[msg("Claim amount is wrong")]
    ClaimAmountBig,
    #[msg("The days of Epoch is over")]
    WrongEpochDay,
    #[msg("Calculate reward one time every day")]
    NoDaily,
    #[msg("The owner of nft is wrong")]
    WrongOwner,
    #[msg("There is no stakers")]
    NoStaker
}