use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::TransferChecked,
    token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface},
};

use crate::{
    errors::AmmError,
    math::{Calculator, U128},
    state::Config,
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
    )]
    pub config: Account<'info, Config>,
    pub base_mint: Box<InterfaceAccount<'info, Mint>>,
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = payer,
        seeds = [b"lp_mint", seed.to_le_bytes().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config.key(),
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = payer,
        associated_token::mint = base_mint,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub base_mint_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = payer,
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
        init_if_needed,
        payer = payer,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub user_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        seed: u64,
        init_base_amount: u64,
        init_quote_amount: u64,
        fee: u16,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        require!(fee < 10_000, AmmError::InvalidFee);

        self.config.set_inner(Config {
            seed,
            base_mint: self.base_mint.key(),
            quote_mint: self.quote_mint.key(),
            lp_mint: self.lp_mint.key(),
            authority: Some(self.payer.key()),
            locked: false,
            fee,
            config_bump: bumps.config,
            lp_bump: bumps.lp_mint,
        });

        // Transfer base token to amm
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
            init_base_amount,
            self.base_mint.decimals,
        )?;

        self.base_mint_vault.reload()?;

        // Transfer quote token to amm
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
            init_quote_amount,
            self.quote_mint.decimals,
        )?;

        self.quote_mint_vault.reload()?;

        // Calculate lp tokens to mint
        let liquidity = Calculator::to_u64(
            U128::from(self.quote_mint_vault.amount)
                .checked_mul(self.base_mint_vault.amount.into())
                .ok_or(AmmError::CheckedMulOverflow)?
                .integer_sqrt()
                .as_u128(),
        )?;

        let user_lp_amount = liquidity
            .checked_sub(
                (10u64)
                    .checked_pow(self.lp_mint.decimals.into())
                    .ok_or(AmmError::CheckedSubOverflow)?,
            )
            .ok_or(AmmError::InitLpAmountTooLess)?;

        // Mint lp tokens to user
        let signer_seeds: &[&[&[u8]]] = &[&[b"config", &seed.to_le_bytes(), &[bumps.config]]];
        token_interface::mint_to(
            CpiContext::new(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.lp_mint.to_account_info(),
                    to: self.user_lp_ata.to_account_info(),
                    authority: self.config.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            user_lp_amount,
        )?;

        Ok(())
    }
}
