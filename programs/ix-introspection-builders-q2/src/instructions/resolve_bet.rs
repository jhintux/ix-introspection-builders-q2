use crate::errors::DiceError;
use crate::state::HOUSE_EDGE_BASIS_POINTS;
use crate::{constants::MEMO_PROGRAM_ID, state::Bet};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::{
    self, load_current_index_checked, load_instruction_at_checked,
};
use anchor_lang::system_program::{transfer, Transfer};
use solana_program::hash::hash;
use solana_program::native_token::LAMPORTS_PER_SOL;

#[derive(Accounts)]
pub struct ResolveBet<'info> {
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
        seeds = [b"bet", vault.key().as_ref(), player.key().as_ref(), bet.seed.to_le_bytes().as_ref()],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,
    /// CHECK: sysvar program
    #[account(address = instructions::ID)]
    pub instruction_sysvar: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl<'info> ResolveBet<'info> {
    pub fn resolve_bet(&mut self, bumps: &ResolveBetBumps) -> Result<()> {
        let current_index = load_current_index_checked(&self.instruction_sysvar.to_account_info())?;
        let combined_secrets_ix = load_instruction_at_checked(
            (current_index as usize)
                .checked_sub(1)
                .ok_or(DiceError::Overflow)?,
            &self.instruction_sysvar.to_account_info(),
        )?;

        require!(
            combined_secrets_ix.program_id == MEMO_PROGRAM_ID,
            DiceError::InvalidProgramId
        );
        require!(
            combined_secrets_ix.accounts.len() == 2,
            DiceError::NotEnoughSigners
        );
        require_keys_eq!(
            combined_secrets_ix.accounts[0].pubkey,
            self.house.key(),
            DiceError::InvalidSigner
        );
        require_keys_eq!(
            combined_secrets_ix.accounts[1].pubkey,
            self.player.key(),
            DiceError::InvalidSigner
        );

        let data = String::from_utf8(combined_secrets_ix.data.to_vec())
            .map_err(|_| DiceError::InvalidData)?;
        let mut parts = data.split(":");
        let house_secret = parts.next().ok_or(DiceError::InvalidData)?;
        let user_secret = parts.next().ok_or(DiceError::InvalidData)?;

        let house_hash_hex = hex_encode(hash(house_secret.as_bytes()).as_bytes());
        let user_hash_hex = hex_encode(hash(user_secret.as_bytes()).as_bytes());

        let mut combined = Vec::with_capacity(128);
        combined.extend_from_slice(house_hash_hex.as_bytes());
        combined.extend_from_slice(user_hash_hex.as_bytes());

        let recomputed = hash(&combined).to_bytes();
        require!(
            recomputed == self.bet.commitment,
            DiceError::InvalidCommitment
        );

        let roll = (u64::from_le_bytes(recomputed[..8].try_into().unwrap())
            .checked_rem(100)
            .unwrap()) as u8
            + 1;

        if self.bet.roll > roll {
            let winning_numbers = self.bet.roll as u128 - 1;

            let payout: u64 = (self.bet.amount as u128)
                .checked_mul(10_000 - HOUSE_EDGE_BASIS_POINTS as u128)
                .ok_or(DiceError::Overflow)?
                .checked_div(winning_numbers)
                .ok_or(DiceError::Overflow)?
                .checked_div(100)
                .ok_or(DiceError::Overflow)?
                .try_into()
                .map_err(|_| DiceError::Overflow)?;

            msg!(
                "Bet summary: target < {}, resolved roll {}, WON, payout {} SOL ({} lamports)",
                self.bet.roll,
                roll,
                payout as f64 / LAMPORTS_PER_SOL as f64,
                payout,
            );

            let signed_seeds: &[&[&[u8]]] =
                &[&[b"vault", &self.house.key().to_bytes(), &[bumps.vault]]];

            let accounts = Transfer {
                from: self.vault.to_account_info(),
                to: self.player.to_account_info(),
            };

            transfer(
                CpiContext::new_with_signer(
                    self.system_program.to_account_info(),
                    accounts,
                    signed_seeds,
                ),
                payout,
            )?
        }

        Ok(())
    }
}
