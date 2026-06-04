use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::{state::{Bet, MIN_BET_LAMPORTS, MIN_ROLL, MAX_ROLL}, errors::DiceError};

#[derive(Accounts)]
#[instruction(seed: u128)]
pub struct PlaceBet<'info> {
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
        init,
        payer = player,
        space = Bet::DISCRIMINATOR.len() + Bet::INIT_SPACE,
        seeds = [b"bet", vault.key().as_ref(), player.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    pub system_program: Program<'info, System>,
}

impl<'info> PlaceBet<'info> {
    pub fn create_bet(
        &mut self,
        seed: u128,
        roll: u8,
        amount: u64,
        bumps: &PlaceBetBumps,
    ) -> Result<()> {
        require!(amount >= MIN_BET_LAMPORTS, DiceError::InvalidAmount);
        require!(roll >= MIN_ROLL && roll <= MAX_ROLL, DiceError::InvalidRoll);
        let slot = Clock::get()?.slot;

        let commitment = [0u8; 32];
        
        self.bet.set_inner(Bet {
            slot,
            player: self.player.key(),
            seed,
            amount,
            roll,
            bump: bumps.bet,
            commitment,
        });

        Ok(())
    }

    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let accounts = Transfer {
            from: self.player.to_account_info(),
            to: self.vault.to_account_info(),
        };

        transfer(
            CpiContext::new(self.system_program.to_account_info(), accounts),
            amount,
        )
    }
}
