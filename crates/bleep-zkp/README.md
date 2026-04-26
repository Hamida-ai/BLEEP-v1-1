# bleep-zkp

**Zero-Knowledge Proof Primitives for BLEEP**

`bleep-zkp` provides STARK-based block validity proofs and post-quantum ZKP constructions used throughout the BLEEP ecosystem for governance voting, cross-chain verification, and validator attestation.

---

## License

Licensed under **MIT**.
Copyright © 2025 Muhammad Attahir.

---

## Architecture

```
bleep-zkp
├── stark_proofs   — Block validity AIR + STARK prover/verifier (Winterfell)
└── pq_proofs      — Post-quantum ZKP constructions
```

---

## Modules

### `stark_proofs` — Block Validity Circuit

Proves, in zero knowledge and without a trusted setup, that a BLEEP block is valid:

**Circuit inputs:**

| Slot | Public Input | Description |
|------|-------------|-------------|
| `x[0]` | `block_index` | Block number as `BaseElement` |
| `x[1]` | `epoch_id` | Epoch identifier |
| `x[2]` | `tx_count` | Number of transactions |
| `x[3]` | `merkle_root_hash` | SHA3-256 of the Merkle root (lower 31 bytes) |
| `x[4]` | `validator_pk_hash` | SHA3-256 of the validator public key |

**Private witnesses (known only to the prover):**
- `block_hash_witness` — the 32-byte block hash
- `sk_seed_witness` — the 32-byte validator secret key seed

**What the circuit proves:**
1. The block hash is the SHA3-256 of its public fields.
2. The validator knows the secret key whose hash equals the embedded public key hash.
3. The `epoch_id` is consistent with `block_index` and `blocks_per_epoch`.
4. The Merkle root commitment is non-zero (block has been committed).

**Key types:** `StarkProof`, `BlockValidityAir`, `BlockValidityProver`, `BlockValidityVerifier`.

```rust
use bleep_zkp::{BlockValidityProver, BlockValidityVerifier, StarkProof};

let prover = BlockValidityProver::new(block_witness);
let proof: StarkProof = prover.prove()?;

let verifier = BlockValidityVerifier::new(public_inputs);
verifier.verify(&proof)?;
```

STARKs require **no trusted setup**. Proofs are transparent and post-quantum secure (hash-based, not ECC-based).

### `pq_proofs` — Post-Quantum ZKP Constructions

Additional ZKP constructions designed for quantum-adversarial environments, used in:
- Post-quantum governance vote privacy
- Quantum-safe asset recovery proofs
- Cross-chain ZKP relay via `bleep-interop`

---

## Properties

| Property | STARK | Notes |
|----------|-------|-------|
| Trusted setup | ❌ None | Transparent; no SRS required |
| Post-quantum secure | ✅ | Hash-based; no ECC |
| Proof size | ~100 KB | Larger than SNARKs but verifiable faster |
| Prover time | ~seconds | Depends on circuit size |
| Verifier time | ~milliseconds | Logarithmic in trace length |

---

## Integration

`bleep-zkp` is consumed by:

| Consumer | Purpose |
|----------|---------|
| `bleep-consensus` | Block validity attestation |
| `bleep-governance` | Anonymous ZK vote verification |
| `bleep-interop` | Cross-chain STARK proof relay (Layer 3) |
| `bleep-crypto` | Winterfell STARK integration in `zkp_verification` |
| `bleep-vm` | ZK engine for contract ZK verification intents |

---

## Testing

```bash
cargo test -p bleep-zkp
```

---

*Part of the [BLEEP Ecosystem](https://github.com/BleepEcosystem/BLEEP-V1)*
