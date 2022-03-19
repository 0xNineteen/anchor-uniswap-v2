import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Ammv2 } from "../target/types/ammv2";

import { assert } from "chai";
import * as token from "@solana/spl-token";
import * as web3 from "@solana/web3.js";

interface Pool {
  auth: web3.Keypair,
  payer: web3.Keypair,
  mint0: web3.PublicKey,
  mint1: web3.PublicKey,
  vault0: web3.PublicKey,
  vault1: web3.PublicKey,
  poolMint: web3.PublicKey,
  poolState: web3.PublicKey,
  poolAuth: web3.PublicKey,
}

interface LPProvider {
  signer: web3.Keypair,
  user0: web3.PublicKey, 
  user1: web3.PublicKey, 
  poolAta: web3.PublicKey
}

describe("ammv2", () => {
  // Configure the client to use the local cluster.
    // Configure the client to use the local cluster.
    let provider = anchor.Provider.env();
    let connection = provider.connection;
    anchor.setProvider(provider);
  
    const program = anchor.workspace.Ammv2 as Program<Ammv2>;

    let pool: Pool; // async describe in chai doesnt play nice :| so we pass this var around
    let n_decimals = 9

    it("initializes a new pool", async () => {
      let auth = web3.Keypair.generate();
      let sig = await connection.requestAirdrop(auth.publicKey, 100 * web3.LAMPORTS_PER_SOL);
      await connection.confirmTransaction(sig);
    
      let mint0 = await token.createMint(
        connection, 
        auth, 
        auth.publicKey, 
        auth.publicKey, 
        n_decimals, 
      );
      let mint1 = await token.createMint(
        connection, 
        auth, 
        auth.publicKey, 
        auth.publicKey, 
        n_decimals, 
      );

      let [poolState, poolState_b] = await web3.PublicKey.findProgramAddress(
        [Buffer.from("pool_state"), mint0.toBuffer(), mint1.toBuffer()], 
        program.programId,
      );

      // all derive from state
      let [authority, authority_b] = await web3.PublicKey.findProgramAddress(
        [Buffer.from("authority"), poolState.toBuffer()], 
        program.programId,
      );
      let [vault0, vault0_b] = await web3.PublicKey.findProgramAddress(
        [Buffer.from("vault0"), poolState.toBuffer()], 
        program.programId,
      );
      let [vault1, vault1_b] = await web3.PublicKey.findProgramAddress(
        [Buffer.from("vault1"), poolState.toBuffer()], 
        program.programId,
      );
      let [poolMint, poolMint_b] = await web3.PublicKey.findProgramAddress(
        [Buffer.from("pool_mint"), poolState.toBuffer()], 
        program.programId,
      );

      //  1/10K = 0.01% fees 
      let fee_numerator = new anchor.BN(1);
      let fee_denominator = new anchor.BN(10000);
  
      await program.rpc.initializePool(fee_numerator, fee_denominator, {
        accounts: {
          mint0: mint0, 
          mint1: mint1, 
          poolAuthority: authority,
          vault0: vault0,
          vault1: vault1,
          poolMint: poolMint,
          poolState: poolState,
          // the rest 
          payer: provider.wallet.publicKey, 
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: token.TOKEN_PROGRAM_ID, 
          associatedTokenProgram: token.ASSOCIATED_TOKEN_PROGRAM_ID, 
          rent: web3.SYSVAR_RENT_PUBKEY
        }
      });
  
      pool = {
        auth: auth,
        payer: auth,
        mint0: mint0,
        mint1: mint1,
        vault0: vault0,
        vault1: vault1,
        poolMint: poolMint,
        poolState: poolState,
        poolAuth: authority, 
      }
    
    });

      // helper function 
  async function setup_lp_provider(lp_user: web3.PublicKey, amount: number) {
    // setup token accs for deposit 
    let mint0_ata = await token.createAssociatedTokenAccount(
      connection, pool.payer, pool.mint0, lp_user);
    let mint1_ata = await token.createAssociatedTokenAccount(
      connection, pool.payer, pool.mint1, lp_user);

    // setup token accs for LP pool tokens 
    let pool_mint_ata = await token.createAssociatedTokenAccount(
      connection, pool.payer, pool.poolMint, lp_user);

    // setup initial balance of mints 
    await token.mintTo(connection, 
      pool.payer, 
      pool.mint0, 
      mint0_ata, 
      pool.auth, 
      amount * 10 ** n_decimals
    );
    await token.mintTo(connection, 
      pool.payer, 
      pool.mint1, 
      mint1_ata, 
      pool.auth, 
      amount * 10 ** n_decimals
    );

    return [mint0_ata, mint1_ata, pool_mint_ata]
  }

  async function get_token_balance(pk) {
    return (await connection.getTokenAccountBalance(pk)).value.uiAmount;
  }

  function lp_amount(n) {
    return new anchor.BN(n * 10 ** n_decimals)
  }

  let lp_user0: LPProvider; // to be filled in 
  it("adds initial liquidity to the pool", async () => {
    let lp_user_signer = web3.Keypair.generate();
    let lp_user = lp_user_signer.publicKey;
    let [user0, user1, poolAta] = await setup_lp_provider(lp_user, 100);

    lp_user0 = { // here 
      signer: lp_user_signer,
      user0: user0, 
      user1: user1, 
      poolAta: poolAta
    };

    let [src_amount0_in, src_amount1_in] = [
      lp_amount(50), 
      lp_amount(50)
    ];
    await program.rpc.addLiquidity(src_amount0_in, src_amount1_in, {
      accounts: {
        // pool stuff 
        poolAuthority: pool.poolAuth,
        vault0: pool.vault0, 
        vault1: pool.vault1, 
        poolMint: pool.poolMint,
        poolState: pool.poolState, 
        // LP user stuff 
        user0: user0, 
        user1: user1, 
        userPoolAta: poolAta,
        owner: lp_user,
        // other stuff
        tokenProgram: token.TOKEN_PROGRAM_ID, 
      },
      signers: [lp_user_signer]
    });

    // intializing pool liquidity -- mints 1 of each ... = 100% of pool amount 
    let balance_mint0 = await get_token_balance(poolAta)
    let poolState = await program.account.poolState.fetch(pool.poolState);
    let amountTotalMint = poolState.totalAmountMinted.toNumber();
    console.log("depsoiter0 pool mint: ", balance_mint0);
    console.log("total amount mint", amountTotalMint);

    assert(balance_mint0 > 0);

    // ensure vault got some 
    let vb0 = await get_token_balance(pool.vault0);
    let vb1 = await get_token_balance(pool.vault1);
    console.log(vb0)
    console.log(vb1)
    
    assert(vb0 > 0);
    assert(vb1 > 0);
    assert(vb1 == vb0); // 1:1
  })

  let lp_user1: LPProvider; // to be filled in 
  it("adds 2nd liquidity to the pool", async () => {
    let lp_user_signer = web3.Keypair.generate();
    let lp_user = lp_user_signer.publicKey;
    let [user0, user1, poolAta] = await setup_lp_provider(lp_user, 100);

    lp_user1 = { // here 
      signer: lp_user_signer,
      user0: user0, 
      user1: user1, 
      poolAta: poolAta
    };

    let [src_amount0_in, src_amount1_in] = [
      lp_amount(50), 
      lp_amount(50)
    ];
    await program.rpc.addLiquidity(src_amount0_in, src_amount1_in, {
      accounts: {
        // pool stuff 
        poolAuthority: pool.poolAuth,
        vault0: pool.vault0, 
        vault1: pool.vault1, 
        poolMint: pool.poolMint,
        poolState: pool.poolState, 
        // LP user stuff 
        user0: user0, 
        user1: user1, 
        userPoolAta: poolAta,
        owner: lp_user,
        // other stuff
        tokenProgram: token.TOKEN_PROGRAM_ID, 
      },
      signers: [lp_user_signer]
    });

    // intializing pool liquidity -- mints 1 of each ... = 100% of pool amount 
    let balance_mint0 = await get_token_balance(poolAta)
    let poolState = await program.account.poolState.fetch(pool.poolState);
    let amountTotalMint = poolState.totalAmountMinted.toNumber();
    console.log("depsoiter1 pool mint: ", balance_mint0);
    console.log("total amount mint", amountTotalMint);

    assert(balance_mint0 > 0);

    // ensure vault got some 
    let vb0 = await get_token_balance(pool.vault0);
    let vb1 = await get_token_balance(pool.vault1);
    console.log(vb0)
    console.log(vb1)
    
    assert(vb0 > 0);
    assert(vb1 > 0);
    assert(vb1 == vb0); // 1:1
  })

  it("adds 3rd liquidity to the pool", async () => {
    let lp_user_signer = web3.Keypair.generate();
    let lp_user = lp_user_signer.publicKey;
    let [user0, user1, poolAta] = await setup_lp_provider(lp_user, 100);

    let [src_amount0_in, src_amount1_in] = [
      lp_amount(25), 
      lp_amount(100)
    ];
    await program.rpc.addLiquidity(src_amount0_in, src_amount1_in, {
      accounts: {
        // pool stuff 
        poolAuthority: pool.poolAuth,
        vault0: pool.vault0, 
        vault1: pool.vault1, 
        poolMint: pool.poolMint,
        poolState: pool.poolState, 
        // LP user stuff 
        user0: user0, 
        user1: user1, 
        userPoolAta: poolAta,
        owner: lp_user,
        // other stuff
        tokenProgram: token.TOKEN_PROGRAM_ID, 
      },
      signers: [lp_user_signer]
    });

    // intializing pool liquidity -- mints 1 of each ... = 100% of pool amount 
    let balance_mint0 = await get_token_balance(poolAta)
    let poolState = await program.account.poolState.fetch(pool.poolState);
    let amountTotalMint = poolState.totalAmountMinted.toNumber();

    console.log("depsoiter pool mint: ", balance_mint0);
    console.log("total amount mint", amountTotalMint);

    // assert(balance_mint0 == 25 * 10 ** 9);
    // assert(balance_mint0 + 100 * 10 ** 9 == amountTotalMint);

    // ensure vault got some 
    let vb0 = await get_token_balance(pool.vault0);
    let vb1 = await get_token_balance(pool.vault1);
    console.log(vb0)
    console.log(vb1)
    
    assert(vb0 > 0);
    assert(vb1 > 0);
    assert(vb1 == vb0); // still 1:1
  })


  it("removes liquidity", async () => {

    let b_user0 = await get_token_balance(lp_user0.user0);
    let b_user1 = await get_token_balance(lp_user0.user1);
    let balance_mint0 = await get_token_balance(lp_user0.poolAta)

    await program.rpc.removeLiquidity(lp_amount(50), {
      accounts: {
        // pool stuff 
        poolAuthority: pool.poolAuth,
        vault0: pool.vault0, 
        vault1: pool.vault1, 
        poolMint: pool.poolMint,
        poolState: pool.poolState, 
        // LP user stuff 
        user0: lp_user0.user0, 
        user1: lp_user0.user1, 
        userPoolAta: lp_user0.poolAta,
        owner: lp_user0.signer.publicKey,
        // other stuff
        tokenProgram: token.TOKEN_PROGRAM_ID, 
      },
      signers: [lp_user0.signer]
    });

    let b_user0_2 = await get_token_balance(lp_user0.user0);
    let b_user1_2 = await get_token_balance(lp_user0.user1);
    let balance_mint0_2 = await get_token_balance(lp_user0.poolAta)

    // console.log(
    //   balance_mint0, balance_mint0_2,
    //   b_user0, b_user0_2,
    //   b_user1, b_user1_2
    // )

    assert(balance_mint0 > balance_mint0_2);
    assert(b_user0 < b_user0_2);
    assert(b_user1 < b_user1_2);

    // ensure vault got some 
    let vb0 = await get_token_balance(pool.vault0);
    let vb1 = await get_token_balance(pool.vault1);
    console.log(vb0)
    console.log(vb1)
    
  });

  it("swaps",async () => {
      let swapper_signer = web3.Keypair.generate();
      let swapper = swapper_signer.publicKey;

      // setup token accs for deposit 
      let mint0_ata = await token.createAssociatedTokenAccount(
        connection, pool.payer, pool.mint0, swapper);
      let mint1_ata = await token.createAssociatedTokenAccount(
        connection, pool.payer, pool.mint1, swapper);
  
      // setup initial balance of mints 
      let amount = 100; 
      await token.mintTo(connection, 
        pool.payer, 
        pool.mint0, 
        mint0_ata, 
        pool.auth, 
        amount * 10 ** n_decimals
      );

      let b0 = await get_token_balance(mint0_ata);
      let b1 = await get_token_balance(mint1_ata);
      
      // token0 -> token1
      await program.rpc.swap(new anchor.BN(10 * 10 ** n_decimals), new anchor.BN(0),{
        accounts: {
            poolState: pool.poolState,
            poolAuthority: pool.poolAuth,
            vaultSrc: pool.vault0,
            vaultDst: pool.vault1,
            userSrc: mint0_ata,
            userDst: mint1_ata,
            owner: swapper,
            tokenProgram: token.TOKEN_PROGRAM_ID,
        },
        signers: [swapper_signer]
      });

      let new_b0 = await get_token_balance(mint0_ata);
      let new_b1 = await get_token_balance(mint1_ata);

      assert(new_b0 < b0);
      assert(new_b1 > b1);
  });

  it("removes liquidity after a swap", async () => {

    let b_user0 = await get_token_balance(lp_user1.user0);
    let b_user1 = await get_token_balance(lp_user1.user1);
    let balance_mint0 = await get_token_balance(lp_user1.poolAta)

    await program.rpc.removeLiquidity(lp_amount(50), {
      accounts: {
        // pool stuff 
        poolAuthority: pool.poolAuth,
        vault0: pool.vault0, 
        vault1: pool.vault1, 
        poolMint: pool.poolMint,
        poolState: pool.poolState, 
        // LP user stuff 
        user0: lp_user1.user0, 
        user1: lp_user1.user1, 
        userPoolAta: lp_user1.poolAta,
        owner: lp_user1.signer.publicKey,
        // other stuff
        tokenProgram: token.TOKEN_PROGRAM_ID, 
      },
      signers: [lp_user1.signer]
    });

    let b_user0_2 = await get_token_balance(lp_user1.user0);
    let b_user1_2 = await get_token_balance(lp_user1.user1);
    let balance_mint0_2 = await get_token_balance(lp_user1.poolAta)

    console.log(
      balance_mint0, balance_mint0_2,
      b_user0, b_user0_2,
      b_user1, b_user1_2
    )

    assert(balance_mint0 > balance_mint0_2);
    assert(b_user0 < b_user0_2);
    assert(b_user1 < b_user1_2);

    // earned profit from previous swap! :D
    assert(b_user0_2 > b_user0 + 50); // more here 
    assert(b_user1_2 < b_user1 + 50); // less here (impermantent loss)

    // ensure vault got some 
    let vb0 = await get_token_balance(pool.vault0);
    let vb1 = await get_token_balance(pool.vault1);
    // console.log(vb0)
    // console.log(vb1)
    
  });  

});
