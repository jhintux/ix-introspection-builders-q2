use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, BurnChecked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{errors::AmmError, math::Calculator, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub user_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        lp_amount: u64,
        min_base_amount: u64,
        min_quote_amount: u64,
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(lp_amount > 0, AmmError::AmountTooLow);

        let base_reserve = self.base_mint_vault.amount;
        let quote_reserve = self.quote_mint_vault.amount;
        let lp_supply = self.lp_mint.supply;
        require!(lp_amount <= lp_supply, AmmError::MaxAmountExceeded);

        require!(
            base_reserve > 0 && quote_reserve > 0,
            AmmError::EmptyReserve
        );
        require!(lp_supply > 0, AmmError::NoLpSupply);

        let base_amount = Calculator::amount_from_liquidity(lp_amount, base_reserve, lp_supply)?;
        let quote_amount = Calculator::amount_from_liquidity(lp_amount, quote_reserve, lp_supply)?;

        require!(base_amount > 0 && quote_amount > 0, AmmError::AmountTooLow);
        require!(base_amount >= min_base_amount, AmmError::ExceededSlippage);
        require!(quote_amount >= min_quote_amount, AmmError::ExceededSlippage);

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
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

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
            base_amount,
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
            quote_amount,
            self.quote_mint.decimals,
        )?;

        Ok(())
    }
}
