use anchor_lang::{prelude::*, solana_program::clock};
use anchor_spl::token::{self, Mint, TokenAccount, Transfer, Token, CloseAccount};
use anchor_lang::solana_program::{program::invoke, program::invoke_signed, system_instruction };
use std::mem::size_of;
pub mod error;
use crate::{error::StakeError};
use std::io::{Cursor, Write};
use std::ops::DerefMut;
use anchor_lang::__private::CLOSED_ACCOUNT_DISCRIMINATOR;

pub mod merkle_proof;

//insert here the program id after anchor deploy

declare_id!("generated_program_id after deploy");

const RATE_BANK_TO_TREASURY: u8 = 100; // 100%
const DAYS_7_IN_SECONDS: u32 = 604800; // 7 days in seconds
const EPOCH_DAYS: u8 = 7; // 1 epoch = 7 days
const TOTAL_EPOCH: u8 = 52; // 1year = 52 epoch
#[program]
pub mod token_stake_model {
    use super::*;

    pub fn process_initialize(
        ctx: Context<Initialize>,
        amount: u64
    ) -> Result<()> {

        if ctx.accounts.epoch_state.epoch_no >= 1 {
            return Err(error!(StakeError::EpochAlreadyStarted));
        }

        if ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }
        if ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }

        if **ctx.accounts.bank_account.lamports.borrow() < amount {
            return Err(error!(StakeError::NoEnoughSol));
        }

        // transfer SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.bank_account.key,
                ctx.accounts.treasury_account.key,
                amount,
            ),
            &[
                ctx.accounts.bank_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        let clock = clock::Clock::get().unwrap();

        ctx.accounts.epoch_state.is_initial = true;
        ctx.accounts.epoch_state.epoch_no = 1;
        ctx.accounts.epoch_state.epoch_start_time = clock.unix_timestamp;
        ctx.accounts.epoch_state.cur_epoch_reward_per_day = amount.checked_div(EPOCH_DAYS as u64).unwrap();
        ctx.accounts.epoch_state.epoch_bonus = 0;

        ctx.accounts.stake_info.is_initial = true;
        ctx.accounts.stake_info.day_of_epoch = 0;
        Ok(())
    }

    pub fn process_send_epoch_bonus(
        ctx: Context<EpochBonus>,
        amount: u64
    ) -> Result<()> {
        if **ctx.accounts.bonus_account.lamports.borrow() < amount {
            return Err(error!(StakeError::NoEnoughSol));
        }
        if !ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }

        if ctx.accounts.epoch_state.epoch_no > TOTAL_EPOCH {
            return Err(error!(StakeError::EpochEnd));
        }

        // transfer SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.bonus_account.key,
                ctx.accounts.treasury_account.key,
                amount,
            ),
            &[
                ctx.accounts.bonus_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        ctx.accounts.epoch_state.epoch_bonus = ctx.accounts.epoch_state.epoch_bonus.checked_add(amount).unwrap();

        Ok(())
    }

    pub fn process_start_epoch(
        ctx: Context<StartEpoch>,
    ) -> Result<()> {

        if **ctx.accounts.bank_account.lamports.borrow() <= 0 {
            return Err(error!(StakeError::NoEnoughSol));
        }
        if !ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if ctx.accounts.epoch_state.epoch_no == 1 {
            return Err(error!(StakeError::EpochWrongDays));
        }
        if ctx.accounts.epoch_state.epoch_no > TOTAL_EPOCH {
            return Err(error!(StakeError::EpochEnd));
        }
        if ctx.accounts.stake_info.day_of_epoch != 0 {
            return Err(error!(StakeError::EpochEnd));
        }
        let clock = clock::Clock::get().unwrap();

        if clock.unix_timestamp > ctx.accounts.epoch_state.epoch_start_time.checked_add(DAYS_7_IN_SECONDS as i64).unwrap() {
            return Err(error!(StakeError::EpochWrongDays));
        }

        
        ctx.accounts.epoch_state.epoch_no += 1;
        ctx.accounts.epoch_state.epoch_start_time = ctx.accounts.epoch_state.epoch_start_time.checked_add(DAYS_7_IN_SECONDS as i64).unwrap();
        let bank_amount = **ctx.accounts.bank_account.lamports.borrow() ;
        let send_amount = bank_amount.checked_add(ctx.accounts.epoch_state.epoch_bonus).unwrap().checked_add(ctx.accounts.epoch_state.remain_reward).unwrap().checked_mul(RATE_BANK_TO_TREASURY as u64).unwrap().checked_div(100).unwrap();
        
        ctx.accounts.epoch_state.cur_epoch_reward_per_day = send_amount.checked_div(EPOCH_DAYS as u64).unwrap();
        ctx.accounts.epoch_state.epoch_bonus = 0;
        ctx.accounts.epoch_state.remain_reward = 0;

        // transfer SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.bank_account.key,
                ctx.accounts.treasury_account.key,
                send_amount,
            ),
            &[
                ctx.accounts.bank_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        Ok(())
    }
    
    pub fn process_initialize_merkle(
        ctx: Context<InitializeMerkle>,
        root: [u8; 32],
    ) -> Result<()> {
        if ctx.accounts.merkle.is_init {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }
        let merkle = &mut ctx.accounts.merkle;
        merkle.root = root;
        merkle.is_init = true;
        merkle.admin_account = ctx.accounts.admin_account.key();
        Ok(())
    }

    pub fn update_merkle(
        ctx: Context<UpdateMerkle>,
        root: [u8; 32],
    ) -> Result<()> {
        if !ctx.accounts.merkle.is_init {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let merkle = &mut ctx.accounts.merkle;
        merkle.root = root;
        Ok(())
    }

    pub fn process_initialize_user (
        ctx: Context<StakeUser>,
    ) -> Result<()> {
        if ctx.accounts.stake_user.is_initial {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }
        ctx.accounts.stake_user.user_account = ctx.accounts.user_account.key();
        ctx.accounts.stake_user.is_initial = true;
        Ok(())
    }

    pub fn process_stake_nft(
        ctx: Context<StakeNft>,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {

        if !ctx.accounts.merkle.is_init {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        
        let clock = clock::Clock::get().unwrap();

        let merkle_seed = b"nft-staking-merkle-tree";

        let node = anchor_lang::solana_program::keccak::hashv(&[
            &merkle_seed.as_ref(),
            &ctx.accounts.nft_mint.to_account_info().key().to_bytes(),
        ]);

        let merkle = &ctx.accounts.merkle;
        
        if !merkle_proof::verify(proof, merkle.root, node.0) {
            return Err(error!(StakeError::InvalidProof));
        }

        // transfer the nft to vault account
        token::transfer(
            ctx.accounts.into_transfer_to_pda_context(),
            1,
        )?;

        ctx.accounts.stake_info_account.user_account = ctx.accounts.user_account.key();
        ctx.accounts.stake_info_account.nft_mint = ctx.accounts.nft_mint.key();
        ctx.accounts.stake_info_account.stake_time = clock.unix_timestamp;

        ctx.accounts.stake_info.total_stakers += 1;
        
        Ok(())
    }

    pub fn process_unstake_nft(
        ctx: Context<UnStakeNft>,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {

        if !ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.merkle.is_init {
            return Err(ProgramError::UninitializedAccount.into());
        }

        let merkle_seed = b"nft-staking-merkle-tree";

        let node = anchor_lang::solana_program::keccak::hashv(&[
            &merkle_seed.as_ref(),
            &ctx.accounts.nft_mint.to_account_info().key().to_bytes(),
        ]);

        let merkle = &ctx.accounts.merkle;
        
        if !merkle_proof::verify(proof, merkle.root, node.0) {
            return Err(error!(StakeError::InvalidProof));
        } 

        // transfer the nft to vault account
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[b"vault-stake-auth"], ctx.program_id);

        let authority_seeds = &[&b"vault-stake-auth"[..], &[vault_authority_bump]];

        token::transfer(
            ctx.accounts.into_transfer_to_user_context().with_signer(&[&authority_seeds[..]]),
            1,
        )?;

        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.nft_vault_account.to_account_info(),
                destination: ctx.accounts.user_account.to_account_info(),
                authority: ctx.accounts.vault_auth.to_account_info(),
            },
            &[&authority_seeds[..]],
        ));

        ctx.accounts.epoch_state.remain_reward = ctx.accounts.epoch_state.remain_reward.checked_add(ctx.accounts.stake_user.pending_amount).unwrap();

        ctx.accounts.stake_user.pending_amount = 0;

        ctx.accounts.stake_info.total_stakers -= 1;

        Ok(())
    }

    pub fn process_claim_reward(
        ctx: Context<ClaimReward>,
        treasury_nonce: u8,
    ) -> Result<()> {
        let claim_amount = ctx.accounts.stake_user.reward_amount;

        if claim_amount > 0 {
            invoke_signed(
                &system_instruction::transfer(
                    ctx.accounts.treasury_account.key,
                    ctx.accounts.user_account.key,
                    claim_amount,
                ),
                &[
                    ctx.accounts.treasury_account.clone(),
                    ctx.accounts.user_account.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
                &[&[b"epoch-treasury", &[treasury_nonce]]],
            )?;

            ctx.accounts.stake_user.reward_amount = ctx.accounts.stake_user.reward_amount.checked_sub(claim_amount).unwrap();

        }
                
        Ok(())
    }

    pub fn process_daily_reward(
        ctx: Context<DailyReward>,
    ) -> Result<()> {
        if ctx.accounts.stake_info.day_of_epoch > EPOCH_DAYS {
            return Err(error!(StakeError::WrongEpochDay));
        }
        if !ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if ctx.accounts.epoch_state.epoch_no > TOTAL_EPOCH {
            return Err(error!(StakeError::EpochEnd));
        }

        if ctx.accounts.stake_info.total_stakers > 0 {
            
            let reward_per_nft = ctx.accounts.epoch_state.cur_epoch_reward_per_day.checked_div(ctx.accounts.stake_info.total_stakers as u64).unwrap();

            if ctx.accounts.stake_nft_info.user_account != ctx.accounts.stake_user_account.user_account {
                return Err(error!(StakeError::WrongOwner));
            }

            if ctx.accounts.stake_info.day_of_epoch == 0 {
                ctx.accounts.stake_user_account.reward_amount = ctx.accounts.stake_user_account.reward_amount.checked_add(ctx.accounts.stake_user_account.pending_amount).unwrap();
                ctx.accounts.stake_user_account.pending_amount = 0;
            } else {
                ctx.accounts.stake_user_account.pending_amount = ctx.accounts.stake_user_account.pending_amount.checked_add(reward_per_nft).unwrap();
            }

        } else {
            // return Err(StakeError::NoStaker.into());
        }
        
        
        Ok(())
    }

    pub fn process_update_day_of_epoch(
        ctx: Context<UpdateDayEpoch>,
    ) -> Result<()> {
        if ctx.accounts.stake_info.day_of_epoch > EPOCH_DAYS {
            return Err(error!(StakeError::WrongEpochDay));
        }
        if !ctx.accounts.epoch_state.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !ctx.accounts.stake_info.is_initial {
            return Err(ProgramError::UninitializedAccount.into());
        }
        if ctx.accounts.epoch_state.epoch_no > TOTAL_EPOCH {
            return Err(error!(StakeError::EpochEnd));
        }

        let clock = clock::Clock::get().unwrap();
        let time_range = (clock.unix_timestamp.checked_sub(ctx.accounts.epoch_state.epoch_start_time).unwrap()) % 86400;
        if time_range > 60 {
            return Err(error!(StakeError::NoDaily));
        }

        if ctx.accounts.stake_info.day_of_epoch == EPOCH_DAYS - 1 {
            ctx.accounts.stake_info.day_of_epoch = 0;
        } else {
            ctx.accounts.stake_info.day_of_epoch += 1;
        }

        Ok(())
    }

    pub fn process_restart_epoch(
        ctx: Context<RestartEpoch>,
    ) -> Result<()> {
        let amount = **ctx.accounts.treasury_account.lamports.borrow();
        let clock = clock::Clock::get().unwrap();

        ctx.accounts.epoch_state.is_initial = true;
        ctx.accounts.epoch_state.epoch_no = 1;
        ctx.accounts.epoch_state.epoch_start_time = clock.unix_timestamp;
        ctx.accounts.epoch_state.cur_epoch_reward_per_day = amount.checked_div(EPOCH_DAYS as u64).unwrap();
        ctx.accounts.epoch_state.epoch_bonus = 0;

        ctx.accounts.stake_info.is_initial = true;
        ctx.accounts.stake_info.day_of_epoch = 0;
        Ok(())
    }

    // pub fn back_treasury (
    //     ctx: Context<BackTreasury>,
    //     treasury_nonce: u8
    // ) -> Result<()> {
    //     let dest_starting_lamports = ctx.accounts.admin_account.lamports();

    //     let account = ctx.accounts.treasury_account.to_account_info();
    //     **ctx.accounts.admin_account.lamports.borrow_mut() = dest_starting_lamports
    //         .checked_add(account.lamports())
    //         .unwrap();
    //     **account.lamports.borrow_mut() = 0;

    //     let mut data = account.try_borrow_mut_data()?;
    //     for byte in data.deref_mut().iter_mut() {
    //         *byte = 0;
    //     }

    //     let dst: &mut [u8] = &mut data;
    //     let mut cursor = Cursor::new(dst);
    //     cursor.write_all(&CLOSED_ACCOUNT_DISCRIMINATOR).unwrap();
    //     Ok(())
    // }
}


