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
declare_id!("68LxBK1wV34aNmJBcMLYpJKcmouwn7Eiv5TY5j4Pf3wp");

#[program]
pub mod amm {
    use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
    use anchor_spl::token;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee:u8) -> Result<()> {
        let amm_info = &mut ctx.accounts.amm;
        amm_info.user= ctx.accounts.signer.key();
        amm_info.fee= fee;
        Ok(())
    }
    
    pub fn create_pool(ctx:Context<CreatePool>, token_amount_a:u64, token_amount_b:u64)-> Result<()>{
        require!(ctx.accounts.user_token_account_a.amount>=token_amount_a, ErrorCode::InsufficientBalance);
        require!(ctx.accounts.user_token_account_b.amount>=token_amount_b, ErrorCode::InsufficientBalance);
        let mint_a = ctx.accounts.mint_a.key();
        let mint_b = ctx.accounts.mint_b.key();
        let seeds = &["lp_mint".as_bytes(), mint_a.as_ref(),mint_b.as_ref(),&[ctx.bumps.lp_mint]];
        let signer = &[&seeds[..]];

        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata_account.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            mint_authority: ctx.accounts.lp_mint.to_account_info(),
            payer: ctx.accounts.signer.to_account_info(),
            update_authority: ctx.accounts.lp_mint.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            cpi_accounts,
            signer,
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

        create_metadata_accounts_v3(cpi_ctx, data_v2, true, true, None)?;
        
        let pool_info = &mut ctx.accounts.pool;
        pool_info.mint_a = ctx.accounts.mint_a.key();
        pool_info.mint_b = ctx.accounts.mint_b.key();

        let cpi_ctx_mint_a = Transfer{
            from: ctx.accounts.user_token_account_a.to_account_info(),
            to:ctx.accounts.pool_token_account_a.to_account_info(),
            authority:ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_b = Transfer{
            from: ctx.accounts.user_token_account_b.to_account_info(),
            to:ctx.accounts.pool_token_account_b.to_account_info(),
            authority:ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_a_accounts = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_a);
        let cpi_ctx_mint_b_accounts = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_b);

        anchor_spl::token::transfer(cpi_ctx_mint_a_accounts, token_amount_a)?;
        anchor_spl::token::transfer(cpi_ctx_mint_b_accounts, token_amount_b)?;

        let cpi_accounts_mint = MintTo{
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.lp_mint.to_account_info(),
        };

        
        let mint_a = ctx.accounts.mint_a.key();
        let mint_b = ctx.accounts.mint_b.key();

        let seeds = &["lp_mint".as_bytes(),mint_a.as_ref(),mint_b.as_ref(),&[ctx.bumps.lp_mint]];
        let signers = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts_mint, signers);

        let multiply = token_amount_a * token_amount_b;
        let lp_amount = multiply.isqrt() * LAMPORTS_PER_SOL;

        anchor_spl::token::mint_to(cpi_context,lp_amount)?;

        msg!("pool created for token a with mint {} and {}", ctx.accounts.mint_a.key(),ctx.accounts.mint_b.key());
        Ok(())
    }

    pub fn add_liquidity(ctx:Context<AddLiquidity>,token_amount_a:u64, token_amount_b:u64) -> Result<()>{
        require!(ctx.accounts.user_token_account_a.amount>=token_amount_a, ErrorCode::InsufficientBalance);
        require!(ctx.accounts.user_token_account_b.amount>=token_amount_b, ErrorCode::InsufficientBalance);
        let pool_info = &mut ctx.accounts.pool;

        let amount_out:u64 = pool_info.get_amount(token_amount_a, ctx.accounts.pool_token_account_a.amount, ctx.accounts.pool_token_account_b.amount);
        

        pool_info.mint_a = ctx.accounts.mint_a.key();
        pool_info.mint_b = ctx.accounts.mint_b.key();

        let cpi_ctx_mint_a = Transfer{
            from: ctx.accounts.user_token_account_a.to_account_info(),
            to:ctx.accounts.pool_token_account_a.to_account_info(),
            authority:ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_b = Transfer{
            from: ctx.accounts.user_token_account_b.to_account_info(),
            to:ctx.accounts.pool_token_account_b.to_account_info(),
            authority:ctx.accounts.signer.to_account_info(),
        };

        let cpi_ctx_mint_a_accounts = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_a);
        let cpi_ctx_mint_b_accounts = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_ctx_mint_b);

        anchor_spl::token::transfer(cpi_ctx_mint_a_accounts, token_amount_a)?;
        anchor_spl::token::transfer(cpi_ctx_mint_b_accounts, token_amount_b)?;

        let cpi_accounts_mint = MintTo{
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.lp_mint.to_account_info(),
        };

        
        let mint_a = ctx.accounts.mint_a.key();
        let mint_b = ctx.accounts.mint_b.key();

        let seeds = &["lp_mint".as_bytes(),mint_a.as_ref(),mint_b.as_ref(),&[ctx.bumps.lp_mint]];
        let signers = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts_mint, signers);

        let multiply = token_amount_a * token_amount_b;
        let lp_amount = multiply.isqrt() * LAMPORTS_PER_SOL;

        anchor_spl::token::mint_to(cpi_context,lp_amount)?;


        Ok(())
    } 
}

