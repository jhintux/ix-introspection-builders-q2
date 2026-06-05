import * as anchor from "@coral-xyz/anchor";

export async function confirmTx(provider: anchor.AnchorProvider, tx: anchor.web3.TransactionSignature) {
  const blockhash = await provider.connection.getLatestBlockhash();
  const result = await provider.connection.confirmTransaction({
    blockhash: blockhash.blockhash,
    lastValidBlockHeight: blockhash.lastValidBlockHeight,
    signature: tx,
  });
  if (result.value.err) {
    throw new Error(result.value.err.toString());
  }
  console.log("Transaction confirmed", tx);
}