#[derive(Accounts)]
pub struct Initialize<'info> {
    // The account which have the fee from other contracts fee + implementation fix fee
    #[account(mut)]
    pub bank_account: Signer<'info>,
    // Create Epoch Account
    #[account(
        init,
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
        payer = bank_account,
        space = 8 + size_of::<EpochState>()
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
    #[account(
        init,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
        payer = bank_account,
        space = 8 + size_of::<StakeInfoState>()
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    
    // Treasury account for each epoch
    /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    
    pub treasury_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


#[derive(Accounts)]
pub struct StakeUser<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"stake_user".as_ref(),
            user_account.key().as_ref(),
        ],
        bump,
        payer = user_account,
        space = 8 + size_of::<StakeUserState>()
    )]
    pub stake_user: Box<Account<'info, StakeUserState>>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct EpochBonus<'info> {
    // The account which have the fee from other contracts fee + implementation fix fee
    #[account(mut)]
    pub bonus_account: Signer<'info>,
    // Treasury account for each epoch
        /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    pub treasury_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartEpoch<'info> {
    // The account which have the fee from other contracts fee + implementation fix fee
    #[account(mut)]
    pub bank_account: Signer<'info>,
    // Treasury account for each epoch
        /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    pub treasury_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeMerkle<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"Epoch-Merkle-Whitelist".as_ref(),
            b"Solluminati-NFT-List".as_ref(),
            admin_account.key().to_bytes().as_ref()
        ],
        bump,
        payer = admin_account,
        space = 8 + size_of::<Merkle>()
    )]
    pub merkle: Box<Account<'info, Merkle>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMerkle<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        mut,
        has_one = admin_account
    )]
    pub merkle: Box<Account<'info, Merkle>>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct StakeNft<'info> {
    // user who stack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        mut,
        constraint = user_nft_token_account.amount == 1,
        constraint = user_nft_token_account.owner == user_account.key(),
        constraint = user_nft_token_account.mint == nft_mint.key()
    )]
    pub user_nft_token_account: Box<Account<'info, TokenAccount>>,
    // NFT mint
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        seeds = [ 
            b"user-stake".as_ref(),
            nft_mint.key().as_ref(),
            user_account.key().as_ref(),
        ],
        bump,
        payer = user_account,
        space = 8 + size_of::<StakeNftInfoState>()
    )]
    pub stake_info_account: Box<Account<'info, StakeNftInfoState>>,
    #[account(
        init,
        seeds = [
            b"vault-stake".as_ref(),
            nft_mint.key().as_ref(),
            user_account.key().as_ref(),
        ],
        bump,
        payer = user_account,
        token::mint = nft_mint,
        token::authority = nft_authority,
    )]
    pub nft_vault_account: Box<Account<'info, TokenAccount>>,
        /// CHECK: Safe account
    #[account(
        seeds = [
            b"vault-stake-auth".as_ref(),
        ],
        bump,
    )]
    pub nft_authority: AccountInfo<'info>,
    pub merkle: Box<Account<'info, Merkle>>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,

}

