use anchor_lang::prelude::*;

use crate::{
    constants::{CONFIG_SEED, MARKET_SEED},
    error::LaunchpadError,
    events::{GraduationFinalized, MarketStatusChanged},
    state::{DexKind, GlobalConfig, LaunchMarket, MarketStatus},
};

#[derive(Accounts)]
pub struct ManageMarket<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(
        mut,
        seeds = [MARKET_SEED, market.base_mint.as_ref()],
        bump = market.bump,
        has_one = config
    )]
    pub market: Account<'info, LaunchMarket>,
}

fn require_authority(ctx: &Context<ManageMarket>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.authority.key(),
        ctx.accounts.config.authority,
        LaunchpadError::Unauthorized
    );
    Ok(())
}

fn emit_status(market_key: Pubkey, market: &LaunchMarket, previous_status: MarketStatus) {
    emit!(MarketStatusChanged {
        market: market_key,
        previous_status,
        status: market.status,
        external_pool: market.external_pool,
    });
}

pub fn handle_set_paused(ctx: Context<ManageMarket>, paused: bool) -> Result<()> {
    require_authority(&ctx)?;
    let market_key = ctx.accounts.market.key();
    let market = &mut ctx.accounts.market;
    let previous = market.status;
    if paused {
        require!(
            market.status == MarketStatus::Trading,
            LaunchpadError::InvalidMarketStatus
        );
        market.status = MarketStatus::Paused;
    } else {
        require!(
            market.status == MarketStatus::Paused,
            LaunchpadError::InvalidMarketStatus
        );
        market.status = MarketStatus::Trading;
    }
    emit_status(market_key, market, previous);
    Ok(())
}

pub fn handle_begin_graduation(ctx: Context<ManageMarket>) -> Result<()> {
    require_authority(&ctx)?;
    let market_key = ctx.accounts.market.key();
    let market = &mut ctx.accounts.market;
    require!(
        market.status == MarketStatus::Trading || market.status == MarketStatus::Graduating,
        LaunchpadError::InvalidMarketStatus
    );
    require!(
        market.real_quote_reserve >= market.graduation_threshold,
        LaunchpadError::GraduationThresholdNotReached
    );
    let previous = market.status;
    if market.status == MarketStatus::Trading {
        market.status = MarketStatus::Graduating;
    }
    emit_status(market_key, market, previous);
    Ok(())
}

pub fn handle_finalize_graduation(
    ctx: Context<ManageMarket>,
    external_pool: Pubkey,
    dex: DexKind,
) -> Result<()> {
    require_authority(&ctx)?;
    require!(
        external_pool != Pubkey::default(),
        LaunchpadError::InvalidExternalPool
    );
    let market_key = ctx.accounts.market.key();
    let market = &mut ctx.accounts.market;
    require!(
        market.status == MarketStatus::Graduating,
        LaunchpadError::InvalidMarketStatus
    );
    let previous = market.status;
    market.external_pool = external_pool;
    market.graduated_at = Clock::get()?.unix_timestamp;
    market.status = MarketStatus::Graduated;
    emit_status(market_key, market, previous);
    emit!(GraduationFinalized {
        market: market_key,
        base_mint: market.base_mint,
        dex,
        external_pool,
        graduated_at: market.graduated_at,
    });
    Ok(())
}
