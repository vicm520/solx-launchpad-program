use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, TransferChecked};

use crate::{
    constants::{
        BASE_VAULT_SEED, BURN_VAULT_SEED, CONFIG_SEED, MARKET_SEED, QUOTE_VAULT_SEED,
        REWARD_VAULT_SEED, SOL_BURN_VAULT_SEED, SOL_REWARD_VAULT_SEED, SOL_VAULT_SEED,
    },
    error::LaunchpadError,
    events::TradeExecuted,
    math::{cap_buy_quote, quote_buy, quote_sell, split_fee},
    state::{GlobalConfig, LaunchMarket, MarketStatus, QuoteMode},
};

#[derive(Accounts)]
pub struct TradeSol<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, GlobalConfig>>,
    #[account(
        mut,
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = base_mint,
        has_one = base_vault,
    )]
    pub market: Box<Account<'info, LaunchMarket>>,
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [BASE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub base_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = base_mint, token::authority = trader)]
    pub trader_base_token: Box<Account<'info, TokenAccount>>,
    /// CHECK: A system-owned PDA used only to custody this market's net SOL liquidity.
    #[account(mut, seeds = [SOL_VAULT_SEED, base_mint.key().as_ref()], bump = market.sol_vault_bump)]
    pub sol_vault: UncheckedAccount<'info>,
    /// CHECK: A system-owned PDA used only to custody this market's SOL buyback allocation.
    #[account(mut, seeds = [SOL_BURN_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub sol_burn_vault: UncheckedAccount<'info>,
    /// CHECK: A system-owned PDA used only to custody this market's SOL reward allocation.
    #[account(mut, seeds = [SOL_REWARD_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub sol_reward_vault: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TradeSolx<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, GlobalConfig>>,
    #[account(
        mut,
        seeds = [MARKET_SEED, base_mint.key().as_ref()],
        bump = market.bump,
        has_one = base_mint,
        has_one = base_vault,
        has_one = quote_vault,
        has_one = burn_vault,
        has_one = reward_vault,
    )]
    pub market: Box<Account<'info, LaunchMarket>>,
    pub base_mint: Box<Account<'info, Mint>>,
    #[account(address = config.solx_mint @ LaunchpadError::InvalidTokenAccount)]
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [BASE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub base_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [QUOTE_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub quote_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [BURN_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub burn_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [REWARD_VAULT_SEED, base_mint.key().as_ref()], bump)]
    pub reward_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = base_mint, token::authority = trader)]
    pub trader_base_token: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, token::authority = trader)]
    pub trader_quote_token: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

fn validate_trade(market: &LaunchMarket, amount: u64, mode: QuoteMode) -> Result<()> {
    require!(amount > 0, LaunchpadError::InvalidTradeAmount);
    require!(
        market.status == MarketStatus::Trading,
        LaunchpadError::InvalidMarketStatus
    );
    require!(market.quote_mode == mode, LaunchpadError::InvalidQuoteMode);
    Ok(())
}

fn record_buy(
    market: &mut LaunchMarket,
    quote_in: u64,
    net_quote: u64,
    token_out: u64,
) -> Result<()> {
    market.virtual_quote_reserve = market
        .virtual_quote_reserve
        .checked_add(net_quote)
        .ok_or(LaunchpadError::MathOverflow)?;
    market.virtual_token_reserve = market
        .virtual_token_reserve
        .checked_sub(token_out)
        .ok_or(LaunchpadError::InsufficientLiquidity)?;
    market.real_quote_reserve = market
        .real_quote_reserve
        .checked_add(net_quote)
        .ok_or(LaunchpadError::MathOverflow)?;
    market.real_token_reserve = market
        .real_token_reserve
        .checked_sub(token_out)
        .ok_or(LaunchpadError::InsufficientLiquidity)?;
    market.cumulative_quote_volume = market
        .cumulative_quote_volume
        .checked_add(quote_in)
        .ok_or(LaunchpadError::MathOverflow)?;
    market.cumulative_mechanism_fees = market
        .cumulative_mechanism_fees
        .checked_add(
            quote_in
                .checked_sub(net_quote)
                .ok_or(LaunchpadError::MathOverflow)?,
        )
        .ok_or(LaunchpadError::MathOverflow)?;
    if market.real_quote_reserve >= market.graduation_threshold {
        market.status = MarketStatus::Graduating;
    }
    Ok(())
}

