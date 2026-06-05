# AMM Builders Q2

Constant-product automated market maker on Solana for a **base / quote** pair. Reserves sit in program vaults; liquidity providers hold an **LP mint** representing a pro-rata share of both tokens.

## Model

The pool keeps the invariant:

$$
X \cdot Y = k
$$

where $X$ and $Y$ are base and quote vault balances.

A swap adds effective input $X_o$ and removes output $Y_o$ (base → quote), preserving the product (up to integer rounding):

$$
(X + X_o)(Y - Y_o) = X \cdot Y
$$

Solving for output when $X_o$ is the **fee-adjusted** input:

$$
Y_o = \frac{Y \cdot X_o}{X + X_o}
$$

On-chain math uses **floor division** on `u64` / `U128`. Symbols below are real-valued; implementations use $\lfloor \cdot \rfloor$.

---

## Instructions

### `initialize`

Creates config, LP mint, vaults, seeds reserves, and mints initial LP to the creator.

| Parameter | Role |
|-----------|------|
| `init_base_amount` | Initial base $X_0$ |
| `init_quote_amount` | Initial quote $Y_0$ |
| `fee` | Swap fee in basis points; must be `< 10_000` |

**LP minted to creator**

$$
L_0 = \left\lfloor \sqrt{X_0 \cdot Y_0} \right\rfloor - 10^{d_{\text{lp}}}
$$

with $d_{\text{lp}} = 6$ (LP mint decimals). The subtracted term is minimum liquidity that stays locked in the pool.

---

### `deposit`

Adds liquidity at the current ratio. The user fixes one side (`fixed_side`) and supplies `amount_in`; the other leg is proportional.

Let $S$ = LP supply, $X$ / $Y$ = reserves.

**Other leg** (fixed base, `amount_in` = $\Delta X_{\text{req}}$):

$$
\Delta Y_{\text{req}} = \left\lfloor \frac{\Delta X_{\text{req}} \cdot Y}{X} \right\rfloor
$$

(Swap $X$ / $Y$ when `fixed_side` is quote.)

**LP credited** (binding side keeps the pool ratio):

$$
L_{\text{base}} = \left\lfloor \frac{\Delta X_{\text{req}} \cdot S}{X} \right\rfloor, \qquad
L_{\text{quote}} = \left\lfloor \frac{\Delta Y_{\text{req}} \cdot S}{Y} \right\rfloor
$$

$$
L = \min(L_{\text{base}}, L_{\text{quote}})
$$

**Tokens transferred** (amounts implied by $L$):

$$
\Delta X = \left\lfloor \frac{L \cdot X}{S} \right\rfloor, \qquad
\Delta Y = \left\lfloor \frac{L \cdot Y}{S} \right\rfloor
$$

Slippage: `max_amount` on the computed leg, `min_lp_amount` on $L$.

---

### `withdraw`

Burns `lp_amount` $L$ and returns pro-rata reserves:

$$
\Delta X = \left\lfloor \frac{L \cdot X}{S} \right\rfloor, \qquad
\Delta Y = \left\lfloor \frac{L \cdot Y}{S} \right\rfloor
$$

Slippage: `min_base_amount`, `min_quote_amount`.

---

### `swap`

Trades on $X \cdot Y = k$ with fee on the input. `direction` is base → quote or quote → base.

Let fee $= f$ (basis points, e.g. $f = 30$ → 0.30%), gross input $A_{\text{in}}$, reserves $R_{\text{in}}$, $R_{\text{out}}$.

**Effective input**

$$
A_{\text{eff}} = \left\lfloor \frac{A_{\text{in}} \cdot (10{,}000 - f)}{10{,}000} \right\rfloor
$$

**Output** (equivalent to $(R_{\text{in}} + A_{\text{eff}})(R_{\text{out}} - A_{\text{out}}) = R_{\text{in}} R_{\text{out}}$):

$$
A_{\text{out}} = \left\lfloor \frac{R_{\text{out}} \cdot A_{\text{eff}}}{R_{\text{in}} + A_{\text{eff}}} \right\rfloor
$$

Slippage: `min_amount_out`. Requires non-empty reserves and `locked == false`.

**Example** — reserves $1000$ / $1000$, $A_{\text{in}} = 100$, $f = 0$:

$$
A_{\text{out}} = \left\lfloor \frac{1000 \cdot 100}{1000 + 100} \right\rfloor = 90
$$

With $f = 100$ (1%): $A_{\text{eff}} = 99$, same formula → $90$ out.

---

### `lock` / `unlock`

Pool admin controls (set at `initialize` as `authority`). Only that signer may call these instructions.

| Instruction | Effect |
|-------------|--------|
| `lock` | Sets `locked = true`. Fails if already locked (`PoolAlreadyLocked`) or signer is not `authority` (`Unauthorized`). |
| `unlock` | Sets `locked = false`. Fails if not locked (`PoolNotLocked`) or signer is not `authority` (`Unauthorized`). |

While locked, `deposit`, `withdraw`, and `swap` return `PoolLocked`.

---

## Pool state

| Field | Meaning |
|-------|------|
| `fee` | Swap fee, basis points (0–9999) |
| `locked` | When `true`, `deposit` / `withdraw` / `swap` fail |
| `seed` | PDA seed for config and LP mint |

---

## Build & test

```bash
anchor build
cargo test --features idl-build
```

Program ID: `5TUw4ygwTkfZwSjhFonvj8haz4DuQd5vCFhJBHnYmmpc` (`declare_id!` in `programs/amm-builders-q2/src/lib.rs`).

### Tests running

![Tests running](assets/image.png)
