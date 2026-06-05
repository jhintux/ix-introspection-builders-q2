use uint::construct_uint;

use crate::errors::AmmError;

construct_uint! {
    pub struct U128(2);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Calculator {}

impl Calculator {
    pub fn to_u128(val: u64) -> Result<U128, AmmError> {
        val.try_into().map_err(|_| AmmError::ConversionFailure)
    }

    pub fn to_u64(val: u128) -> Result<u64, AmmError> {
        val.try_into().map_err(|_| AmmError::ConversionFailure)
    }

    pub fn proportional_amount(
        amount: u64,
        reserve_in: u64,
        reserve_out: u64,
    ) -> Result<u64, AmmError> {
        Calculator::to_u64(
            U128::from(amount)
                .checked_mul(U128::from(reserve_out))
                .ok_or(AmmError::CheckedMulOverflow)?
                .checked_div(U128::from(reserve_in))
                .ok_or(AmmError::CheckedDivOverflow)?
                .as_u128(),
        )
    }

    pub fn liquidity_from_amount(
        amount: u64,
        reserve: u64,
        lp_supply: u64,
    ) -> Result<u64, AmmError> {
        Calculator::proportional_amount(amount, reserve, lp_supply)
    }

    pub fn amount_from_liquidity(
        lp_amount: u64,
        reserve: u64,
        lp_supply: u64,
    ) -> Result<u64, AmmError> {
        Calculator::proportional_amount(lp_amount, lp_supply, reserve)
    }

    pub fn swap_output(
        amount_in: u64,
        fee: u16,
        reserve_in: u64,
        reserve_out: u64,
    ) -> Result<u64, AmmError> {
        let amount_in_after_fee = U128::from(amount_in)
            .checked_mul(U128::from(10_000u16 - fee))
            .ok_or(AmmError::CheckedMulOverflow)?
            .checked_div(U128::from(10_000))
            .ok_or(AmmError::CheckedDivOverflow)?;

        Calculator::to_u64(
            U128::from(reserve_out)
                .checked_mul(amount_in_after_fee)
                .ok_or(AmmError::CheckedMulOverflow)?
                .checked_div(
                    U128::from(reserve_in)
                        .checked_add(amount_in_after_fee)
                        .ok_or(AmmError::CheckedAddOverflow)?,
                )
                .ok_or(AmmError::CheckedDivOverflow)?
                .as_u128(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Calculator;

    #[test]
    fn swap_output_zero_fee_matches_constant_product() {
        // reserves 1000/1000, swap 100 base in -> out = 1000 * 100 / (1000 + 100) = 90
        let out = Calculator::swap_output(100, 0, 1000, 1000).unwrap();
        assert_eq!(out, 90);
    }

    #[test]
    fn swap_output_applies_fee() {
        // 1% fee: effective in = 99, out = floor(1000 * 99 / (1000 + 99)) = 90
        let out = Calculator::swap_output(100, 100, 1000, 1000).unwrap();
        assert_eq!(out, 90);
    }

    #[test]
    fn swap_output_with_zero_reserve_in() {
        // Instruction layer must reject empty reserves; math uses amount_in as denominator base.
        let out = Calculator::swap_output(100, 0, 0, 1000).unwrap();
        assert_eq!(out, 1000);
    }
}
