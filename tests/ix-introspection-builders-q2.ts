import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { IxIntrospectionBuildersQ2 } from "../target/types/ix_introspection_builders_q2";
import { createHash, randomBytes } from "crypto";
import { createMemoInstruction } from "@solana/spl-memo";
import { confirmTx } from "./helpers";

const BET_ROLL = 50;
const BET_AMOUNT = BigInt(anchor.web3.LAMPORTS_PER_SOL / 100);
const HOUSE_EDGE_BASIS_POINTS = BigInt(150);

describe("ix-introspection-builders-q2", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .ixIntrospectionBuildersQ2 as Program<IxIntrospectionBuildersQ2>;

  const house = anchor.web3.Keypair.generate();
  const user = anchor.web3.Keypair.generate();
  const seed = new BN(randomBytes(16));
  const house_secret = randomBytes(32).toString("hex");
  const user_secret = randomBytes(32).toString("hex");

  const [vault] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), house.publicKey.toBuffer()],
    program.programId,
  );

  const [bet] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("bet"),
      vault.toBuffer(),
      user.publicKey.toBuffer(),
      seed.toArrayLike(Buffer, "le", 16),
    ],
    program.programId,
  );

  it("Airdrop", async () => {
    await Promise.all([
      provider.connection.requestAirdrop(
        house.publicKey,
        1000 * anchor.web3.LAMPORTS_PER_SOL,
      ),
      provider.connection.requestAirdrop(
        user.publicKey,
        1000 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    ]);
  });

  it("Is initialized!", async () => {
    await program.methods
      .initialize(new BN(100 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsStrict({
        house: house.publicKey,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([house])
      .rpc()
      .then((tx) => confirmTx(provider, tx));
  });

  it("Place bet", async () => {
    const combined_commitment_ix = createMemoInstruction(
      `${createHash("sha256").update(house_secret).digest("hex")}:${createHash("sha256").update(user_secret).digest("hex")}`,
      [house.publicKey, user.publicKey],
    );

    await program.methods
      .placeBetV2(seed, BET_ROLL, new BN(BET_AMOUNT.toString()))
      .accountsStrict({
        player: user.publicKey,
        house: house.publicKey,
        vault,
        bet,
        instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user, house])
      .preInstructions([combined_commitment_ix])
      .rpc()
      .then((tx) => confirmTx(provider, tx));
  });

  it("Resolve a bet", async () => {
    const combined_secrets_ix = createMemoInstruction(
      `${house_secret}:${user_secret}`,
      [house.publicKey, user.publicKey],
    );

    await program.methods
      .resolveBet()
      .accountsStrict({
        player: user.publicKey,
        house: house.publicKey,
        vault,
        bet,
        instructionSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user, house])
      .preInstructions([combined_secrets_ix])
      .rpc()
      .then((tx) => confirmTx(provider, tx));
  });

  it.skip("Refund a bet", async () => {
    const betAccount = await program.account.bet.fetch(bet);
    console.log("Bet slot", betAccount.slot.toString());
    const epoch = await provider.connection.getEpochInfo();
    const blockTime = await provider.connection.getBlockTime(epoch.epoch);
    console.log("Block time", blockTime);

    // https://docs.surfpool.run/rpc/cheatcodes#surfnet_timeTravel
    const res = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [
          {
            absoluteTimestamp: blockTime + 5 * 60 * 1000,
          },
        ],
      }),
      headers: {
        "Content-Type": "application/json",
      },
    });
    if (!res.ok) {
      throw new Error(`Time travel failed: ${res.statusText}`);
    }
    console.log("Time travel response", await res.json());

    await program.methods
      .refundBet()
      .accountsStrict({
        player: user.publicKey,
        house: house.publicKey,
        vault,
        bet,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user])
      .rpc({ skipPreflight: true });
    //.then(confirmTx);
  });

  it.skip("test time travel", async () => {
    const epoch = await provider.connection.getEpochInfo();
    console.log("Epoch", epoch);
    const blockTime = await provider.connection.getBlockTime(epoch.epoch);
    console.log("Block time", blockTime);
  });

  
});
