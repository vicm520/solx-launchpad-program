use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

use crate::{
    constants::{BASE_VAULT_SEED, CONFIG_SEED, MARKET_SEED, QUOTE_VAULT_SEED, SOL_VAULT_SEED},
    error::LaunchpadError,
    events::MigrationAssetsReleased,
    state::{GlobalConfig, LaunchMarket, MarketStatus, QuoteMode},
};

#[derive(Accounts)]
pub struct ReleaseMigrationAssetsSol<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, GlobalConfig>>,
    #[account(
        mut,
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = config,
        has_one = base_vault,
    )]
    pub market: Box<Account<'info, LaunchMarket>>,
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [BASE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub base_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = base_mint, token::authority = authority)]
    pub authority_base_token: Box<Account<'info, TokenAccount>>,
    /// CHECK: System-owned PDA used by the market to hold SOL reserves.
    #[account(mut, seeds = [SOL_VAULT_SEED, base_mint.key().as_ref()], bump = market.sol_vault_bump)]
    pub sol_vault: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseMigrationAssetsSolx<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, GlobalConfig>>,
    #[account(
        mut,
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = config,
        has_one = base_vault,
        has_one = quote_vault,
    )]
    pub market: Box<Account<'info, LaunchMarket>>,
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [BASE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub base_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = base_mint, token::authority = authority)]
    pub authority_base_token: Box<Account<'info, TokenAccount>>,
    #[account(address = config.solx_mint @ LaunchpadError::InvalidTokenAccount)]
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [QUOTE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, token::authority = authority)]
    pub authority_quote_token: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

fn require_authority(authority: &Signer, config: &GlobalConfig) -> Result<()> {
    require_keys_eq!(
        authority.key(),
        config.authority,
        LaunchpadError::Unauthorized
    );
    Ok(())
}

fn require_releasable(market: &LaunchMarket) -> Result<()> {
    require!(
        market.status == MarketStatus::Graduating,
        LaunchpadError::InvalidMarketStatus
    );
    require!(
        market.external_pool == Pubkey::default(),
        LaunchpadError::MigrationAlreadyReleased
    );
    Ok(())
}

pub fn handle_release_migration_assets_sol(ctx: Context<ReleaseMigrationAssetsSol>) -> Result<()> {
    require_authority(&ctx.accounts.authority, &ctx.accounts.config)?;
    require_releasable(&ctx.accounts.market)?;

    let token_amount = ctx
        .accounts
        .market
        .real_token_reserve
        .checked_add(ctx.accounts.market.liquidity_token_amount)
        .ok_or(LaunchpadError::MathOverflow)?;
    let quote_amount = ctx.accounts.market.real_quote_reserve;
    require!(
        token_amount > 0 && quote_amount > 0,
        LaunchpadError::InsufficientLiquidity
    );

    let market_key = ctx.accounts.base_mint.key();
    let market_bump = [ctx.accounts.market.bump];
    let market_signer: &[&[u8]] = &[MARKET_SEED, market_key.as_ref(), &market_bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.base_vault.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.authority_base_token.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[market_signer],
        ),
        token_amount,
        ctx.accounts.base_mint.decimals,
    )?;

    let rent_reserve = Rent::get()?.minimum_balance(0);
    let available = ctx
        .accounts
        .sol_vault
        .lamports()
        .saturating_sub(rent_reserve);
    require!(
        available >= quote_amount,
        LaunchpadError::InsufficientLiquidity
    );
    let vault_bump = [ctx.accounts.market.sol_vault_bump];
    let vault_signer: &[&[u8]] = &[SOL_VAULT_SEED, market_key.as_ref(), &vault_bump];
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.sol_vault.to_account_info(),
                to: ctx.accounts.authority.to_account_info(),
            },
            &[vault_signer],
        ),
        quote_amount,
    )?;

    // Once the reserves leave the bonding-curve vaults they must no longer be
    // spendable by a second migration attempt. The amounts are preserved by
    // the event and the Convex graduation record.
    ctx.accounts.market.real_token_reserve = 0;
    ctx.accounts.market.real_quote_reserve = 0;
    ctx.accounts.market.liquidity_token_amount = 0;

    emit!(MigrationAssetsReleased {
        market: ctx.accounts.market.key(),
        base_mint: ctx.accounts.base_mint.key(),
        token_amount,
        quote_amount,
        quote_mode: QuoteMode::Sol,
    });
    Ok(())
}

pub fn handle_release_migration_assets_solx(
    ctx: Context<ReleaseMigrationAssetsSolx>,
) -> Result<()> {
    require_authority(&ctx.accounts.authority, &ctx.accounts.config)?;
    require_releasable(&ctx.accounts.market)?;

    let token_amount = ctx
        .accounts
        .market
        .real_token_reserve
        .checked_add(ctx.accounts.market.liquidity_token_amount)
        .ok_or(LaunchpadError::MathOverflow)?;
    let quote_amount = ctx.accounts.market.real_quote_reserve;
    require!(
        token_amount > 0 && quote_amount > 0,
        LaunchpadError::InsufficientLiquidity
    );

    let market_key = ctx.accounts.base_mint.key();
    let market_bump = [ctx.accounts.market.bump];
    let market_signer: &[&[u8]] = &[MARKET_SEED, market_key.as_ref(), &market_bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.base_vault.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.authority_base_token.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[market_signer],
        ),
        token_amount,
        ctx.accounts.base_mint.decimals,
    )?;
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.quote_vault.to_account_info(),
                mint: ctx.accounts.quote_mint.to_account_info(),
                to: ctx.accounts.authority_quote_token.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[market_signer],
        ),
        quote_amount,
        ctx.accounts.quote_mint.decimals,
    )?;

    ctx.accounts.market.real_token_reserve = 0;
    ctx.accounts.market.real_quote_reserve = 0;
    ctx.accounts.market.liquidity_token_amount = 0;

    emit!(MigrationAssetsReleased {
        market: ctx.accounts.market.key(),
        base_mint: ctx.accounts.base_mint.key(),
        token_amount,
        quote_amount,
        quote_mode: QuoteMode::Solx,
    });
    Ok(())
}
