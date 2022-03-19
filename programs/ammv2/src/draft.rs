use anchor_lang::prelude::*;
use anchor_spl::{
    token,
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, Transfer, Burn},
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod concentrated_liquidity {
    use super::*;

    // accounts to initialize: 
        // vault0, vault1 : holds tokens
        // pool_mint : given to liquidity providers (LPs)
        // pool_state : track # of tokens minted  -- can mint object track this? 
    pub fn initialize_pool(_ctx: Context<Blank>) -> Result<()> {
        Ok(())
    }
        
    //       ** LP on uniswap ETH template for reference
    // ETH & tokenx pool 
    // 1. deposit ETH
    // 2. deposit equal amount of token (based on pool exchange rate)
    //      ... : deposit_x = (pool_eth / pool_token) * eth_deposit amount 
    // 3. mint amount = deposit_x / pool_x * total_pool_mints
    // 4. burn amount = [
    //  burn_amount/total_amount * eth_pool_amount, 
    //  burn_amount/total_amount of token
    //                  ]

    // Notes based on UniswapV1 whitepaper:
    // ** mint = deposit_x / pool_x * total_pool_mints
    // | deposit amount| => | mint equation | => | pool token mint amount |
    // 50, 50 => inital add => mint 1 & pool = 50
    // 50, 50 => 50 / 50 * 1  => mint 1 & pool = 100
    // 50, 50 => 50 / 100 * 2 => mint 1 & ... 
    // 50, 50 => 50 / 150 * 3 => mint 1
    // 100, 100 => 100 / 200 * 4 => mint 2 

    // requiring equal amount of tokenX to tokenY ENABLES single pool mint token 
    pub fn add_liquidity(
        ctx: Context<Blank>, 
        user_liq0: u64, // amount of token0 
        // amount of token1
            // note: only needed on pool init deposit 
            // ... can derive it once exchange is up
        user_liq1: u64, 
    ) -> Result<()> {

        // IF: 0th LP deposit 
            // ** geometric mean for uniswap v2 
            // amount_to_mint = sqrt(user_liq0 * user_liq1)

        // ELSE: 1th+ LP deposit
            // get : pool_balance0, pool_balance1
            // get : user_liq1 based on current pool exchange 
                // define: depo_user_liq1 = user_liq0 * pool_balance1 / pool_balance0
            // require : user_liq0 >= depo_user_liq1 -- they ok with deposit
            // require : enough funds
            // amount_to_mint = user_liq0 / pool_balance0 * total_pool_mints

        // ** trnsf token + pool token mint 
        // transfer token0: amount0_in -> vault0
        // transfer token1: amount1_in -> vault1
        // pool mint: amount_to_mint -> user_amount0_in

        // ** update pool mint count 
        // total_pool_mints += amount_to_mint 

        Ok(())
    }

    pub fn remove_liquidity(
        ctx: Context<Blank>, 
        burn_amount: u64
    ) -> Result<()> {
        // require : pool_token_balance > burn_amount
        // let total_amount = total # of pool tokens minted 
        // let [amount0, amount1] = [
        //  burn_amount/total_amount * pool_balance0, 
        //  burn_amount/total_amount * pool_balance1
        // ]

        // transfer from vault -> back to user 
        // burn pool tokens 
        // reduce total_amount

        Ok(())
    }

    // acccounts: 
    // src_token_account 
    // dst_token_account
    pub fn swap(
        ctx: Context<Blank>,
        amount_in: u64, 
    ) {
        // amount_in = amount_in * (1 - 0.03) [3% fee]
            // transfer: fee -> vault

        // derive : balance of pool_src & pool_dst
        // compute : constant product formula output
    }
}

#[derive(Accounts)]
pub struct Blank<'info> {}

