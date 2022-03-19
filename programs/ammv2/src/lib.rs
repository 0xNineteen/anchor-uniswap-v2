use anchor_lang::prelude::*;
use anchor_spl::{
    token,
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, Transfer, Burn},
};

pub mod error; 
use crate::error::ErrorCode;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod ammv2 {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>, 
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Result<()> {

        let pool_state = &mut ctx.accounts.pool_state;
        pool_state.fee_numerator = fee_numerator;
        pool_state.fee_denominator = fee_denominator;
        pool_state.total_amount_minted = 0; 

        Ok(())
    }

    pub fn remove_liquidity(
        ctx: Context<LiquidityOperation>, 
        burn_amount: u64,
    ) -> Result<()> {

        let pool_mint_balance = ctx.accounts.user_pool_ata.amount; 
        require!(burn_amount <= pool_mint_balance, NotEnoughBalance);

        let pool_key = ctx.accounts.pool_state.key();
        let state = &mut ctx.accounts.pool_state;
        require!(state.total_amount_minted >= burn_amount, BurnTooMuch);
        
        let vault0_amount = ctx.accounts.vault0.amount as u128;
        let vault1_amount = ctx.accounts.vault1.amount as u128;
        let u128_burn_amount = burn_amount as u128;

        // compute how much to give back 
        let [amount0, amount1] = [
            u128_burn_amount
                .checked_mul(vault0_amount).unwrap()
                .checked_div(state.total_amount_minted as u128).unwrap() as u64,
            u128_burn_amount
                .checked_mul(vault1_amount).unwrap()
                .checked_div(state.total_amount_minted as u128).unwrap() as u64
        ];

        // deposit user funds into vaults
        let bump = *ctx.bumps.get("pool_authority").unwrap();
        let pda_sign = &[b"authority", pool_key.as_ref(), &[bump]];
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.vault0.to_account_info(), 
                to: ctx.accounts.user0.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(), 
            }
        ).with_signer(&[pda_sign]), amount0)?;

        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.vault1.to_account_info(), 
                to: ctx.accounts.user1.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(), 
            }
        ).with_signer(&[pda_sign]), amount1)?;

        // burn pool tokens 
        token::burn(CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Burn { 
                mint: ctx.accounts.pool_mint.to_account_info(), 
                to: ctx.accounts.user_pool_ata.to_account_info(), 
                authority:  ctx.accounts.owner.to_account_info(),
            }
        ).with_signer(&[pda_sign]), burn_amount)?;

        state.total_amount_minted -= burn_amount; 

        Ok(())
    }

    pub fn add_liquidity(
        ctx: Context<LiquidityOperation>, 
        amount_liq0: u64, // amount of token0 
        // amount of token1
            // note: only needed on pool init deposit 
            // ... can derive it once exchange is up
        amount_liq1: u64, 
    ) -> Result<()> {

        let user_balance0 = ctx.accounts.user0.amount; 
        let user_balance1 = ctx.accounts.user1.amount; 

        // ensure enough balance 
        require!(amount_liq0 <= user_balance0, NotEnoughBalance);
        require!(amount_liq1 <= user_balance1, NotEnoughBalance);

        let vault_balance0 = ctx.accounts.vault0.amount;
        let vault_balance1 = ctx.accounts.vault1.amount;
        let pool_state = &mut ctx.accounts.pool_state; 
        
        let deposit0 = amount_liq0;
        // vars to fill out during if statement  
        let deposit1; 
        let amount_to_mint;
        
        // initial deposit
        msg!("vaults: {} {}", vault_balance0, vault_balance1);
        msg!("init deposits: {} {}", amount_liq0, amount_liq1);

        if vault_balance0 == 0 && vault_balance1 == 0 {
            // bit shift (a + b)/2
            amount_to_mint = (amount_liq0 + amount_liq1) >> 1; 
            deposit1 = amount_liq1;
        } else { 
            // require equal amount deposit based on pool exchange rate 
            let exchange01 = vault_balance0.checked_div(vault_balance1).unwrap();
            let amount_deposit_1 = amount_liq0.checked_mul(exchange01).unwrap();
            msg!("new deposits: {} {} {}", exchange01, amount_liq0, amount_deposit_1);

            // enough funds + user is ok with it in single check 
            require!(amount_deposit_1 <= amount_liq1, NotEnoughBalance);
            deposit1 = amount_deposit_1; // update liquidity amount ! 

            // mint = relative to the entire pool + total amount minted 
            // u128 so we can do multiply first without overflow 
            // then div and recast back 
            amount_to_mint = (
                (deposit1 as u128)
                .checked_mul(pool_state.total_amount_minted as u128).unwrap()
                .checked_div(vault_balance1 as u128).unwrap()
            ) as u64;

            msg!("pmint: {}", amount_to_mint);
        }

        // saftey checks 
        require!(amount_to_mint > 0, NoPoolMintOutput);

        // give pool_mints 
        pool_state.total_amount_minted += amount_to_mint;
        let mint_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            MintTo {
                to: ctx.accounts.user_pool_ata.to_account_info(),
                mint: ctx.accounts.pool_mint.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            }
        );
        let bump = *ctx.bumps.get("pool_authority").unwrap();
        let pool_key = ctx.accounts.pool_state.key();
        let pda_sign = &[b"authority", pool_key.as_ref(), &[bump]];
        token::mint_to(
            mint_ctx.with_signer(&[pda_sign]), 
            amount_to_mint
        )?;
        
        // deposit user funds into vaults
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.user0.to_account_info(), 
                to: ctx.accounts.vault0.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(), 
            }
        ), deposit0)?;

        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.user1.to_account_info(), 
                to: ctx.accounts.vault1.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(), 
            }
        ), deposit1)?;

        Ok(())
    }

    pub fn swap(
        ctx: Context<Swap>, 
        amount_in: u64, 
        min_amount_out: u64,
    ) -> Result<()> {

        let src_balance = ctx.accounts.user_src.amount;
        require!(src_balance >= amount_in, NotEnoughBalance);

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
        require!(output_amount >= min_amount_out as u128, NotEnoughOut);

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

#[derive(Accounts)]
pub struct LiquidityOperation<'info> {

    // pool token accounts 
    #[account(mut)]
    pub pool_state: Box<Account<'info, PoolState>>,
    
    #[account(seeds=[b"authority", pool_state.key().as_ref()], bump)]
    pub pool_authority: AccountInfo<'info>,
    #[account(mut, 
        constraint = vault0.mint == user0.mint,
        seeds=[b"vault0", pool_state.key().as_ref()], bump)]
    pub vault0: Box<Account<'info, TokenAccount>>, 
    #[account(mut, 
        constraint = vault1.mint == user1.mint,
        seeds=[b"vault1", pool_state.key().as_ref()], bump)]
    pub vault1: Box<Account<'info, TokenAccount>>,
    #[account(mut, 
        constraint = user_pool_ata.mint == pool_mint.key(),
        seeds=[b"pool_mint", pool_state.key().as_ref()], bump)]
    pub pool_mint: Box<Account<'info, Mint>>,  
    
    // user token accounts 
    #[account(mut, has_one = owner)]
    pub user0: Box<Account<'info, TokenAccount>>, 
    #[account(mut, has_one = owner)]
    pub user1: Box<Account<'info, TokenAccount>>, 
    #[account(mut, has_one = owner)]
    pub user_pool_ata: Box<Account<'info, TokenAccount>>, 
    pub owner: Signer<'info>,

    // other 
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    // pool for token_x -> token_y 
    pub mint0: Account<'info, Mint>,
    pub mint1: Account<'info, Mint>,

    #[account(
        init, 
        payer=payer, 
        seeds=[b"pool_state", mint0.key().as_ref(), mint1.key().as_ref()], 
        bump,
    )]
    pub pool_state: Box<Account<'info, PoolState>>,

    // authority so 1 acc pass in can derive all other pdas 
    #[account(seeds=[b"authority", pool_state.key().as_ref()], bump)]
    pub pool_authority: AccountInfo<'info>,

    // account to hold token X
    #[account(
        init, 
        payer=payer, 
        seeds=[b"vault0", pool_state.key().as_ref()], 
        bump,
        token::mint = mint0,
        token::authority = pool_authority
    )]
    pub vault0: Box<Account<'info, TokenAccount>>, 
    // account to hold token Y
    #[account(
        init, 
        payer=payer, 
        seeds=[b"vault1", pool_state.key().as_ref()],
        bump,
        token::mint = mint1,
        token::authority = pool_authority
    )]
    pub vault1: Box<Account<'info, TokenAccount>>, 

    // pool mint : used to track relative contribution amount of LPs
    #[account(
        init, 
        payer=payer,
        seeds=[b"pool_mint", pool_state.key().as_ref()], 
        bump, 
        mint::decimals = 9,
        mint::authority = pool_authority
    )] 
    pub pool_mint: Box<Account<'info, Mint>>, 
    #[account(mut)]
    pub payer: Signer<'info>,

    // accounts required to init a new mint
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
#[derive(Default)] // defaults to zeros -- which we want 
pub struct PoolState {
    pub total_amount_minted: u64, 
    pub fee_numerator: u64, 
    pub fee_denominator: u64,
}
