use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, BurnChecked, Mint, TokenAccount, TokenInterface};

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    #[account(address = config.base_mint @ AmmError::InvalidBaseMint)]
    pub base_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(address = config.quote_mint @ AmmError::InvalidQuoteMint)]
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"lp_mint", config.seed.to_le_bytes().as_ref()],
        bump = config.lp_bump,
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub base_mint_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub quote_mint_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub user_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Burn<'info> {
    pub fn burn(&mut self, lp_amount: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(lp_amount > 0, AmmError::AmountTooLow);

        let lp_supply = self.lp_mint.supply;
        require!(lp_amount <= lp_supply, AmmError::MaxAmountExceeded);
        require!(lp_supply > 0, AmmError::NoLpSupply);

        require!(
            self.user_lp_ata.amount >= lp_amount,
            AmmError::InsufficientLpAmount
        );
        token_interface::burn_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                BurnChecked {
                    mint: self.lp_mint.to_account_info(),
                    from: self.user_lp_ata.to_account_info(),
                    authority: self.payer.to_account_info(),
                },
            ),
            lp_amount,
            self.lp_mint.decimals,
        )
    }
}
