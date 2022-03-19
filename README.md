# Uniswap V2 AMM implemented in Anchor

## Supported Instructions 

- `programs/ammv2/src/`
    - `instructions/`
        - `init_pool.rs`: initialize a new pool
        - `liqduidity.rs`: add and remove liquidity 
        - `swap.rs`: perform a token swap 

## Implemented Tests 

- `tests/ammv2.ts`: 
    - intialize a new pool 
    - add liqduidity (x3)
    - remove liquidity 
    - swap 
    - remove liquidity 