fn record_sell(market: &mut LaunchMarket, token_in: u64, gross_quote: u64) -> Result<()> {
    market.virtual_token_reserve = market
        .virtual_token_reserve
        .checked_add(token_in)
        .ok_or(LaunchpadError::MathOverflow)?;
    market.virtual_quote_reserve = market
        .virtual_quote_reserve
        .checked_sub(gross_quote)
        .ok_or(LaunchpadError::InsufficientLiquidity)?;
    market.real_token_reserve = market
        .real_token_reserve
        .checked_add(token_in)
        .ok_or(LaunchpadError::MathOverflow)?;
    market.real_quote_reserve = market
        .real_quote_reserve
        .checked_sub(gross_quote)
        .ok_or(LaunchpadError::InsufficientLiquidity)?;
    market.cumulative_quote_volume = market
        .cumulative_quote_volume
        .checked_add(gross_quote)
        .ok_or(LaunchpadError::MathOverflow)?;
    Ok(())
}

pub fn handle_buy_sol(ctx: Context<TradeSol>, quote_in: u64, min_token_out: u64) -> Result<()> {
    validate_trade(&ctx.accounts.market, quote_in, QuoteMode::Sol)?;
    let remaining_quote = ctx
        .accounts
        .market
        .graduation_threshold
        .checked_sub(ctx.accounts.market.real_quote_reserve)
        .ok_or(LaunchpadError::MathOverflow)?;
    let capped = cap_buy_quote(
        quote_in,
        remaining_quote,
        ctx.accounts.config.mechanism_fee_bps,
        ctx.accounts.config.burn_bps,
    )?;
    let token_out = quote_buy(
        ctx.accounts.market.virtual_token_reserve,
        ctx.accounts.market.virtual_quote_reserve,
        capped.net_quote,
    )?;
    require!(token_out >= min_token_out, LaunchpadError::SlippageExceeded);
    require!(
        token_out <= ctx.accounts.market.real_token_reserve,
        LaunchpadError::InsufficientLiquidity
    );

    for (destination, amount) in [
        (ctx.accounts.sol_vault.to_account_info(), capped.net_quote),
        (
            ctx.accounts.sol_burn_vault.to_account_info(),
            capped.burn_fee,
        ),
        (
            ctx.accounts.sol_reward_vault.to_account_info(),
            capped.reward_fee,
        ),
    ] {
        if amount > 0 {
            system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.key(),
                    system_program::Transfer {
                        from: ctx.accounts.trader.to_account_info(),
                        to: destination,
                    },
                ),
                amount,
            )?;
        }
    }

    let base_key = ctx.accounts.base_mint.key();
    let bump = [ctx.accounts.market.bump];
    let signer: &[&[u8]] = &[MARKET_SEED, base_key.as_ref(), &bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.base_vault.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.trader_base_token.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[signer],
        ),
        token_out,
        ctx.accounts.base_mint.decimals,
    )?;

    record_buy(
        &mut ctx.accounts.market,
        capped.accepted_quote,
        capped.net_quote,
        token_out,
    )?;
    emit_trade(
        &ctx.accounts.market,
        ctx.accounts.trader.key(),
        true,
        capped.accepted_quote,
        token_out,
        capped.burn_fee,
        capped.reward_fee,
        capped.requested_quote,
        capped.refunded_quote,
    );
    Ok(())
}

