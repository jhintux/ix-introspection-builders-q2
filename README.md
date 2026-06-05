# Instruction Introspection Builders Q2

This repo contains two Solana programs that use **instruction introspection** — reading another instruction in the same transaction via the [Instructions sysvar](https://docs.anza.xyz/runtime/sysvars#instructions) — for different purposes.

---

## AMM Builders Q2 — `burn` + `payout`

The AMM is a constant-product pool (base / quote) with LP mint liquidity. Most flows (`deposit`, `withdraw`, `swap`) are standard; the introspection pattern appears in the **withdraw path**, split across two instructions in one transaction.

### Pattern

1. **`burn`** — Burns `lp_amount` from the user's LP ATA and reduces LP supply.
2. **`payout`** — Reads the **previous** instruction in the transaction (index `current - 1`), verifies it is a `burn` to this program, parses `lp_amount` from the instruction data, and transfers pro-rata base and quote from the vaults to the user.

`payout` never takes `lp_amount` as an argument. It derives the amount from the preceding `burn`, so the burn and payout stay atomically linked and cannot be mismatched within the same transaction.

### What `payout` validates

Via `load_current_index_checked` and `load_instruction_at_checked`:

1. A previous instruction exists (`BurnInstructionNotFound` if not).
2. Program ID matches the AMM program.
3. Instruction discriminator matches `Burn`.
4. Bytes `[8..16]` decode to the burned `lp_amount`.

### Payout math

Because `burn` runs first, on-chain LP supply is already reduced. `payout` reconstructs pre-burn supply as `lp_mint.supply + lp_amount`, then applies the same pro-rata formula as `withdraw`:

$$
\Delta X = \left\lfloor \frac{L \cdot X}{S} \right\rfloor, \qquad
\Delta Y = \left\lfloor \frac{L \cdot Y}{S} \right\rfloor
$$

Slippage guards: `min_base_amount`, `min_quote_amount`.

See [`programs/amm-builders-q2/README.md`](programs/amm-builders-q2/README.md) for the full AMM model (initialize, deposit, withdraw, swap, lock/unlock).

---

## ix-introspection-builders-q2 — memo commitments

A house vs. player betting program where randomness is committed before a bet and revealed at resolution. Introspection ties **SPL Memo** pre-instructions to the program call that follows, so commitments and secrets are co-signed and ordered without storing plaintext on-chain at bet time.

### Instructions that introspect

| Instruction | Reads previous ix for |
|-------------|------------------------|
| `place_bet_v2` | Hash commitments in a memo (`sha256(house_secret):sha256(user_secret)`) |
| `resolve_bet` | Plaintext secrets in a memo (`house_secret:user_secret`) |

Legacy `place_bet` skips memo validation and stores a zero commitment.

### Commit flow (`place_bet_v2`)

Before `place_bet_v2`, the client includes a memo in the same transaction:

```
memo data: "<hex_sha256(house_secret)>:<hex_sha256(user_secret)>"
signers:   [house, player]
```

The program loads instruction at `current_index - 1` and checks:

1. Program ID is SPL Memo.
2. Exactly two accounts: house and player pubkeys.
3. Memo data is UTF-8, split on `:` into two parts.
4. Stored commitment: `sha256(part0_bytes || part1_bytes)` (raw hex strings concatenated).

Only hashes appear in the memo; the bet account persists the 32-byte commitment.

### Reveal flow (`resolve_bet`)

Before `resolve_bet`, another memo:

```
memo data: "<house_secret>:<user_secret>"
signers:   [house, player]
```

Same memo shape validation, then:

1. Parse both secrets.
2. Recompute `sha256(hex(sha256(house_secret)) || hex(sha256(user_secret)))`.
3. Require match with `bet.commitment` (`InvalidCommitment` on lie or tamper).
4. Derive roll: `(u64_from_le_bytes(commitment[0..8]) % 100) + 1`.
5. Pay out if `bet.roll > roll` (parimutuel odds with 1.5% house edge).

### Why introspection here

- **Co-signing** — Both parties must sign the memo.
- **Ordering** — The memo must immediately precede the program instruction; payloads cannot be swapped or replayed from another transaction.
- **No secret storage at place time** — Only the commitment hash is written to the bet PDA until resolve.

Full sequence diagrams, payout formula, and test setup: [`programs/ix-introspection-builders-q2/README.md`](programs/ix-introspection-builders-q2/README.md).

---

## Comparison

| | AMM (`burn` / `payout`) | Betting (`place_bet_v2` / `resolve_bet`) |
|---|-------------------------|--------------------------------------------|
| Previous instruction | Same program (`burn`) | SPL Memo program |
| Payload | Anchor discriminator + `lp_amount` | UTF-8 memo string (hashes or secrets) |
| Goal | Atomic burn-then-pay without trusting a caller-supplied amount | Commit–reveal randomness with dual signatures |
| Sysvar account | `instruction_sysvar` on `Payout` | Instructions sysvar in bet/resolve handlers |

---

## Build & test

```bash
anchor build
anchor test
```

Program READMEs: [AMM](programs/amm-builders-q2/README.md) · [Betting](programs/ix-introspection-builders-q2/README.md)
