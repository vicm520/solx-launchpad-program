use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

use crate::{
    constants::{
        BASE_VAULT_SEED, BURN_VAULT_SEED, MARKET_SEED, QUOTE_VAULT_SEED, REWARD_VAULT_SEED,
        SOL_VAULT_SEED,
    },
    error::LaunchpadError,
    events::MarketCreated,
    math::allocation_for,
    state::{DexKind, GlobalConfig, LaunchMarket, MarketStatus, QuoteMode},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMarketArgs {
    pub quote_mode: QuoteMode,
    pub target_dex: DexKind,
    pub curve_allocation_bps: u16,
    pub liquidity_allocation_bps: u16,
    pub ecosystem_allocation_bps: u16,
    pub total_supply: u64,
    pub curve_token_amount: u64,
    pub liquidity_token_amount: u64,
    pub ecosystem_token_amount: u64,
    pub graduation_threshold: u64,
    pub virtual_token_reserve: u64,
    pub virtual_quote_reserve: u64,
}

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(seeds = [crate::constants::CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, GlobalConfig>>,
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = creator,
        space = 8 + LaunchMarket::INIT_SPACE,
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump
    )]
    pub market: Box<Account<'info, LaunchMarket>>,
    #[account(
        mut,
        token::mint = base_mint,
        token::authority = creator,
    )]
    pub creator_base_token: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = creator,
        token::mint = base_mint,
        token::authority = market,
        seeds = [BASE_VAULT_SEED, base_mint.key().as_ref()],
        bump,
    )]
    pub base_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: A system-owned PDA funded with the zero-data rent reserve at market creation.
    #[account(mut, seeds = [SOL_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub sol_vault: UncheckedAccount<'info>,
    #[account(address = config.solx_mint @ LaunchpadError::InvalidTokenAccount)]
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = creator,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [QUOTE_VAULT_SEED, base_mint.key().as_ref()],
        bump,
    )]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = creator,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [BURN_VAULT_SEED, base_mint.key().as_ref()],
        bump,
    )]
    pub burn_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = creator,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [REWARD_VAULT_SEED, base_mint.key().as_ref()],
        bump,
    )]
    pub reward_vault: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_market(ctx: Context<CreateMarket>, args: CreateMarketArgs) -> Result<()> {
    require!(
        !ctx.accounts.config.paused,
        LaunchpadError::InvalidMarketStatus
    );
    let allocation_total = args
        .curve_token_amount
        .checked_add(args.liquidity_token_amount)
        .and_then(|value| value.checked_add(args.ecosystem_token_amount))
        .ok_or(LaunchpadError::MathOverflow)?;
    require!(
        allocation_total == args.total_supply,
        LaunchpadError::InvalidAllocationTotal
    );
    let allocation_bps_total = args
        .curve_allocation_bps
        .checked_add(args.liquidity_allocation_bps)
        .and_then(|value| value.checked_add(args.ecosystem_allocation_bps))
        .ok_or(LaunchpadError::MathOverflow)?;
    require!(
        allocation_bps_total == 10_000,
        LaunchpadError::InvalidAllocationRatio
    );
    require!(
        (4_000..=9_000).contains(&args.curve_allocation_bps)
            && (1_000..=5_000).contains(&args.liquidity_allocation_bps)
            && args.ecosystem_allocation_bps <= 4_000,
        LaunchpadError::UnsafeAllocationRatio
    );
    let expected_curve = allocation_for(args.total_supply, args.curve_allocation_bps)?;
    let expected_liquidity = allocation_for(args.total_supply, args.liquidity_allocation_bps)?;
    let expected_ecosystem = args
        .total_supply
        .checked_sub(expected_curve)
        .and_then(|value| value.checked_sub(expected_liquidity))
        .ok_or(LaunchpadError::MathOverflow)?;
    require!(
        args.curve_token_amount == expected_curve
            && args.liquidity_token_amount == expected_liquidity
            && args.ecosystem_token_amount == expected_ecosystem,
        LaunchpadError::InvalidAllocationRatio
    );
    require!(
        args.graduation_threshold > 0
            && args.virtual_token_reserve > 0
            && args.virtual_quote_reserve > 0,
        LaunchpadError::InvalidCurveParameters
    );
    require!(
        ctx.accounts.creator_base_token.amount == args.total_supply,
        LaunchpadError::InvalidAllocationTotal
    );
    require!(
        ctx.accounts.base_mint.supply == args.total_supply,
        LaunchpadError::InvalidAllocationTotal
    );

    // Curve inventory and the dedicated graduation allocation are protocol
    // custody. The creator/ecosystem share remains in the creator ATA.
    let protocol_custody_amount = args
        .curve_token_amount
        .checked_add(args.liquidity_token_amount)
        .ok_or(LaunchpadError::MathOverflow)?;
    token::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.creator_base_token.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            },
        ),
        protocol_custody_amount,
        ctx.accounts.base_mint.decimals,
    )?;

    let sol_vault_rent = Rent::get()?.minimum_balance(0);
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.creator.to_account_info(),
                to: ctx.accounts.sol_vault.to_account_info(),
            },
        ),
        sol_vault_rent,
    )?;

    let market = &mut ctx.accounts.market;
    market.config = ctx.accounts.config.key();
    market.creator = ctx.accounts.creator.key();
    market.base_mint = ctx.accounts.base_mint.key();
    market.base_vault = ctx.accounts.base_vault.key();
    market.quote_vault = ctx.accounts.quote_vault.key();
    market.burn_vault = ctx.accounts.burn_vault.key();
    market.reward_vault = ctx.accounts.reward_vault.key();
    market.quote_mode = args.quote_mode;
    market.status = MarketStatus::Trading;
    market.total_supply = args.total_supply;
    market.curve_token_amount = args.curve_token_amount;
    market.liquidity_token_amount = args.liquidity_token_amount;
    market.ecosystem_token_amount = args.ecosystem_token_amount;
    market.graduation_threshold = args.graduation_threshold;
    market.virtual_token_reserve = args.virtual_token_reserve;
    market.virtual_quote_reserve = args.virtual_quote_reserve;
    market.real_token_reserve = args.curve_token_amount;
    market.real_quote_reserve = 0;
    market.cumulative_quote_volume = 0;
    market.cumulative_mechanism_fees = 0;
    market.external_pool = Pubkey::default();
    market.created_at = Clock::get()?.unix_timestamp;
    market.graduated_at = 0;
    market.bump = ctx.bumps.market;
    market.sol_vault_bump = ctx.bumps.sol_vault;

    emit!(MarketCreated {
        market: market.key(),
        creator: market.creator,
        base_mint: market.base_mint,
        quote_mode: market.quote_mode,
        target_dex: args.target_dex,
        total_supply: market.total_supply,
        graduation_threshold: market.graduation_threshold,
    });
    Ok(())
}