pub fn handle_sell_sol(ctx: Context<TradeSol>, token_in: u64, min_quote_out: u64) -> Result<()> {
    validate_trade(&ctx.accounts.market, token_in, QuoteMode::Sol)?;
    let gross_quote = quote_sell(
        ctx.accounts.market.virtual_token_reserve,
        ctx.accounts.market.virtual_quote_reserve,
        token_in,
    )?;
    require!(
        gross_quote <= ctx.accounts.market.real_quote_reserve,
        LaunchpadError::InsufficientLiquidity
    );
    let (net_quote, burn_fee, reward_fee) = split_fee(
        gross_quote,
        ctx.accounts.config.mechanism_fee_bps,
        ctx.accounts.config.burn_bps,
    )?;
    require!(net_quote >= min_quote_out, LaunchpadError::SlippageExceeded);

    token::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.trader_base_token.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.trader.to_account_info(),
            },
        ),
        token_in,
        ctx.accounts.base_mint.decimals,
    )?;

    let base_key = ctx.accounts.base_mint.key();
    let bump = [ctx.accounts.market.sol_vault_bump];
    let signer: &[&[u8]] = &[SOL_VAULT_SEED, base_key.as_ref(), &bump];
    for (destination, amount) in [
        (ctx.accounts.trader.to_account_info(), net_quote),
        (ctx.accounts.sol_burn_vault.to_account_info(), burn_fee),
        (ctx.accounts.sol_reward_vault.to_account_info(), reward_fee),
    ] {
        if amount > 0 {
            system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.key(),
                    system_program::Transfer {
                        from: ctx.accounts.sol_vault.to_account_info(),
                        to: destination,
                    },
                    &[signer],
                ),
                amount,
            )?;
        }
    }

    record_sell(&mut ctx.accounts.market, token_in, gross_quote)?;
    ctx.accounts.market.cumulative_mechanism_fees = ctx
        .accounts
        .market
        .cumulative_mechanism_fees
        .checked_add(
            burn_fee
                .checked_add(reward_fee)
                .ok_or(LaunchpadError::MathOverflow)?,
        )
        .ok_or(LaunchpadError::MathOverflow)?;
    emit_trade(
        &ctx.accounts.market,
        ctx.accounts.trader.key(),
        false,
        gross_quote,
        token_in,
        burn_fee,
        reward_fee,
        gross_quote,
        0,
    );
    Ok(())
}

pub fn handle_buy_solx(ctx: Context<TradeSolx>, quote_in: u64, min_token_out: u64) -> Result<()> {
    validate_trade(&ctx.accounts.market, quote_in, QuoteMode::Solx)?;
    let remaining_quote = ctx
        .accounts
        .market
        .graduation_threshold
        .checked_sub(ctx.accounts.market.real_quote_reserve)
        .ok_or(LaunchpadError::MathOverflow)?;
    let capped = cap_buy_quote(
        quote_in,
        remaining_quote,
        ctx.accounts.config.mechanism_fee_bps,
        ctx.accounts.config.burn_bps,
    )?;
    let token_out = quote_buy(
        ctx.accounts.market.virtual_token_reserve,
        ctx.accounts.market.virtual_quote_reserve,
        capped.net_quote,
    )?;
    require!(token_out >= min_token_out, LaunchpadError::SlippageExceeded);
    require!(
        token_out <= ctx.accounts.market.real_token_reserve,
        LaunchpadError::InsufficientLiquidity
    );

    for (destination, amount) in [
        (ctx.accounts.quote_vault.to_account_info(), capped.net_quote),
        (
            ctx.accounts.reward_vault.to_account_info(),
            capped.reward_fee,
        ),
    ] {
        if amount > 0 {
            token::transfer_checked(
                CpiContext::new(
                    ctx.accounts.token_program.key(),
                    TransferChecked {
                        from: ctx.accounts.trader_quote_token.to_account_info(),
                        mint: ctx.accounts.quote_mint.to_account_info(),
                        to: destination,
                        authority: ctx.accounts.trader.to_account_info(),
                    },
                ),
                amount,
                ctx.accounts.quote_mint.decimals,
            )?;
        }
    }

    if capped.burn_fee > 0 {
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Burn {
                    mint: ctx.accounts.quote_mint.to_account_info(),
                    from: ctx.accounts.trader_quote_token.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            capped.burn_fee,
        )?;
    }

    transfer_base_to_trader(&ctx, token_out)?;
    record_buy(
        &mut ctx.accounts.market,
        capped.accepted_quote,
        capped.net_quote,
        token_out,
    )?;
    emit_trade(
        &ctx.accounts.market,
        ctx.accounts.trader.key(),
        true,
        capped.accepted_quote,
        token_out,
        capped.burn_fee,
        capped.reward_fee,
        capped.requested_quote,
        capped.refunded_quote,
    );
    Ok(())
}

