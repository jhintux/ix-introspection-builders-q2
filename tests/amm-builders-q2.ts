import * as anchor from "@coral-xyz/anchor";
import { AnchorError, BN, Program } from "@coral-xyz/anchor";
import { AmmBuildersQ2 } from "../target/types/amm_builders_q2";
import { randomBytes } from "crypto";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAssociatedTokenAddressSync,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";
import { confirmTx } from "./helpers";

async function read_token_account_balance(
  connection: anchor.web3.Connection,
  token_account: anchor.web3.PublicKey,
) {
  const balance = await connection.getTokenAccountBalance(token_account);
  return balance.value.amount;
}

interface PoolSetup {
  seed: BN;
  config: anchor.web3.PublicKey;
  lp_mint: anchor.web3.PublicKey;
  base_mint: anchor.web3.PublicKey;
  quote_mint: anchor.web3.PublicKey;
  base_mint_vault: anchor.web3.PublicKey;
  quote_mint_vault: anchor.web3.PublicKey;
  user_base: anchor.web3.PublicKey;
  user_quote: anchor.web3.PublicKey;
  user_lp: anchor.web3.PublicKey;
  init_base: BN;
  init_quote: BN;
  fee: number;
}

describe("amm-builders-q2", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ammBuildersQ2 as Program<AmmBuildersQ2>;
  const user = anchor.web3.Keypair.generate();

  async function create_and_mint(amount: number) {
    const mint = await createMint(
      provider.connection,
      user,
      user.publicKey,
      null,
      6,
    );
    const ata = await createAssociatedTokenAccount(
      provider.connection,
      user,
      mint,
      user.publicKey,
    );

    await mintTo(provider.connection, user, mint, ata, user, amount);

    return { mint, ata };
  }

  async function funded_user(token_amount: number) {
    await provider.connection.requestAirdrop(
      user.publicKey,
      100 * anchor.web3.LAMPORTS_PER_SOL,
    );
    let { mint: base_mint, ata: user_base } = await create_and_mint(
      token_amount,
    );
    let { mint: quote_mint, ata: user_quote } = await create_and_mint(
      token_amount,
    );

    return { base_mint, quote_mint, user_base, user_quote };
  }

  function pool_accounts(
    base_mint: anchor.web3.PublicKey,
    quote_mint: anchor.web3.PublicKey,
    user_base: anchor.web3.PublicKey,
    user_quote: anchor.web3.PublicKey,
  ): PoolSetup {
    const seed = new BN(randomBytes(8), "le");
    const [config] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );
    const [lp_mint] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );
    return {
      seed,
      config,
      lp_mint,
      base_mint,
      quote_mint,
      base_mint_vault: getAssociatedTokenAddressSync(base_mint, config, true),
      quote_mint_vault: getAssociatedTokenAddressSync(quote_mint, config, true),
      user_base,
      user_quote,
      user_lp: getAssociatedTokenAddressSync(lp_mint, user.publicKey),
      init_base: new BN(0),
      init_quote: new BN(0),
      fee: 0,
    };
  }

  async function read_balances(pool: PoolSetup) {
    return {
      base_vault: (
        await provider.connection.getTokenAccountBalance(pool.base_mint_vault)
      ).value.amount,
      quote_vault: (
        await provider.connection.getTokenAccountBalance(pool.quote_mint_vault)
      ).value.amount,
      user_base: (
        await provider.connection.getTokenAccountBalance(pool.user_base)
      ).value.amount,
      user_quote: (
        await provider.connection.getTokenAccountBalance(pool.user_quote)
      ).value.amount,
      user_lp: (await provider.connection.getTokenAccountBalance(pool.user_lp))
        .value.amount,
      lp_supply: (await provider.connection.getTokenSupply(pool.lp_mint)).value
        .amount,
    };
  }

  function proportional_amount(amount: BN, reserve_in: BN, reserve_out: BN) {
    return amount.mul(reserve_out).div(reserve_in);
  }

  function amount_for_liquidity(lp_amount: BN, reserve: BN, lp_supply: BN) {
    return proportional_amount(lp_amount, lp_supply, reserve);
  }
  async function assert_token_balance(
    token_account: anchor.web3.PublicKey,
    expected_balance: BN,
  ) {
    const account = await provider.connection.getTokenAccountBalance(
      token_account,
    );
    expect(account.value.amount).to.equal(expected_balance.toString());
  }

  async function client_withdraw_preview(
    lp_amount: BN,
    base_reserve: BN,
    quote_reserve: BN,
    lp_supply: BN,
  ) {
    return {
      base_amount: amount_for_liquidity(lp_amount, base_reserve, lp_supply),
      quote_amount: amount_for_liquidity(lp_amount, quote_reserve, lp_supply),
    };
  }

  async function initialize_pool(
    base_mint: anchor.web3.PublicKey,
    quote_mint: anchor.web3.PublicKey,
    user_base: anchor.web3.PublicKey,
    user_quote: anchor.web3.PublicKey,
    init_base: BN,
    init_quote: BN,
    fee: number,
  ) {
    const pool = pool_accounts(base_mint, quote_mint, user_base, user_quote);
    pool.init_base = init_base;
    pool.init_quote = init_quote;
    pool.fee = fee;

    await program.methods
      .initialize(pool.seed, pool.init_base, pool.init_quote, pool.fee)
      .accountsStrict({
        payer: user.publicKey,
        config: pool.config,
        baseMint: pool.base_mint,
        quoteMint: pool.quote_mint,
        lpMint: pool.lp_mint,
        baseMintVault: pool.base_mint_vault,
        quoteMintVault: pool.quote_mint_vault,
        userBaseAta: pool.user_base,
        userQuoteAta: pool.user_quote,
        userLpAta: pool.user_lp,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc()
      .then((tx) => confirmTx(provider, tx));

    return pool;
  }

  async function execute_withdraw(
    pool: any,
    lp_amount: BN,
    base_amount: BN,
    quote_amount: BN,
  ) {
    await program.methods
      .burn(lp_amount)
      .accountsStrict({
        payer: user.publicKey,
        config: pool.config,
        baseMint: pool.base_mint,
        quoteMint: pool.quote_mint,
        lpMint: pool.lp_mint,
        baseMintVault: pool.base_mint_vault,
        quoteMintVault: pool.quote_mint_vault,
        userLpAta: pool.user_lp,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .postInstructions([
        await program.methods
          .payout(base_amount, quote_amount)
          .accountsStrict({
            payer: user.publicKey,
            config: pool.config,
            baseMint: pool.base_mint,
            quoteMint: pool.quote_mint,
            lpMint: pool.lp_mint,
            baseMintVault: pool.base_mint_vault,
            quoteMintVault: pool.quote_mint_vault,
            userBaseAta: pool.user_base,
            userQuoteAta: pool.user_quote,
            instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .instruction(),
      ])
      .signers([user])
      .rpc()
      .then((tx) => confirmTx(provider, tx));
  }

  it("Burn & Payout", async () => {
    let { base_mint, quote_mint, user_base, user_quote } = await funded_user(
      2_000_000_000,
    );

    let pool = await initialize_pool(
      base_mint,
      quote_mint,
      user_base,
      user_quote,
      new BN(1_000_000_000),
      new BN(1_000_000_000),
      0,
    );
    let before = await read_balances(pool);
    const lp_amount = new BN(before.user_lp);
    const { base_amount, quote_amount } = await client_withdraw_preview(
      lp_amount,
      new BN(before.base_vault),
      new BN(before.quote_vault),
      new BN(before.lp_supply),
    );

    await execute_withdraw(pool, lp_amount, base_amount, quote_amount);

    assert_token_balance(pool.base_mint_vault, new BN(0));
    assert_token_balance(pool.quote_mint_vault, new BN(0));
    assert_token_balance(pool.user_lp, new BN(0));
    assert_token_balance(pool.lp_mint, new BN(0));
  });

  it("Fails to payout without burning first", async () => {
    let { base_mint, quote_mint, user_base, user_quote } = await funded_user(
      2_000_000_000,
    );

    let pool = await initialize_pool(
      base_mint,
      quote_mint,
      user_base,
      user_quote,
      new BN(1_000_000_000),
      new BN(1_000_000_000),
      0,
    );
    let before = await read_balances(pool);
    const lp_amount = new BN(before.user_lp);
    const { base_amount, quote_amount } = await client_withdraw_preview(
      lp_amount,
      new BN(before.base_vault),
      new BN(before.quote_vault),
      new BN(before.lp_supply),
    );
    try {
      await program.methods
        .payout(base_amount, quote_amount)
        .accountsStrict({
          payer: user.publicKey,
          config: pool.config,
          baseMint: pool.base_mint,
          quoteMint: pool.quote_mint,
          lpMint: pool.lp_mint,
          baseMintVault: pool.base_mint_vault,
          quoteMintVault: pool.quote_mint_vault,
          userBaseAta: pool.user_base,
          userQuoteAta: pool.user_quote,
          instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc()
        .then((tx) => confirmTx(provider, tx));
    } catch (error: unknown) {
      if (error instanceof AnchorError) {
        expect(error.error.errorCode.number).to.equal(program.idl.errors[23].code);
      } else {
        console.log(error);
      }
    }
  });
});
