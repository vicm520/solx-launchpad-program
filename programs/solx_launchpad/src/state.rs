use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub solx_mint: Pubkey,
    pub graduation_program: Pubkey,
    pub mechanism_fee_bps: u16,
    pub burn_bps: u16,
    pub reward_bps: u16,
    pub curve_allocation_bps: u16,
    pub liquidity_allocation_bps: u16,
    pub ecosystem_allocation_bps: u16,
    pub equity_unit_tokens: u64,
    pub eligibility_delay_seconds: u32,
    pub min_buyback_interval_seconds: u32,
    pub paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct LaunchMarket {
    pub config: Pubkey,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub burn_vault: Pubkey,
    pub reward_vault: Pubkey,
    pub quote_mode: QuoteMode,
    pub status: MarketStatus,
    pub total_supply: u64,
    pub curve_token_amount: u64,
    pub liquidity_token_amount: u64,
    pub ecosystem_token_amount: u64,
    pub graduation_threshold: u64,
    pub virtual_token_reserve: u64,
    pub virtual_quote_reserve: u64,
    pub real_token_reserve: u64,
    pub real_quote_reserve: u64,
    pub cumulative_quote_volume: u64,
    pub cumulative_mechanism_fees: u64,
    pub external_pool: Pubkey,
    pub created_at: i64,
    pub graduated_at: i64,
    pub bump: u8,
    pub sol_vault_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PlatformVerification {
    pub mint: Pubkey,
    pub reviewer: Pubkey,
    pub verified: bool,
    pub reason_hash: [u8; 32],
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum QuoteMode {
    Sol,
    Solx,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum DexKind {
    Raydium,
    Meteora,
    Orca,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum MarketStatus {
    Trading,
    Paused,
    Graduating,
    Graduated,
}
