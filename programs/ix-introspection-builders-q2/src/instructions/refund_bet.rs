use crate::{errors::DiceError, state::Bet};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct RefundBet<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    /// CHECK: Safe
    pub house: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"vault", house.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        has_one = player,
        close = player,
        seeds = [b"bet", vault.key().as_ref(), player.key().as_ref(), bet.seed.to_le_bytes().as_ref()],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,
    pub system_program: Program<'info, System>,
}

impl<'info> RefundBet<'info> {
    pub fn refund(&mut self, bumps: &RefundBetBumps) -> Result<()> {
        let slot = Clock::get()?.slot;
        msg!("Slot: {}", slot);
        msg!("Bet slot: {}", self.bet.slot);
        let elapsed_slots = slot.checked_sub(self.bet.slot).ok_or(DiceError::Overflow)?;
        require!(elapsed_slots > 1000, DiceError::TimeoutNotReached);

        let accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.player.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[b"vault", &self.house.key().to_bytes(), &[bumps.vault]]];

        transfer(
            CpiContext::new_with_signer(self.system_program.to_account_info(), accounts, signer_seeds),
            self.bet.amount,
        )
    }
}
