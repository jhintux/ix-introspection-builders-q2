use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Conversion failure")]
    ConversionFailure,
    #[msg("Initial LP amount is too less")]
    InitLpAmountTooLess,

    /// instruction exceeds desired slippage limit
    #[msg("instruction exceeds desired slippage limit")]
    ExceededSlippage,
    /// The calculation exchange rate failed.
    #[msg("CalculationExRateFailure")]
    CalculationExRateFailure,
    /// Checked_Sub Overflow
    #[msg("Checked_Sub Overflow")]
    CheckedSubOverflow,
    /// Checked_Add Overflow
    #[msg("Checked_Add Overflow")]
    CheckedAddOverflow,
    /// Checked_Mul Overflow
    #[msg("Checked_Mul Overflow")]
    CheckedMulOverflow,
    /// Checked_Div Overflow
    #[msg("Checked_Div Overflow")]
    CheckedDivOverflow,

    #[msg("Invalid base mint")]
    InvalidBaseMint,
    #[msg("Invalid quote mint")]
    InvalidQuoteMint,
    #[msg("Invalid fee")]
    InvalidFee,

    #[msg("Max amount exceeded")]
    MaxAmountExceeded,
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Pool is already locked")]
    PoolAlreadyLocked,
    #[msg("Pool is not locked")]
    PoolNotLocked,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Reserve is empty")]
    EmptyReserve,
    #[msg("LP supply is zero")]
    NoLpSupply,
    #[msg("Liquidity amount is too low")]
    LiquidityTooLow,
    #[msg("Amount is too low")]
    AmountTooLow,
    #[msg("Insufficient LP amount")]
    InsufficientLpAmount,

    #[msg("Invalid program id")]
    InvalidProgramId,
    #[msg("Invalid instruction")]
    InvalidIx,
    #[msg("Burn instruction not found")]
    BurnInstructionNotFound,
}