use anchor_lang::prelude::*;

use crate::state::{DexKind, MarketStatus, QuoteMode};

#[event]
pub struct ConfigInitialized {
    pub authority: Pubkey,
    pub solx_mint: Pubkey,
    pub mechanism_fee_bps: u16,
}

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mode: QuoteMode,
    pub target_dex: DexKind,
    pub total_supply: u64,
    pub graduation_threshold: u64,
}

#[event]
pub struct MarketStatusChanged {
    pub market: Pubkey,
    pub previous_status: MarketStatus,
    pub status: MarketStatus,
    pub external_pool: Pubkey,
}

#[event]
pub struct TradeExecuted {
    pub market: Pubkey,
    pub trader: Pubkey,
    pub buy: bool,
    pub quote_mode: QuoteMode,
    pub quote_amount: u64,
    pub requested_quote_amount: u64,
    pub refunded_quote_amount: u64,
    pub token_amount: u64,
    pub burn_fee: u64,
    pub reward_fee: u64,
    pub real_token_reserve: u64,
    pub real_quote_reserve: u64,
}

#[event]
pub struct GraduationFinalized {
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub dex: DexKind,
    pub external_pool: Pubkey,
    pub graduated_at: i64,
}

#[event]
pub struct MigrationAssetsReleased {
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub token_amount: u64,
    pub quote_amount: u64,
    pub quote_mode: QuoteMode,
}

#[event]
pub struct PlatformVerificationUpdated {
    pub mint: Pubkey,
    pub reviewer: Pubkey,
    pub verified: bool,
    pub reason_hash: [u8; 32],
    pub updated_at: i64,
}
