use anchor_lang::prelude::*;

use crate::{
    constants::{
        CONFIG_SEED, DEFAULT_CURVE_ALLOCATION_BPS, DEFAULT_ECOSYSTEM_ALLOCATION_BPS,
        DEFAULT_LIQUIDITY_ALLOCATION_BPS, MAX_MECHANISM_FEE_BPS,
    },
    error::LaunchpadError,
    events::ConfigInitialized,
    state::GlobalConfig,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeConfigArgs {
    pub solx_mint: Pubkey,
    pub graduation_program: Pubkey,
    pub mechanism_fee_bps: u16,
    pub burn_bps: u16,
    pub reward_bps: u16,
    pub equity_unit_tokens: u64,
    pub eligibility_delay_seconds: u32,
    pub min_buyback_interval_seconds: u32,
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + GlobalConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    args: InitializeConfigArgs,
) -> Result<()> {
    require!(
        args.mechanism_fee_bps == args.burn_bps.saturating_add(args.reward_bps),
        LaunchpadError::InvalidFeeSplit
    );
    require!(
        args.mechanism_fee_bps <= MAX_MECHANISM_FEE_BPS,
        LaunchpadError::FeeTooHigh
    );

    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.payer.key();
    config.solx_mint = args.solx_mint;
    config.graduation_program = args.graduation_program;
    config.mechanism_fee_bps = args.mechanism_fee_bps;
    config.burn_bps = args.burn_bps;
    config.reward_bps = args.reward_bps;
    config.curve_allocation_bps = DEFAULT_CURVE_ALLOCATION_BPS;
    config.liquidity_allocation_bps = DEFAULT_LIQUIDITY_ALLOCATION_BPS;
    config.ecosystem_allocation_bps = DEFAULT_ECOSYSTEM_ALLOCATION_BPS;
    config.equity_unit_tokens = args.equity_unit_tokens;
    config.eligibility_delay_seconds = args.eligibility_delay_seconds;
    config.min_buyback_interval_seconds = args.min_buyback_interval_seconds;
    config.paused = false;
    config.bump = ctx.bumps.config;

    emit!(ConfigInitialized {
        authority: config.authority,
        solx_mint: config.solx_mint,
        mechanism_fee_bps: config.mechanism_fee_bps,
    });
    Ok(())
}
