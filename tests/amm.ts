import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  ComputeBudgetProgram
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount
} from "@solana/spl-token";

describe("amm", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Amm as Program<Amm>;
  const wallet = provider.wallet as anchor.Wallet;

  // Constants
  const TOKEN_METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
  const mintA = new PublicKey("2qCdKiVLzHm8Je5caB8i63bYuNcwf34jmGTjWww5y3ks");
  const mintB = new PublicKey("Caa9Mn3FFjTi9jo17JvVDbNfo9hMYFiwaTJrzyD8wBv5");
  const userTokenAccountA = new PublicKey("77DU1EcMVQ4y4Yz1aApRMji1XWnLvXZPcPxXan6kkDPt");
  const userTokenAccountB = new PublicKey("Hbk6jsJNrxi9dMvo63ahZWqsaPXji59MSGDhh48HZHRU");

  // Test State Variables
  // let mintA: PublicKey;
  // let mintB: PublicKey;
  // let userTokenAccountA: PublicKey;
  // let userTokenAccountB: PublicKey;

  // Helper to create mints and fund the user
  const setupMint = async (decimals: number) => {
    const mint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      decimals
    );
    return mint;
  };

  it("Is initialized!", async () => {
    const [ammPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("AMM")],
      program.programId
    );

    // Check if AMM already exists to avoid error on repeated runs
    const info = await provider.connection.getAccountInfo(ammPDA);
    if (info) {
      console.log("AMM already initialized");
      return;
    }

    const fee = 3; // 3%
    const tx = await program.methods.initialize(fee)
      .accounts({
        signer: wallet.publicKey,
        amm: ammPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("AMM Initialized signature", tx);
  });

  // it("Should setup mints and fund user", async () => {
  //   // 1. Create fresh mints for this test run
  //   mintA = await setupMint(6);
  //   mintB = await setupMint(6);
  //   console.log("Mint A:", mintA.toBase58());
  //   console.log("Mint B:", mintB.toBase58());

  //   // 2. Create User ATAs
  //   const uA = await getOrCreateAssociatedTokenAccount(
  //     provider.connection,
  //     wallet.payer,
  //     mintA,
  //     wallet.publicKey
  //   );
  //   const uB = await getOrCreateAssociatedTokenAccount(
  //     provider.connection,
  //     wallet.payer,
  //     mintB,
  //     wallet.publicKey
  //   );
  //   userTokenAccountA = uA.address;
  //   userTokenAccountB = uB.address;

  //   // 3. Mint tokens to user so they can create the pool
  //   await mintTo(provider.connection, wallet.payer, mintA, userTokenAccountA, wallet.publicKey, 10_000_000_000);
  //   await mintTo(provider.connection, wallet.payer, mintB, userTokenAccountB, wallet.publicKey, 10_000_000_000);

  //   console.log("User funded with tokens");
  // });

  // it("Should create pool and mint LP tokens", async () => {
  //   // Amounts to deposit (e.g., 100 tokens)
  //   const tokenAmountA = new anchor.BN(100_000_000);
  //   const tokenAmountB = new anchor.BN(200_000_000);

  //   // 1. Sort Mints (Crucial for Deterministic PDA)
  //   // We strictly follow: if (A > B) swap(A, B)
  //   let sortedMintA = mintA;
  //   let sortedMintB = mintB;
  //   let sortedUserA = userTokenAccountA;
  //   let sortedUserB = userTokenAccountB;

  //   if (mintA.toBuffer().compare(mintB.toBuffer()) > 0) {
  //     [sortedMintA, sortedMintB] = [mintB, mintA];
  //     [sortedUserA, sortedUserB] = [userTokenAccountB, userTokenAccountA];
  //     console.log("Mints swapped for sorting order");
  //   }

  //   // 2. Derive PDAs
  //   const [pool] = PublicKey.findProgramAddressSync(
  //     [Buffer.from("pool"), sortedMintA.toBuffer(), sortedMintB.toBuffer()],
  //     program.programId
  //   );

  //   const [lpMint] = PublicKey.findProgramAddressSync(
  //     [Buffer.from("lp_mint"), sortedMintA.toBuffer(), sortedMintB.toBuffer()],
  //     program.programId
  //   );

  //   const [metadataAccount] = PublicKey.findProgramAddressSync(
  //     [
  //       Buffer.from("metadata"),
  //       TOKEN_METADATA_PROGRAM_ID.toBuffer(),
  //       lpMint.toBuffer(),
  //     ],
  //     TOKEN_METADATA_PROGRAM_ID
  //   );

  //   // 3. Derive Vault Addresses (Owned by Pool PDA)
  //   const poolTokenAccountA = await getAssociatedTokenAddress(
  //     sortedMintA,
  //     pool,
  //     true // allowOwnerOffCurve = true (because owner is a PDA)
  //   );

  //   const poolTokenAccountB = await getAssociatedTokenAddress(
  //     sortedMintB,
  //     pool,
  //     true
  //   );

  //   console.log("Pool PDA:", pool.toBase58());

  //   // 4. Derive User's LP Token Account
  //   const userLpTokenAccount = await getAssociatedTokenAddress(
  //     lpMint,
  //     wallet.publicKey,
  //     false
  //   );

  //   // 5. Execute Create Pool
  //   try {
  //     // Request more compute units - createPool does a lot of work
  //     const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
  //       units: 400_000,
  //     });

  //     const tx = await program.methods
  //       .createPool(tokenAmountA, tokenAmountB)
  //       .accounts({
  //         signer: wallet.publicKey,
  //         mintA: sortedMintA,
  //         mintB: sortedMintB,
  //         userTokenAccountA: sortedUserA,
  //         userTokenAccountB: sortedUserB,
  //         pool: pool,
  //         poolTokenAccountA: poolTokenAccountA,
  //         poolTokenAccountB: poolTokenAccountB,
  //         lpMint: lpMint,
  //         metadataAccount: metadataAccount,
  //         userLpTokenAccount: userLpTokenAccount,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //         systemProgram: SystemProgram.programId,
  //         metadataProgram: TOKEN_METADATA_PROGRAM_ID,
  //         rent: SYSVAR_RENT_PUBKEY,
  //       })
  //       .preInstructions([modifyComputeUnits])
  //       .rpc();

  //     console.log("Pool Created! Signature:", tx);
  //   } catch (e) {
  //     console.error("Error creating pool:", e);
  //     throw e;
  //   }
  // });

  it("should add liqudity for the pair created", async ()=>{
    const tokenAmountA = new anchor.BN(1_000_000_000);
    const tokenAmountB = new anchor.BN(2_000_000_000);
    try {
      const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
        units: 400_000,
      });

        //   // 1. Sort Mints (Crucial for Deterministic PDA)
  //   // We strictly follow: if (A > B) swap(A, B)
      let sortedMintA = mintA;
      let sortedMintB = mintB;
      let sortedUserA = userTokenAccountA;
      let sortedUserB = userTokenAccountB;

      if (mintA.toBuffer().compare(mintB.toBuffer()) > 0) {
        [sortedMintA, sortedMintB] = [mintB, mintA];
        [sortedUserA, sortedUserB] = [userTokenAccountB, userTokenAccountA];
        console.log("Mints swapped for sorting order");
      }

       const [pool] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), sortedMintA.toBuffer(), sortedMintB.toBuffer()],
      program.programId
    );

    const [lpMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), sortedMintA.toBuffer(), sortedMintB.toBuffer()],
      program.programId
    );

    const [metadataAccount] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        lpMint.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID
    );

    // 3. Derive Vault Addresses (Owned by Pool PDA)
    const poolTokenAccountA = await getAssociatedTokenAddress(
      sortedMintA,
      pool,
      true // allowOwnerOffCurve = true (because owner is a PDA)
    );

    const poolTokenAccountB = await getAssociatedTokenAddress(
      sortedMintB,
      pool,
      true
    );

    console.log("Pool PDA:", pool.toBase58());

    // 4. Derive User's LP Token Account
    const userLpTokenAccount = await getAssociatedTokenAddress(
      lpMint,
      wallet.publicKey,
      false
    );

      const tx = await program.methods
        .addLiquidity(tokenAmountA, tokenAmountB)
        .accounts({
          signer: wallet.publicKey,
          mintA: sortedMintA,
          mintB: sortedMintB,
          pool: pool,
          userTokenAccountA: sortedUserA,
          userTokenAccountB: sortedUserB,
          poolTokenAccountA: poolTokenAccountA,
          lpMint: lpMint,
          poolTokenAccountB: poolTokenAccountB,
          userLpTokenAccount: userLpTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .preInstructions([modifyComputeUnits])
        .rpc();

      console.log("Liquidity Added! Signature:", tx);
    } catch (e) {
      console.error("Error adding liquidity:", e);
      throw e;
    }
  })
});