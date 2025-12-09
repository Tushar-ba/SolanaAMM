use std::ops::Mul;
use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata,
    },
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, Burn, Transfer}
};

declare_id!("HvM5J3JTPXViGexcxkDHKtBXvv8vwLMcZSrpqKimBFHr");

#[program]
pub mod amm {
    use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
    use anchor_spl::token;
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee:u8) -> Result<()> {
        let amm_info = &mut ctx.accounts.amm;
        amm_info.user = ctx.accounts.signer.key();
        amm_info.fee = fee;
        Ok(())
    }
    
    pub fn create_pool(ctx:Context<CreatePool>, token_amount_a:u64, token_amount_b:u64)-> Result<()>{
        require!(ctx.accounts.user_token_account_a.amount >= token_amount_a, ErrorCode::InsufficientBalance);
        require!(ctx.accounts.user_token_account_b.amount >= token_amount_b, ErrorCode::InsufficientBalance);

        // 1. TRANSFER TOKENS INTO VAULTS
        let cpi_ctx_mint_a = Transfer{
            from: ctx.accounts.user_token_account_a.to_account_info(),
            to: ctx.accounts.pool_token_account_a.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_b = Transfer{
            from: ctx.accounts.user_token_account_b.to_account_info(),
            to: ctx.accounts.pool_token_account_b.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };

        // These are standard transfers, user signs
        anchor_spl::token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_a), token_amount_a)?;
        anchor_spl::token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_b), token_amount_b)?;

        // 2. PREPARE POOL SEEDS (The Pool is the Authority)
        let mint_a = ctx.accounts.mint_a.key();
        let mint_b = ctx.accounts.mint_b.key();
        let pool_bump = ctx.bumps.pool;
        
        // Seeds must match the Pool PDA derivation
        let seeds = &[
            b"pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
            &[pool_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        // 3. CREATE METADATA (Pool signs as authority)
        let cpi_accounts_metadata = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata_account.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            mint_authority: ctx.accounts.pool.to_account_info(), // Pool is authority
            payer: ctx.accounts.signer.to_account_info(),
            update_authority: ctx.accounts.pool.to_account_info(), // Pool is authority
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        
        let cpi_ctx_metadata = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            cpi_accounts_metadata,
            signer_seeds, // Pool signs
        );

        let data_v2 = DataV2 {
            name: "Liquidity Tokens".to_string(),
            symbol: "LP".to_string(),
            uri: "https://brown-wonderful-whale-251.mypinata.cloud/ipfs/bafybeiag6suuu7vp23jpdl3o3kut5rtmuvhkvsx7elgt3m2c7n54lv66p4".to_string(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        create_metadata_accounts_v3(cpi_ctx_metadata, data_v2, true, true, None)?;
        
        // Save Pool State
        let pool_info = &mut ctx.accounts.pool;
        pool_info.mint_a = ctx.accounts.mint_a.key();
        pool_info.mint_b = ctx.accounts.mint_b.key();

        // 4. MINT LP TOKENS (Pool signs as authority)
        let cpi_accounts_mint = MintTo{
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_token_account.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(), // Pool is authority
        };

        let cpi_context_mint = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts_mint, 
            signer_seeds // Pool signs
        );

        // Calculate Initial LP (sqrt(x*y))
        let multiply = token_amount_a as u128 * token_amount_b as u128; // Use u128 to prevent overflow
        let lp_amount = (multiply as f64).sqrt() as u64; 

        anchor_spl::token::mint_to(cpi_context_mint, lp_amount)?;

        msg!("Pool created for {} and {}", ctx.accounts.mint_a.key(), ctx.accounts.mint_b.key());
        Ok(())
    }

    pub fn add_liquidity(ctx:Context<AddLiquidity>, token_amount_a:u64, token_amount_b:u64) -> Result<()>{
        // 1. Transfer Tokens
        let cpi_ctx_mint_a = Transfer{
            from: ctx.accounts.user_token_account_a.to_account_info(),
            to: ctx.accounts.pool_token_account_a.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_b = Transfer{
            from: ctx.accounts.user_token_account_b.to_account_info(),
            to: ctx.accounts.pool_token_account_b.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };

        anchor_spl::token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_a), token_amount_a)?;
        anchor_spl::token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_b), token_amount_b)?;

        // 2. Mint LP Tokens
        let mint_a = ctx.accounts.mint_a.key();
        let mint_b = ctx.accounts.mint_b.key();
        let pool_bump = ctx.bumps.pool;
        
        let seeds = &[
            b"pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
            &[pool_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts_mint = MintTo{
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_token_account.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(), // Pool signs
        };

        let cpi_context = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts_mint, signer_seeds);

        // WARNING: Simplified Math. 
        // In a real AMM, for add_liquidity (after creation), you calculate ratio based on existing supply, not sqrt(a*b)
        let multiply = token_amount_a as u128 * token_amount_b as u128;
        let lp_amount = (multiply as f64).sqrt() as u64;

        anchor_spl::token::mint_to(cpi_context, lp_amount)?;

        Ok(())
    } 

    pub fn remove_liquidity(ctx:Context<RemoveLiquidity>, lp_amount:u64) -> Result<()>{
        let total_supply = ctx.accounts.lp_mint.supply;
        let reserve_a = ctx.accounts.pool_token_account_a.amount;
        let reserve_b = ctx.accounts.pool_token_account_b.amount;

        let amount_a = (reserve_a as u128).checked_mul(lp_amount as u128).unwrap().checked_div(total_supply as u128).unwrap() as u64;
        let amount_b = (reserve_b as u128).checked_mul(lp_amount as u128).unwrap().checked_div(total_supply as u128).unwrap() as u64;
  
        let cpi_burn = Burn{
            mint: ctx.accounts.lp_mint.to_account_info(),
            from: ctx.accounts.lp_mint_token_account.to_account_info(),
            authority:ctx.accounts.signer.to_account_info(),
        };
        let cpi_accounts = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_burn);
        anchor_spl::token::burn(cpi_accounts, lp_amount)?;

        let mint_a_key = ctx.accounts.token_a_mint.key();
        let mint_b_key = ctx.accounts.token_b_mint.key();
        let bump = ctx.bumps.pool;

        let seeds = &[b"pool",mint_a_key.as_ref(), mint_b_key.as_ref(),&[bump]];
        let signer_seeds = &[&seeds[..]];

        let cpi_trasfer_a = Transfer{
            from: ctx.accounts.pool_token_account_a.to_account_info(),
            to: ctx.accounts.user_token_account_a.to_account_info(),
            authority:ctx.accounts.pool.to_account_info(),
        };

        let cpi_token_a = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_trasfer_a, signer_seeds);
        anchor_spl::token::transfer(cpi_token_a, amount_a)?;

        let cpi_trasfer_b = Transfer{
            from: ctx.accounts.pool_token_account_a.to_account_info(),
            to: ctx.accounts.user_token_account_a.to_account_info(),
            authority:ctx.accounts.pool.to_account_info(),
        };

        let cpi_token_b = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_trasfer_b, signer_seeds);
        anchor_spl::token::transfer(cpi_token_b, amount_b)?;
        Ok(())
    }

    pub fn swap(
        ctx: Context<Swap>, 
        amount_in: u64, 
        min_amount_out: u64
    ) -> Result<()> {
        
        // 1. Determine Direction (A -> B or B -> A)
        let is_a_to_b = ctx.accounts.input_mint.key() == ctx.accounts.pool.mint_a;
        
        // Setup variables based on direction
        let (
            input_token_account,  // User's Source
            pool_receive_account, // Pool's Vault for Input
            pool_pay_account,     // Pool's Vault for Output
            user_receive_account  // User's Destination
        ) = if is_a_to_b {
            (
                &ctx.accounts.user_token_account_a,
                &ctx.accounts.pool_token_account_a,
                &ctx.accounts.pool_token_account_b,
                &ctx.accounts.user_token_account_b,
            )
        } else {
            (
                &ctx.accounts.user_token_account_b,
                &ctx.accounts.pool_token_account_b,
                &ctx.accounts.pool_token_account_a,
                &ctx.accounts.user_token_account_a,
            )
        };

        // 2. Transfer Input (User -> Pool)
        // User signs this transaction, so we can transfer from their account
        let cpi_accounts_in = Transfer {
            from: input_token_account.to_account_info(),
            to: pool_receive_account.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_ctx_in = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_in
        );
        anchor_spl::token::transfer(cpi_ctx_in, amount_in)?;

        // 3. Calculate Output Amount (Constant Product Formula)
        // Load reserves *after* the transfer? 
        // Note: For safety, standard AMMs usually calculate based on pre-transfer balance, 
        // or account for the amount_in we just added.
        // Let's use the balances:
        let reserve_in = pool_receive_account.amount; // This essentially includes the amount_in we just sent if we reload, but let's do the math carefully.
        let reserve_out = pool_pay_account.amount;

        // Formula: dy = (y * dx) / (x + dx)
        // x = reserve_in (before deposit)
        // y = reserve_out
        // dx = amount_in
        // Note: Since we ALREADY transferred amount_in, pool_receive_account.amount is (x + dx).
        // So we subtract it back for the calculation "x".
        
        // Reload the account to get updated balance
        let pool_receive_account_loaded = anchor_spl::token::accessor::amount(&pool_receive_account.to_account_info())?;
        let actual_amount_in = amount_in; // Simplified. In prod, check difference in balance for deflationary tokens.
        
        let reserve_in_before = pool_receive_account_loaded - actual_amount_in;

        let amount_out = calculate_amm_output(
            actual_amount_in,
            reserve_in_before,
            reserve_out
        ).ok_or(ErrorCode::MathOverflow)?;

        // 4. Check Slippage
        require!(amount_out >= min_amount_out, ErrorCode::SlippageExceeded);

        // 5. Transfer Output (Pool -> User)
        // Pool PDA signs this
        let mint_a = ctx.accounts.pool.mint_a;
        let mint_b = ctx.accounts.pool.mint_b;
        let bump = ctx.bumps.pool;
        
        let seeds = &[
            b"pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
            &[bump]
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts_out = Transfer {
            from: pool_pay_account.to_account_info(),
            to: user_receive_account.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
        let cpi_ctx_out = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_out,
            signer_seeds
        );
        anchor_spl::token::transfer(cpi_ctx_out, amount_out)?;

        msg!("Swapped {} tokens for {}", amount_in, amount_out);
        Ok(())
    }
}

