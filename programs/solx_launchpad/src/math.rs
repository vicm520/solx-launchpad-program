use anchor_lang::prelude::*;

use crate::{constants::BASIS_POINTS, error::LaunchpadError};

pub fn allocation_for(total_supply: u64, basis_points: u16) -> Result<u64> {
    let amount = (total_supply as u128)
        .checked_mul(basis_points as u128)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(BASIS_POINTS as u128)
        .ok_or(LaunchpadError::MathOverflow)?;
    u64::try_from(amount).map_err(|_| error!(LaunchpadError::MathOverflow))
}

pub fn split_fee(amount: u64, fee_bps: u16, burn_bps: u16) -> Result<(u64, u64, u64)> {
    let fee = mul_bps(amount, fee_bps)?;
    let burn = mul_bps(amount, burn_bps)?;
    let reward = fee.checked_sub(burn).ok_or(LaunchpadError::MathOverflow)?;
    let net = amount
        .checked_sub(fee)
        .ok_or(LaunchpadError::MathOverflow)?;
    Ok((net, burn, reward))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CappedBuyQuote {
    pub requested_quote: u64,
    pub accepted_quote: u64,
    pub refunded_quote: u64,
    pub net_quote: u64,
    pub burn_fee: u64,
    pub reward_fee: u64,
}

pub fn cap_buy_quote(
    requested_quote: u64,
    remaining_net_quote: u64,
    fee_bps: u16,
    burn_bps: u16,
) -> Result<CappedBuyQuote> {
    require!(requested_quote > 0, LaunchpadError::InvalidTradeAmount);
    require!(
        remaining_net_quote > 0,
        LaunchpadError::GraduationThresholdNotReached
    );

    let requested_split = split_fee(requested_quote, fee_bps, burn_bps)?;
    let accepted_quote = if requested_split.0 <= remaining_net_quote {
        requested_quote
    } else {
        let mut low = 1u64;
        let mut high = requested_quote;
        while low < high {
            let middle = low + (high - low) / 2;
            if split_fee(middle, fee_bps, burn_bps)?.0 < remaining_net_quote {
                low = middle.checked_add(1).ok_or(LaunchpadError::MathOverflow)?;
            } else {
                high = middle;
            }
        }
        low
    };
    let (net_quote, burn_fee, reward_fee) = split_fee(accepted_quote, fee_bps, burn_bps)?;
    require!(
        net_quote <= remaining_net_quote,
        LaunchpadError::MathOverflow
    );

    Ok(CappedBuyQuote {
        requested_quote,
        accepted_quote,
        refunded_quote: requested_quote
            .checked_sub(accepted_quote)
            .ok_or(LaunchpadError::MathOverflow)?,
        net_quote,
        burn_fee,
        reward_fee,
    })
}

pub fn quote_buy(token_reserve: u64, quote_reserve: u64, quote_in: u64) -> Result<u64> {
    require!(
        token_reserve > 0 && quote_reserve > 0 && quote_in > 0,
        LaunchpadError::InvalidCurveParameters
    );
    let output = (token_reserve as u128)
        .checked_mul(quote_in as u128)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(
            (quote_reserve as u128)
                .checked_add(quote_in as u128)
                .ok_or(LaunchpadError::MathOverflow)?,
        )
        .ok_or(LaunchpadError::MathOverflow)?;
    u64::try_from(output).map_err(|_| error!(LaunchpadError::MathOverflow))
}

pub fn quote_sell(token_reserve: u64, quote_reserve: u64, token_in: u64) -> Result<u64> {
    require!(
        token_reserve > 0 && quote_reserve > 0 && token_in > 0,
        LaunchpadError::InvalidCurveParameters
    );
    let output = (quote_reserve as u128)
        .checked_mul(token_in as u128)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(
            (token_reserve as u128)
                .checked_add(token_in as u128)
                .ok_or(LaunchpadError::MathOverflow)?,
        )
        .ok_or(LaunchpadError::MathOverflow)?;
    u64::try_from(output).map_err(|_| error!(LaunchpadError::MathOverflow))
}

fn mul_bps(amount: u64, bps: u16) -> Result<u64> {
    let value = (amount as u128)
        .checked_mul(bps as u128)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(BASIS_POINTS as u128)
        .ok_or(LaunchpadError::MathOverflow)?;
    u64::try_from(value).map_err(|_| error!(LaunchpadError::MathOverflow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_one_billion_with_six_decimals() {
        let supply = 1_000_000_000_000_000u64;
        assert_eq!(allocation_for(supply, 6_000).unwrap(), 600_000_000_000_000);
        assert_eq!(allocation_for(supply, 2_000).unwrap(), 200_000_000_000_000);
    }

    #[test]
    fn splits_two_percent_mechanism_fee_evenly() {
        assert_eq!(
            split_fee(1_000_000, 200, 100).unwrap(),
            (980_000, 10_000, 10_000)
        );
    }

    #[test]
    fn constant_product_quotes_round_down() {
        let bought = quote_buy(1_000_000, 100_000, 10_000).unwrap();
        assert_eq!(bought, 90_909);
        let sold = quote_sell(909_091, 110_000, bought).unwrap();
        assert!(sold <= 10_000);
    }

    #[test]
    fn caps_zero_fee_buy_at_remaining_graduation_quote() {
        let quote = cap_buy_quote(1_000_000, 200_000, 0, 0).unwrap();
        assert_eq!(quote.accepted_quote, 200_000);
        assert_eq!(quote.net_quote, 200_000);
        assert_eq!(quote.refunded_quote, 800_000);
    }

    #[test]
    fn caps_fee_buy_without_overfunding_the_market() {
        let quote = cap_buy_quote(1_000_000, 196_000, 200, 100).unwrap();
        assert_eq!(quote.accepted_quote, 199_999);
        assert_eq!(quote.net_quote, 196_000);
        assert_eq!(quote.burn_fee, 1_999);
        assert_eq!(quote.reward_fee, 2_000);
        assert_eq!(quote.refunded_quote, 800_001);
    }

    #[test]
    fn leaves_regular_buy_unchanged() {
        let quote = cap_buy_quote(100_000, 196_000, 200, 100).unwrap();
        assert_eq!(quote.accepted_quote, 100_000);
        assert_eq!(quote.net_quote, 98_000);
        assert_eq!(quote.refunded_quote, 0);
    }
}
