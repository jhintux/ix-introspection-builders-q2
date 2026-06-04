use anchor_lang::prelude::*;

#[error_code]
pub enum DiceError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid roll must be between 1 and 99")]
    InvalidRoll,
    #[msg("Timeout not reached")]
    TimeoutNotReached,
    #[msg("Overflow")]
    Overflow,

    #[msg("Invalid program id")]
    InvalidProgramId,
    #[msg("Invalid signer")]
    InvalidSigner,
    #[msg("Invalid data, expected commit:commitment")]
    InvalidData,
    #[msg("Not enough signers")]
    NotEnoughSigners,
    #[msg("Invalid commitment")]
    InvalidCommitment,
}