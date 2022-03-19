use anchor_lang::prelude::*;
use anchor_spl::{
    token,
    token::{Token, TokenAccount, Transfer},
};

use crate::state::PoolState;
use crate::error::ErrorCode;

pub fn swap(
    ctx: Context<Swap>, 
    amount_in: u64, 
    min_amount_out: u64,
) -> Result<()> {

    let src_balance = ctx.accounts.user_src.amount;
    require!(src_balance >= amount_in, ErrorCode::NotEnoughBalance);

    let u128_amount_in = amount_in as u128;

    let pool_state = &ctx.accounts.pool_state; 
    let src_vault_amount = ctx.accounts.vault_src.amount as u128;
    let dst_vault_amount = ctx.accounts.vault_dst.amount as u128;

    // minus fees 
    let fee_amount = u128_amount_in
        .checked_mul(pool_state.fee_numerator as u128).unwrap()
        .checked_div(pool_state.fee_denominator as u128).unwrap(); 
    let amount_in_minus_fees = u128_amount_in - fee_amount; 

    // compute output amount using constant product equation 
    let invariant = src_vault_amount.checked_mul(dst_vault_amount).unwrap();
    let new_src_vault = src_vault_amount + amount_in_minus_fees; 
    let new_dst_vault = invariant.checked_div(new_src_vault).unwrap(); 
    let output_amount = dst_vault_amount.checked_sub(new_dst_vault).unwrap();

    // revert if not enough out
    require!(output_amount >= min_amount_out as u128, ErrorCode::NotEnoughOut);

    // output_amount -> user_dst
    let bump = *ctx.bumps.get("pool_authority").unwrap();
    let pool_key = ctx.accounts.pool_state.key();
    let pda_sign = &[b"authority", pool_key.as_ref(), &[bump]];
    
    token::transfer(CpiContext::new(
        ctx.accounts.token_program.to_account_info(), 
        Transfer {
            from: ctx.accounts.vault_dst.to_account_info(), 
            to: ctx.accounts.user_dst.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(), 
        }
    ).with_signer(&[pda_sign]), output_amount as u64)?;

    // amount_in -> vault (including fees for LPs)
    token::transfer(CpiContext::new(
        ctx.accounts.token_program.to_account_info(), 
        Transfer {
            from: ctx.accounts.user_src.to_account_info(), 
            to: ctx.accounts.vault_src.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(), 
        }
    ), amount_in)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Swap<'info> {

    // pool token accounts 
    #[account(mut)]
    pub pool_state: Box<Account<'info, PoolState>>,

    #[account(mut, seeds=[b"authority", pool_state.key().as_ref()], bump)]
    pub pool_authority: AccountInfo<'info>,
    #[account(mut, 
        constraint=vault_src.owner == pool_authority.key(),
        constraint=vault_src.mint == user_src.mint,
    )]
    pub vault_src: Box<Account<'info, TokenAccount>>, 
    #[account(mut, 
        constraint=vault_dst.owner == pool_authority.key(),
        constraint=vault_src.mint == user_src.mint,
    )]
    pub vault_dst: Box<Account<'info, TokenAccount>>,
    
    // user token accounts 
    #[account(mut,
        has_one=owner,
    )]
    pub user_src: Box<Account<'info, TokenAccount>>, 
    #[account(mut,
        has_one=owner,
    )]
    pub user_dst: Box<Account<'info, TokenAccount>>, 
    pub owner: Signer<'info>,

    // other 
    pub token_program: Program<'info, Token>,
}