pub fn handle_sell_solx(ctx: Context<TradeSolx>, token_in: u64, min_quote_out: u64) -> Result<()> {
    validate_trade(&ctx.accounts.market, token_in, QuoteMode::Solx)?;
    let gross_quote = quote_sell(
        ctx.accounts.market.virtual_token_reserve,
        ctx.accounts.market.virtual_quote_reserve,
        token_in,
    )?;
    require!(
        gross_quote <= ctx.accounts.market.real_quote_reserve,
        LaunchpadError::InsufficientLiquidity
    );
    let (net_quote, burn_fee, reward_fee) = split_fee(
        gross_quote,
        ctx.accounts.config.mechanism_fee_bps,
        ctx.accounts.config.burn_bps,
    )?;
    require!(net_quote >= min_quote_out, LaunchpadError::SlippageExceeded);

    token::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.trader_base_token.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.trader.to_account_info(),
            },
        ),
        token_in,
        ctx.accounts.base_mint.decimals,
    )?;

    let base_key = ctx.accounts.base_mint.key();
    let bump = [ctx.accounts.market.bump];
    let signer: &[&[u8]] = &[MARKET_SEED, base_key.as_ref(), &bump];
    for (source, destination, amount) in [
        (
            ctx.accounts.quote_vault.to_account_info(),
            ctx.accounts.trader_quote_token.to_account_info(),
            net_quote,
        ),
        (
            ctx.accounts.quote_vault.to_account_info(),
            ctx.accounts.reward_vault.to_account_info(),
            reward_fee,
        ),
    ] {
        if amount > 0 {
            token::transfer_checked(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.key(),
                    TransferChecked {
                        from: source,
                        mint: ctx.accounts.quote_mint.to_account_info(),
                        to: destination,
                        authority: ctx.accounts.market.to_account_info(),
                    },
                    &[signer],
                ),
                amount,
                ctx.accounts.quote_mint.decimals,
            )?;
        }
    }

    if burn_fee > 0 {
        token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Burn {
                    mint: ctx.accounts.quote_mint.to_account_info(),
                    from: ctx.accounts.quote_vault.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                &[signer],
            ),
            burn_fee,
        )?;
    }

    record_sell(&mut ctx.accounts.market, token_in, gross_quote)?;
    ctx.accounts.market.cumulative_mechanism_fees = ctx
        .accounts
        .market
        .cumulative_mechanism_fees
        .checked_add(
            burn_fee
                .checked_add(reward_fee)
                .ok_or(LaunchpadError::MathOverflow)?,
        )
        .ok_or(LaunchpadError::MathOverflow)?;
    emit_trade(
        &ctx.accounts.market,
        ctx.accounts.trader.key(),
        false,
        gross_quote,
        token_in,
        burn_fee,
        reward_fee,
        gross_quote,
        0,
    );
    Ok(())
}

fn transfer_base_to_trader(ctx: &Context<TradeSolx>, amount: u64) -> Result<()> {
    let base_key = ctx.accounts.base_mint.key();
    let bump = [ctx.accounts.market.bump];
    let signer: &[&[u8]] = &[MARKET_SEED, base_key.as_ref(), &bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.base_vault.to_account_info(),
                mint: ctx.accounts.base_mint.to_account_info(),
                to: ctx.accounts.trader_base_token.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[signer],
        ),
        amount,
        ctx.accounts.base_mint.decimals,
    )
}

fn emit_trade(
    market: &LaunchMarket,
    trader: Pubkey,
    buy: bool,
    quote_amount: u64,
    token_amount: u64,
    burn_fee: u64,
    reward_fee: u64,
    requested_quote_amount: u64,
    refunded_quote_amount: u64,
) {
    emit!(TradeExecuted {
        market: market.base_mint,
        trader,
        buy,
        quote_mode: market.quote_mode,
        quote_amount,
        requested_quote_amount,
        refunded_quote_amount,
        token_amount,
        burn_fee,
        reward_fee,
        real_token_reserve: market.real_token_reserve,
        real_quote_reserve: market.real_quote_reserve,
    });
}
