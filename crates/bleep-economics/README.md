# bleep-economics

**Economic Nervous System — BLEEP Quantum Trust Network**

`bleep-economics` implements the complete tokenomics layer of BLEEP: constitutional supply enforcement, emission schedules, EIP-1559-style fee market, validator incentives, burn mechanics, supply tracking, oracle price aggregation, and game-theoretic safety proofs. Four parameters are constitutionally immutable and enforced by Rust `const_assert!`.

---

## License

Licensed under **Apache 2.0**.
Copyright © 2026 Muhammad Attahir.

---

## Architecture

```
bleep-economics
├── tokenomics           — CanonicalTokenomicsEngine, EmissionSchedule, SupplyState
├── distribution         — GenesisAllocation, VestingPolicy, FeeDistribution, BucketSnapshot
├── fee_market           — EIP-1559-style adaptive base fee
├── validator_incentives — Per-epoch reward calculation, commission splits
├── oracle_bridge        — Trust-minimised price feed aggregation
├── game_theory          — SafetyVerifier: mechanism design proofs and adversarial modelling
└── runtime              — Scheduler hooks called by bleep-scheduler each epoch
```

---

## Constitutional Token Parameters

These four values are enforced by Rust `const_assert!`. A code change that violates them does not compile. They cannot be changed by governance vote, software upgrade, or validator supermajority.

| Parameter | Value | Source |
|---|---|---|
| Maximum supply | **200,000,000 BLEEP** | `MAX_SUPPLY` in `tokenomics.rs` |
| Maximum per-epoch inflation | **500 bps (5%)** | `MAX_INFLATION_RATE_BPS` |
| Fee burn floor | **2,500 bps (25%)** | `FEE_BURN_BPS` in `distribution.rs` |
| Validator fee share | **5,000 bps (50%)** | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury fee share | **2,500 bps (25%)** | `FEE_TREASURY_BPS` |

The fee splits sum to exactly 10,000 bps — enforced by a separate compile-time assertion in `distribution.rs`.

---

## Token Distribution

| Allocation | Tokens | % | Launch Unlock | Vesting |
|---|---|---|---|---|
| Validator Rewards | 60,000,000 | 30% | 10,000,000 | Emission decay schedule |
| Ecosystem Fund | 50,000,000 | 25% | 5,000,000 | 10-year linear; governance disbursement |
| Community Incentives | 30,000,000 | 15% | 5,000,000 | Governance-triggered release |
| Foundation Treasury | 30,000,000 | 15% | 5,000,000 | 6-year linear; governance spending |
| Core Contributors | 20,000,000 | 10% | 0 | 1-year cliff + 4-year linear; immutable on-chain |
| Strategic Reserve | 10,000,000 | 5% | 0 | Governance-controlled unlock |
| **Total** | **200,000,000** | **100%** | **25,000,000** (12.5%) | |

---

## Validator Emission Schedule

Emissions decrease year-over-year from the 60,000,000 BLEEP validator rewards allocation:

| Year | Rate | Annual Emission |
|---|---|---|
| 1 | 12% | 7,200,000 BLEEP |
| 2 | 10% | 6,000,000 BLEEP |
| 3 | 8% | 4,800,000 BLEEP |
| 4 | 6% | 3,600,000 BLEEP |
| 5+ | 4% | ~2,400,000 BLEEP/yr |

Emission types: `Block`, `Epoch`, `Governance`, `Bootstrap`.

---

## Fee Market

EIP-1559-style adaptive base fee adjusting per block against a 50% capacity target:

```rust
use bleep_economics::fee_market::FeeMarket;

let base_fee = FeeMarket::current_base_fee(&supply_state);
```

| Parameter | Value |
|---|---|
| Minimum base fee | 1,000 microBLEEP |
| Maximum base fee | 10,000,000,000 microBLEEP |
| Max base fee change per block | 1,250 bps (12.5%) |
| Capacity target | 50% of MAX_TXS_PER_BLOCK |

Each collected fee is split: **25% burned** / **50% validator rewards** / **25% treasury** — sums enforced at compile time.

---

## Validator Incentives

Each epoch, `ValidatorEmissionSchedule` distributes rewards proportional to:
- Stake weight
- Uptime (missed blocks reduce reward proportionally)
- Reputation score from `bleep-consensus`
- Commission rate (validator-configurable)

Slashing deductions from `bleep-consensus` are reflected in the `SupplyState` burn counter and reconciled against the supply invariant.

---

## Oracle Bridge

`oracle_bridge.rs` aggregates price feeds from multiple authorised providers, applies a median filter, and stores the result on-chain for fee denomination and governance quorum calculations. Providers submitting outlier values are weight-penalised. The oracle result is queryable at `GET /rpc/oracle/price/BLEEP%2FUSD`.

---

## Game-Theoretic Safety — `SafetyVerifier`

`game_theory.rs` implements a `SafetyVerifier` that evaluates five adversarial attack models against current protocol parameters:

| Attack Model | Check |
|---|---|
| Equivocation | Is double-signing profitable after slashing? |
| Censorship | Can a validator cartel profitably censor transactions? |
| NonParticipation | Is withholding votes economically rational? |
| Griefing | Can an attacker reduce others' rewards at net profit? |
| Cartel formation | Is forming a >33% stake coalition stable? |

**A build fails if any model returns `is_profitable = true` at current parameters.** This is a CI gate, not a runtime check.

---

## Supply Invariant

At all times, the following invariant must hold:

```
total_minted - total_burned = circulating_supply + locked_supply
```

Verified by `bleep-core`'s `InvariantEnforcement` on every block. The `supply_state_verify` scheduler task checks this invariant on a timed interval and halts the node if violated — this task is marked **SAFETY CRITICAL** in the scheduler registry.

---

## Testing

```bash
cargo test -p bleep-economics
```

---

*Part of the [BLEEP Quantum Trust Network](https://github.com/BleepEcosystem/BLEEP-v1) · Protocol Version 5*
*© 2026 Muhammad Attahir — Apache 2.0 Licence*
