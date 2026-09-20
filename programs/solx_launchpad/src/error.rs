use anchor_lang::prelude::*;

#[error_code]
pub enum LaunchpadError {
    #[msg("Mechanism fee must equal burn plus reward fee")]
    InvalidFeeSplit,
    #[msg("Mechanism fee is above the protocol maximum")]
    FeeTooHigh,
    #[msg("Token allocations must add up to total supply")]
    InvalidAllocationTotal,
    #[msg("Token allocations do not match protocol basis points")]
    InvalidAllocationRatio,
    #[msg("Token allocations are outside the launchpad safety limits")]
    UnsafeAllocationRatio,
    #[msg("Virtual reserves and graduation threshold must be positive")]
    InvalidCurveParameters,
    #[msg("Only the protocol authority can perform this action")]
    Unauthorized,
    #[msg("Market status does not allow this action")]
    InvalidMarketStatus,
    #[msg("The market has not reached its graduation threshold")]
    GraduationThresholdNotReached,
    #[msg("External pool cannot be the default public key")]
    InvalidExternalPool,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Trade amount must be greater than zero")]
    InvalidTradeAmount,
    #[msg("The selected instruction does not match this market quote mode")]
    InvalidQuoteMode,
    #[msg("The trade would return less than the requested minimum")]
    SlippageExceeded,
    #[msg("The market does not have enough real reserves")]
    InsufficientLiquidity,
    #[msg("The supplied token account or mint does not belong to this market")]
    InvalidTokenAccount,
    #[msg("Migration assets have already been released")]
    MigrationAlreadyReleased,
}