#[derive(Accounts)]
pub struct UnStakeNft<'info> {
    // user who unstack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        mut,
        constraint = user_nft_token_account.owner == user_account.key(),
        constraint = user_nft_token_account.mint == nft_mint.key()
    )]
    pub user_nft_token_account: Box<Account<'info, TokenAccount>>,
    // NFT mint
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        constraint = nft_vault_account.owner == vault_auth.key(),
        constraint = nft_vault_account.mint == nft_mint.key()
    )]
    pub nft_vault_account: Box<Account<'info, TokenAccount>>,
    
    #[account(
        mut, 
        has_one = user_account,
        close = user_account
    )]
    pub stake_info_account: Box<Account<'info, StakeNftInfoState>>,
    /// CHECK: Safe account
    pub vault_auth: AccountInfo<'info>,
    pub merkle: Box<Account<'info, Merkle>>,
    #[account(
        mut,
        has_one = user_account
    )]
    pub stake_user: Box<Account<'info, StakeUserState>>,
    #[account(
        mut,
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    // user account who stack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
        /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    pub treasury_account: AccountInfo<'info>,
    #[account(
        mut,
        has_one = user_account
    )]
    pub stake_user: Box<Account<'info, StakeUserState>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DailyReward<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    pub stake_nft_info: Box<Account<'info, StakeNftInfoState>>,
    #[account(mut)]
    pub stake_user_account: Box<Account<'info, StakeUserState>>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    #[account(
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
}

