use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    errors::AmmError,
    math::Calculator,
    state::{Config, Direction},
};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
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
        associated_token::mint = base_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub user_base_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub user_quote_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Swap<'info> {
    pub fn swap(
        &mut self,
        direction: Direction,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        require!(amount_in > 0, AmmError::AmountTooLow);
        require!(!self.config.locked, AmmError::PoolLocked);

        let base_reserve = self.base_mint_vault.amount;
        let quote_reserve = self.quote_mint_vault.amount;
        require!(
            base_reserve > 0 && quote_reserve > 0,
            AmmError::EmptyReserve
        );

        let (reserve_in, reserve_out) = match direction {
            Direction::BaseToQuote => (base_reserve, quote_reserve),
            Direction::QuoteToBase => (quote_reserve, base_reserve),
        };

        let amount_out = Calculator::swap_output(
            amount_in,
            self.config.fee,
            reserve_in,
            reserve_out,
        )?;

        require!(amount_out > 0, AmmError::AmountTooLow);
        require!(amount_out <= reserve_out, AmmError::MaxAmountExceeded);
        require!(amount_out >= min_amount_out, AmmError::ExceededSlippage);

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        match direction {
            Direction::BaseToQuote => {
                token_interface::transfer_checked(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        TransferChecked {
                            mint: self.base_mint.to_account_info(),
                            from: self.user_base_ata.to_account_info(),
                            to: self.base_mint_vault.to_account_info(),
                            authority: self.payer.to_account_info(),
                        },
                    ),
                    amount_in,
                    self.base_mint.decimals,
                )?;

                token_interface::transfer_checked(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        TransferChecked {
                            mint: self.quote_mint.to_account_info(),
                            from: self.quote_mint_vault.to_account_info(),
                            to: self.user_quote_ata.to_account_info(),
                            authority: self.config.to_account_info(),
                        },
                    )
                    .with_signer(signer_seeds),
                    amount_out,
                    self.quote_mint.decimals,
                )?;
            }
            Direction::QuoteToBase => {
                token_interface::transfer_checked(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        TransferChecked {
                            mint: self.quote_mint.to_account_info(),
                            from: self.user_quote_ata.to_account_info(),
                            to: self.quote_mint_vault.to_account_info(),
                            authority: self.payer.to_account_info(),
                        },
                    ),
                    amount_in,
                    self.quote_mint.decimals,
                )?;

                token_interface::transfer_checked(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        TransferChecked {
                            mint: self.base_mint.to_account_info(),
                            from: self.base_mint_vault.to_account_info(),
                            to: self.user_base_ata.to_account_info(),
                            authority: self.config.to_account_info(),
                        },
                    )
                    .with_signer(signer_seeds),
                    amount_out,
                    self.base_mint.decimals,
                )?;
            }
        }

        Ok(())
    }
}
