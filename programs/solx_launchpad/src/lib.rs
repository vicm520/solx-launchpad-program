pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use anchor_lang::prelude::*;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

pub use instructions::*;
use state::DexKind;

declare_id!("2AVaQ45R5u1DE4JyvtQ76fUDm72CLSXMRfBQ8d3TPHjA");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "SOLX Launchpad",
    project_url: "https://github.com/vicm520/solx-launchpad-program",
    contacts: "link:https://github.com/vicm520/solx-launchpad-program/security/advisories/new",
    policy: "https://github.com/vicm520/solx-launchpad-program/security/policy",
    preferred_languages: "zh,en",
    source_code: "https://github.com/vicm520/solx-launchpad-program",
    source_release: "v0.1.0-devnet",
    auditors: "None"
}

#[program]
pub mod solx_launchpad {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        args: InitializeConfigArgs,
    ) -> Result<()> {
        instructions::initialize_config::handle_initialize_config(ctx, args)
    }

    pub fn create_market(ctx: Context<CreateMarket>, args: CreateMarketArgs) -> Result<()> {
        instructions::create_market::handle_create_market(ctx, args)
    }

    pub fn buy_sol(ctx: Context<TradeSol>, quote_in: u64, min_token_out: u64) -> Result<()> {
        instructions::trade::handle_buy_sol(ctx, quote_in, min_token_out)
    }

    pub fn sell_sol(ctx: Context<TradeSol>, token_in: u64, min_quote_out: u64) -> Result<()> {
        instructions::trade::handle_sell_sol(ctx, token_in, min_quote_out)
    }

    pub fn buy_solx(ctx: Context<TradeSolx>, quote_in: u64, min_token_out: u64) -> Result<()> {
        instructions::trade::handle_buy_solx(ctx, quote_in, min_token_out)
    }

    pub fn sell_solx(ctx: Context<TradeSolx>, token_in: u64, min_quote_out: u64) -> Result<()> {
        instructions::trade::handle_sell_solx(ctx, token_in, min_quote_out)
    }

    pub fn set_market_paused(ctx: Context<ManageMarket>, paused: bool) -> Result<()> {
        instructions::manage_market::handle_set_paused(ctx, paused)
    }

    pub fn begin_graduation(ctx: Context<ManageMarket>) -> Result<()> {
        instructions::manage_market::handle_begin_graduation(ctx)
    }

    pub fn finalize_graduation(
        ctx: Context<ManageMarket>,
        external_pool: Pubkey,
        dex: DexKind,
    ) -> Result<()> {
        instructions::manage_market::handle_finalize_graduation(ctx, external_pool, dex)
    }

    pub fn release_migration_assets_sol(ctx: Context<ReleaseMigrationAssetsSol>) -> Result<()> {
        instructions::migration::handle_release_migration_assets_sol(ctx)
    }

    pub fn release_migration_assets_solx(ctx: Context<ReleaseMigrationAssetsSolx>) -> Result<()> {
        instructions::migration::handle_release_migration_assets_solx(ctx)
    }
}
