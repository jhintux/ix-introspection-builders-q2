use crate::{
    errors::AmmError,
    math::{Calculator},
    state::{Config, FixedSide},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, MintTo, TransferChecked, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
pub struct Deposit<'info> {
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

impl<'info> Deposit<'info> {
    pub fn deposit(
        &mut self,
        fixed_side: FixedSide,
        amount_in: u64,
        max_amount: u64,
        min_lp_amount: u64,
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount_in > 0, AmmError::AmountTooLow);

        let base_reserve = self.base_mint_vault.amount;
        let quote_reserve = self.quote_mint_vault.amount;
        let lp_supply = self.lp_mint.supply;

        require!(base_reserve > 0 && quote_reserve > 0, AmmError::EmptyReserve);
        require!(lp_supply > 0, AmmError::NoLpSupply);

        let (base_amount, quote_amount) = match fixed_side {
            FixedSide::Base => {
                let quote_amount =
                    Calculator::proportional_amount(amount_in, base_reserve, quote_reserve)?;
                require!(quote_amount > 0, AmmError::AmountTooLow);
                require!(quote_amount <= max_amount, AmmError::ExceededSlippage);
                (amount_in, quote_amount)
            }
            FixedSide::Quote => {
                let base_amount =
                    Calculator::proportional_amount(amount_in, quote_reserve, base_reserve)?;
                require!(base_amount > 0, AmmError::AmountTooLow);
                require!(base_amount <= max_amount, AmmError::ExceededSlippage);
                (base_amount, amount_in)
            }
        };

        let lp_from_base = Calculator::liquidity_from_amount(base_amount, base_reserve, lp_supply)?;
        let lp_from_quote = Calculator::liquidity_from_amount(quote_amount, quote_reserve, lp_supply)?;
        let lp_amount = lp_from_base.min(lp_from_quote);

        require!(lp_amount >= min_lp_amount, AmmError::ExceededSlippage);
        require!(lp_amount > 0, AmmError::LiquidityTooLow);

        let base_amount = Calculator::amount_from_liquidity(lp_amount, base_reserve, lp_supply)?;
        let quote_amount = Calculator::amount_from_liquidity(lp_amount, quote_reserve, lp_supply)?;

        require!(base_amount > 0 && quote_amount > 0, AmmError::AmountTooLow);

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
            base_amount,
            self.base_mint.decimals,
        )?;

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
            quote_amount,
            self.quote_mint.decimals,
        )?;

        let signer_seeds: &[&[&[u8]]] =
            &[&[b"config", &self.config.seed.to_le_bytes(), &[self.config.config_bump]]];
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
            lp_amount,
        )?;

        Ok(())
    }
}
