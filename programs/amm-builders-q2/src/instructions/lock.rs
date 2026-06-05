use anchor_lang::prelude::*;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Lock<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct Unlock<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
}

impl<'info> Lock<'info> {
    pub fn lock(&mut self) -> Result<()> {
        require!(
            self.config.authority == Some(self.authority.key()),
            AmmError::Unauthorized
        );
        require!(!self.config.locked, AmmError::PoolAlreadyLocked);
        self.config.locked = true;
        Ok(())
    }
}

impl<'info> Unlock<'info> {
    pub fn unlock(&mut self) -> Result<()> {
        require!(
            self.config.authority == Some(self.authority.key()),
            AmmError::Unauthorized
        );
        require!(self.config.locked, AmmError::PoolNotLocked);
        self.config.locked = false;
        Ok(())
    }
}