#[derive(Accounts)]
pub struct UpdateDayEpoch<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
    #[account(
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump,
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
}

#[derive(Accounts)]
pub struct RestartEpoch<'info> {
    #[account(mut)]
    pub bank_account: Signer<'info>,
    // Create Epoch Account
    #[account(
        mut,
        seeds = [
            b"epoch-state".as_ref(),
        ],
        bump
    )]
    pub epoch_state: Box<Account<'info, EpochState>>,
    #[account(
        mut,
        seeds = [
            b"stake-info".as_ref(),
        ],
        bump,
    )]
    pub stake_info: Box<Account<'info, StakeInfoState>>,
        /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    pub treasury_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BackTreasury<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
        /// CHECK: Safe account
    #[account(
        mut,
        seeds = [
            b"epoch-treasury".as_ref(),
        ],
        bump,
    )]
    pub treasury_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> StakeNft<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .user_nft_token_account
                .to_account_info()
                .clone(),
            to: self.nft_vault_account.to_account_info().clone(),
            authority: self.user_account.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

impl<'info> UnStakeNft<'info> {
    fn into_transfer_to_user_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .nft_vault_account
                .to_account_info()
                .clone(),
            to: self.user_nft_token_account.to_account_info().clone(),
            authority: self.vault_auth.clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[account]
#[derive(Default)]
pub struct EpochState {
    pub is_initial: bool,
    pub epoch_no: u8,
    pub epoch_start_time: i64,
    pub cur_epoch_reward_per_day: u64,
    pub epoch_bonus: u64,
    pub remain_reward: u64
}

#[account]
pub struct StakeInfoState {
    pub is_initial: bool,
    pub total_stakers: u16,
    pub day_of_epoch: u8
}

#[account]
pub struct StakeNftInfoState {
    pub user_account: Pubkey,
    pub nft_mint: Pubkey,
    pub stake_time: i64,
}

#[account]
pub struct StakeUserState {
    pub is_initial: bool,
    pub user_account: Pubkey,
    pub reward_amount: u64,
    pub pending_amount: u64,
}

#[account]
#[derive(Default)]
pub struct Merkle {
    /// The 256-bit merkle root.
    pub root: [u8; 32],
    pub admin_account: Pubkey,
    pub is_init: bool
}