// Helper function for Constant Product Math
// Output = (Input * Reserve_Out) / (Reserve_In + Input)
fn calculate_amm_output(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Option<u64> {
    let amount_in = amount_in as u128;
    let reserve_in = reserve_in as u128;
    let reserve_out = reserve_out as u128;

    let numerator = amount_in.checked_mul(reserve_out)?;
    let denominator = reserve_in.checked_add(amount_in)?;

    let amount_out = numerator.checked_div(denominator)?;
    
    Some(amount_out as u64)
}

#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init, // Changed to init (init_if_needed is rare for global configs)
        payer = signer,
        space = 8 + Amm::INIT_SPACE,
        seeds = [b"AMM"],
        bump
    )]
    pub amm: Account<'info, Amm>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreatePool<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint_a: Account<'info,Mint>,
    pub mint_b: Account<'info,Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = signer
    )]
    pub user_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = signer,
    )]
    pub user_token_account_b: Account<'info,TokenAccount>,

    #[account(
        init,
        payer = signer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump 
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = signer,
        associated_token::mint = mint_a,
        associated_token::authority = pool,
    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        associated_token::mint = mint_b,
        associated_token::authority = pool,
    )]
    pub pool_token_account_b: Account<'info,TokenAccount>,

    #[account(
        init,
        payer = signer, 
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
        mint::decimals = 9,
        mint::authority = pool, // <--- CORRECT AUTHORITY
        mint::freeze_authority = pool, // <--- CORRECT AUTHORITY
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [
            b"metadata",
            metadata_program.key().as_ref(),
            lp_mint.key().as_ref()
        ],
        bump,
        seeds::program = metadata_program.key()
    )]
    /// CHECK: Metaplex check
    pub metadata_account: UncheckedAccount<'info>,

    #[account(
        init,
        payer = signer,
        associated_token::mint = lp_mint,
        associated_token::authority = signer,
    )]
    pub user_lp_token_account: Account<'info, TokenAccount>,

    pub token_program:Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub mint_a: Account<'info,Mint>,

    #[account(mut)]
    pub mint_b: Account<'info,Mint>,

    #[account(
        mut, // Must be mutable to mint tokens (if storing state)
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump  
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = signer
    )]
    pub user_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = signer,
    )]
    pub user_token_account_b: Account<'info,TokenAccount>,

    #[account(
        mut, // No init_if_needed, it must exist
        associated_token::mint = mint_a,
        associated_token::authority = pool,
    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool,
    )]
    pub pool_token_account_b: Account<'info,TokenAccount>,

    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = signer,
    )]
    pub user_lp_token_account: Account<'info, TokenAccount>,

    pub token_program:Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    ///unchecked account
    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = signer,
    )]
    pub lp_mint_token_account: Account<'info, TokenAccount>,

    ///unchecked account
    #[account(mut)]
    pub token_a_mint: Account<'info, Mint>,

    ///unchecked account
    #[account(mut)]
    pub token_b_mint: Account<'info, Mint>,

    #[account(
        mut, 
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        associated_token::mint = token_a_mint,
        associated_token::authority = pool,
    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = token_b_mint,
        associated_token::authority = pool,
    )]
    pub pool_token_account_b: Account<'info, TokenAccount>,


    #[account(
        mut,
        associated_token::mint = token_a_mint,
        associated_token::authority = signer,
    )]
    pub user_token_account_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        associated_token::mint = token_b_mint,
        associated_token::authority = signer,
    )]
    pub user_token_account_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info,Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info,System>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump,
    )]
    pub pool: Account<'info, Pool>,

    // This tells us WHICH token the user is selling
    pub input_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = pool.mint_a,
        associated_token::authority = pool
    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_b,
        associated_token::authority = pool
    )]
    pub pool_token_account_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_a,
        associated_token::authority = signer
    )]
    pub user_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_b,
        associated_token::authority = signer
    )]
    pub user_token_account_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Amm{
    pub user: Pubkey,
    pub fee:u8,
    pub lp_mint:Pubkey,
}

#[account]
#[derive(InitSpace)]
 pub struct Pool{
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
 }

 #[error_code]
 pub enum ErrorCode {
     #[msg("user account does not have balance")]
     InsufficientBalance,
    #[msg("Slippage Tolerance Exceeded")]
    SlippageExceeded,
    #[msg("Math Overflow")]
    MathOverflow,
 }