use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::{
    constants::{CONFIG_SEED, MARKET_SEED, PLATFORM_VERIFICATION_SEED},
    error::LaunchpadError,
    events::PlatformVerificationUpdated,
    state::{GlobalConfig, LaunchMarket, PlatformVerification},
};

#[derive(Accounts)]
pub struct CreatePlatformVerification<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, GlobalConfig>,
    pub base_mint: Account<'info, Mint>,
    #[account(
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = config,
        constraint = market.base_mint == base_mint.key() @ LaunchpadError::InvalidTokenAccount,
    )]
    pub market: Account<'info, LaunchMarket>,
    #[account(
        init,
        payer = authority,
        space = 8 + PlatformVerification::INIT_SPACE,
        seeds = [PLATFORM_VERIFICATION_SEED, base_mint.key().as_ref()],
        bump,
    )]
    pub verification: Account<'info, PlatformVerification>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePlatformVerification<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, GlobalConfig>,
    pub base_mint: Account<'info, Mint>,
    #[account(
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = config,
        constraint = market.base_mint == base_mint.key() @ LaunchpadError::InvalidTokenAccount,
    )]
    pub market: Account<'info, LaunchMarket>,
    #[account(
        mut,
        seeds = [PLATFORM_VERIFICATION_SEED, base_mint.key().as_ref()],
        bump = verification.bump,
        constraint = verification.mint == base_mint.key() @ LaunchpadError::InvalidTokenAccount,
    )]
    pub verification: Account<'info, PlatformVerification>,
}

fn require_authority(authority: &Signer, config: &GlobalConfig) -> Result<()> {
    require_keys_eq!(
        authority.key(),
        config.authority,
        LaunchpadError::Unauthorized
    );
    Ok(())
}

fn emit_verification(verification: &PlatformVerification) {
    emit!(PlatformVerificationUpdated {
        mint: verification.mint,
        reviewer: verification.reviewer,
        verified: verification.verified,
        reason_hash: verification.reason_hash,
        updated_at: verification.updated_at,
    });
}

pub fn handle_create_platform_verification(
    ctx: Context<CreatePlatformVerification>,
    verified: bool,
    reason_hash: [u8; 32],
) -> Result<()> {
    require_authority(&ctx.accounts.authority, &ctx.accounts.config)?;
    let now = Clock::get()?.unix_timestamp;
    let verification = &mut ctx.accounts.verification;
    verification.mint = ctx.accounts.base_mint.key();
    verification.reviewer = ctx.accounts.authority.key();
    verification.verified = verified;
    verification.reason_hash = reason_hash;
    verification.created_at = now;
    verification.updated_at = now;
    verification.bump = ctx.bumps.verification;
    emit_verification(verification);
    Ok(())
}

pub fn handle_update_platform_verification(
    ctx: Context<UpdatePlatformVerification>,
    verified: bool,
    reason_hash: [u8; 32],
) -> Result<()> {
    require_authority(&ctx.accounts.authority, &ctx.accounts.config)?;
    let verification = &mut ctx.accounts.verification;
    verification.reviewer = ctx.accounts.authority.key();
    verification.verified = verified;
    verification.reason_hash = reason_hash;
    verification.updated_at = Clock::get()?.unix_timestamp;
    emit_verification(verification);
    Ok(())
}
