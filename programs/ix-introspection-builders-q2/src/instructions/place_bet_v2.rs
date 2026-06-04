use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        sysvar::instructions::{self, load_current_index_checked, load_instruction_at_checked},
    },
    system_program::{transfer, Transfer},
};
use solana_program::hash::hash;

use crate::{
    constants::MEMO_PROGRAM_ID,
    errors::DiceError,
    state::{Bet, MAX_ROLL, MIN_BET_LAMPORTS, MIN_ROLL},
};

#[derive(Accounts)]
#[instruction(seed: u128)]
pub struct PlaceBetV2<'info> {
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
    /// CHECK: sysvar program
    #[account(address = instructions::ID)]
    pub instruction_sysvar: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

fn check_commitment(ix: &Instruction, signers: &[Pubkey]) -> Result<[u8; 32]> {
    require!(ix.program_id == MEMO_PROGRAM_ID,DiceError::InvalidProgramId);
    require!(ix.accounts.len() == signers.len(), DiceError::NotEnoughSigners);
    require_keys_eq!(ix.accounts[0].pubkey, signers[0], DiceError::InvalidSigner);
    require_keys_eq!(ix.accounts[1].pubkey, signers[1], DiceError::InvalidSigner);

    let mut combined_data = Vec::with_capacity(64);

    let data = String::from_utf8(ix.data.to_vec()).map_err(|_| DiceError::InvalidData)?;
    data.split(":").for_each(|part| {
        combined_data.extend_from_slice(part.as_bytes());
    });

    Ok(hash(&combined_data).to_bytes())
}

impl<'info> PlaceBetV2<'info> {
    pub fn create_bet_v2(
        &mut self,
        seed: u128,
        roll: u8,
        amount: u64,
        bumps: &PlaceBetV2Bumps,
    ) -> Result<()> {
        require!(amount >= MIN_BET_LAMPORTS, DiceError::InvalidAmount);
        require!(roll >= MIN_ROLL && roll <= MAX_ROLL, DiceError::InvalidRoll);
        let slot = Clock::get()?.slot;

        let current_index = load_current_index_checked(&self.instruction_sysvar.to_account_info())?;
        let combined_commitment_ix = load_instruction_at_checked(
            (current_index as usize).checked_sub(1).ok_or(DiceError::Overflow)?,
            &self.instruction_sysvar.to_account_info(),
        )?;

        let commitment = check_commitment(&combined_commitment_ix, &[self.house.key(), self.player.key()])?;

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
