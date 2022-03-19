# Uniswap V2 AMM implemented in Anchor

## Supported Instructions 

- `programs/ammv2/src/`
    - `instructions/`
        - `init_pool.rs`: initialize a new pool
        - `liqduidity.rs`: add and remove liquidity 
        - `swap.rs`: perform a token swap 
    - `draft.rs`: outline of program with comments -- drafted before implementation 

## Implemented Tests 

- `tests/ammv2.ts`: 
    - intialize a new pool 
    - add liquidity (x3)
    - remove liquidity 
    - swap 
    - remove liquidity 