impl Pool {
    pub fn get_amount(&self, token_amount_a:u64, token_reserve_a:u64, token_reserve_b:u64)-> u64{
        //let total_reserve:u64 = check_mul(token_reserve_a, token_reserve_b);
        let numerator = (token_amount_a as u128 * token_reserve_b as u128);
        let denominator:u128 = token_reserve_a as u128;
        let amount_out= numerator.checked_div(denominator); 
        amount_out.unwrap() as u64
    }
}

#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,


    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + Amm::INIT_SPACE,
        seeds = [b"AMM"],
        bump
    )]
    pub amm: Account<'info, Amm>,

    pub token_program: Program<'info, Token>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreatePool<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,


    ///UnCheckedAccount
    #[account(mut)]
    pub mint_a: Account<'info,Mint>,

    ///UnCheckedAccount
    #[account(mut)]
    pub mint_b: Account<'info,Mint>,

     ///UnCheckedAccount
     #[account(
        associated_token::mint = mint_a,
        associated_token::authority = signer
     )]
     pub user_token_account_a: Account<'info, TokenAccount>,

      ///UnCheckedAccount
    #[account(
        associated_token::mint = mint_b,
        associated_token::authority = signer,
    )]
    pub user_token_account_b: Account<'info,TokenAccount>,

    #[account(
        init,
        payer = signer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", mint_a.key().as_ref(),mint_b.key().as_ref()],
        bump 
    )]
    pub pool: Account<'info, Pool>,

      ///UnCheckedAccount
      #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint_a,
        associated_token::authority = pool,
     )]
     pub pool_token_account_a: Account<'info, TokenAccount>,

      ///UnCheckedAccount
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint_b,
        associated_token::authority = pool,
    )]
    pub pool_token_account_b: Account<'info,TokenAccount>,

    #[account(
        init_if_needed,
        payer = signer, 
        seeds = [b"lp_mint",mint_a.key().as_ref(),mint_b.key().as_ref()],
        bump,
        mint::decimals = 9,
        mint::authority = lp_mint,
        mint::freeze_authority = lp_mint,
    )]
    pub lp_mint: Account<'info, Mint>,

    /// CHECK: This account is validated by the metadata program CPI
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
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program:Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub mint_a: Account<'info,Mint>,

    ///UnCheckedAccount
    #[account(mut)]
    pub mint_b: Account<'info,Mint>,

    #[account(
        seeds = [b"pool", mint_a.key().as_ref(),mint_b.key().as_ref()],
        bump  
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        associated_token::mint = mint_a,
        associated_token::authority = signer
     )]
     pub user_token_account_a: Account<'info, TokenAccount>,

      ///UnCheckedAccount
    #[account(
        associated_token::mint = mint_b,
        associated_token::authority = signer,
    )]
    pub user_token_account_b: Account<'info,TokenAccount>,

    ///UnCheckedAccount
    #[account(
    init_if_needed,
    payer = signer,
    associated_token::mint = mint_a,
    associated_token::authority = pool,
    )]
    pub pool_token_account_a: Account<'info, TokenAccount>,

    ///UnCheckedAccount
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint_b,
        associated_token::authority = pool,
    )]
    pub pool_token_account_b: Account<'info,TokenAccount>,

    pub token_program:Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
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
 }


 