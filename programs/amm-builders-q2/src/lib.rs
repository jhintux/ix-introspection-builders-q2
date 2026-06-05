use anchor_lang::prelude::*;

mod errors;
mod instructions;
pub mod math;
mod state;

use instructions::*;
use state::{FixedSide, Direction};

declare_id!("3r8zwrn1haZMdp83YYaCfhBVnQUpjStYqRCn6KiXJgN9");

#[program]
pub mod amm_builders_q2 {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        init_base_amount: u64,
        init_quote_amount: u64,
        fee: u16,
    ) -> Result<()> {
        ctx.accounts
            .initialize(seed, init_base_amount, init_quote_amount, fee, &ctx.bumps)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        fixed_side: FixedSide,
        amount_in: u64,
        max_amount: u64,
        min_lp_amount: u64,
    ) -> Result<()> {
        ctx.accounts
            .deposit(fixed_side, amount_in, max_amount, min_lp_amount)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_base_amount: u64,
        min_quote_amount: u64,
    ) -> Result<()> {
        ctx.accounts
            .withdraw(lp_amount, min_base_amount, min_quote_amount)
    }

    pub fn swap(
        ctx: Context<Swap>,
        direction: Direction,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        ctx.accounts.swap(direction, amount_in, min_amount_out)
    }

    pub fn lock(ctx: Context<Lock>) -> Result<()> {
        ctx.accounts.lock()
    }

    pub fn unlock(ctx: Context<Unlock>) -> Result<()> {
        ctx.accounts.unlock()
    }

    pub fn burn(ctx: Context<Burn>, lp_amount: u64) -> Result<()> {
        ctx.accounts.burn(lp_amount)
    }

    pub fn payout(ctx: Context<Payout>, min_base_amount: u64, min_quote_amount: u64) -> Result<()> {
        ctx.accounts.payout(min_base_amount, min_quote_amount)
    }
}
