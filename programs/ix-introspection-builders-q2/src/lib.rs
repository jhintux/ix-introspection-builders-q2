use anchor_lang::prelude::*;

mod instructions;
mod state;
mod errors;
mod constants;

use instructions::*;

declare_id!("HxE2gTVhrzJmvTUC8xSfxtEKjEjxCZUj5A9srNxVtqVr");

#[program]
pub mod ix_introspection_builders_q2 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, amount: u64) -> Result<()> {
        ctx.accounts.initialize(amount)
    }

   pub fn place_bet(ctx: Context<PlaceBet>, seed: u128, roll: u8, amount: u64) -> Result<()> {
        ctx.accounts.create_bet(seed, roll, amount, &ctx.bumps)?;
        ctx.accounts.deposit(amount)
    }

    pub fn place_bet_v2(ctx: Context<PlaceBetV2>, seed: u128, roll: u8, amount: u64) -> Result<()> {
        ctx.accounts.create_bet_v2(seed, roll, amount, &ctx.bumps)?;
        ctx.accounts.deposit(amount)
    }

    pub fn refund_bet(ctx: Context<RefundBet>) -> Result<()> {
        ctx.accounts.refund(&ctx.bumps)
    }

    pub fn resolve_bet(ctx: Context<ResolveBet>) -> Result<()> {
        ctx.accounts.resolve_bet(&ctx.bumps)
    }
}
