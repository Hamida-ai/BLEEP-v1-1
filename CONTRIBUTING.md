# Contributing to BLEEP

**Protocol Version 5 · Quantum Trust Network**

BLEEP is a self-funded, open-source Layer 1 blockchain built for proven execution, intent-native runtime, and post-quantum cryptographic security. We welcome contributors who care about correctness, cryptographic rigour, and building infrastructure that lasts.

This document covers how to contribute, what areas need help, and the standards we expect.

---

## Before You Start

Read these three documents first:

- [`BUILDING.md`](BUILDING.md) — how to build the project from source
- [`SECURITY.md`](SECURITY.md) — responsible disclosure policy; **do not open public issues for security vulnerabilities**
- [`CODE-OF-CONDUCT.md`](CODE-OF-CONDUCT.md) — community standards

---

## Where BLEEP Needs Help

### High Priority (Pre-Testnet)

| Area | Description | Relevant Crates |
|---|---|---|
| **Post-quantum cryptography** | Review, test, and harden PQ primitive usage | `bleep-crypto`, `bleep-zkp`, `bleep-wallet-core` |
| **STARK proof system** | Winterfell circuit optimisation, proof size reduction, trace width experimentation | `bleep-zkp` |
| **Consensus hardening** | Edge cases in PoS-BFT finality, epoch transitions, recovery paths | `bleep-consensus` |
| **Cross-chain bridge** | Tier 3 ZK proof relay, Tier 4 executor auction, nullifier set correctness | `bleep-interop` |
| **Developer tooling** | TypeScript/Python SDK wrappers for 46 RPC endpoints | New — `bleep-sdk` |
| **Documentation** | Technical guides, tutorials, RPC API docs | `docs/` |
| **Testing** | Additional fuzz targets, property-based tests, adversarial scenarios | All crates |

### Medium Priority

| Area | Description | Relevant Crates |
|---|---|---|
| **Intent execution** | PAT engine improvements, new intent types, gas model refinements | `bleep-pat`, `bleep-vm` |
| **P2P networking** | Gossip efficiency, peer scoring, onion routing improvements | `bleep-p2p` |
| **Self-healing** | New fault detection heuristics, recovery strategies | `bleep-state` |
| **Governance** | ZK voting improvements, proposal template library | `bleep-governance` |
| **Block explorer** | Web-based explorer for blocks, transactions, and governance | New — UI project |

---

## Codebase Structure

```
BLEEP-v1/
├── crates/                  # 19 workspace crates — one responsibility each
│   ├── bleep-core/          # Shared types: Block, ZKTransaction, mempool
│   ├── bleep-crypto/        # PQ cryptography — SPHINCS+, Kyber, BLAKE3, SHA3
│   ├── bleep-zkp/           # Winterfell STARK — BlockValidityProver/Verifier
│   ├── bleep-consensus/     # PoS-BFT, PBFT fallback, slashing, epochs
│   ├── bleep-state/         # Sparse Merkle Trie, RocksDB, cross-shard 2PC
│   ├── bleep-vm/            # 7-tier intent VM (BSL-1.1 → Apache 2028)
│   ├── bleep-pat/           # 6-layer intent PAT engine
│   ├── bleep-ai/            # Deterministic AI advisory (MIT licence)
│   ├── bleep-p2p/           # Kademlia DHT, Plumtree gossip, onion routing
│   ├── bleep-rpc/           # 46 JSON-RPC endpoints
│   ├── bleep-auth/          # Credentials, RBAC, audit log, validator binding
│   ├── bleep-scheduler/     # 20 protocol maintenance tasks
│   ├── bleep-economics/     # Tokenomics, fee market, oracle bridge
│   ├── bleep-governance/    # Constitution, ZK voting, forkless upgrades
│   ├── bleep-indexer/       # Block, Tx, Account, Governance, Validator indexes
│   ├── bleep-wallet-core/   # SPHINCS+, Falcon key management, BIP-39
│   ├── bleep-telemetry/     # Prometheus-compatible metrics
│   ├── bleep-cli/           # Validator, governance, AI, ZKP, faucet CLI
│   └── bleep-interop/       # BLEEP Connect — 10 sub-crates, 4 bridge tiers
├── docs/                    # Technical documentation, specs, tutorials
├── config/                  # genesis.json, mainnet_config.json, testnet_config.json
├── tests/                   # Integration test suites
├── scripts/                 # Deployment and utility scripts
└── tools/                   # Development tools
```

---

## Getting Started

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/BLEEP-v1.git
cd BLEEP-v1
```

### 2. Build

```bash
# Check all crates compile
cargo check --workspace

# Full release build
cargo build --workspace --release
```

See [`BUILDING.md`](BUILDING.md) for full prerequisites and platform-specific instructions.

### 3. Create a Feature Branch

```bash
git checkout -b feat/your-feature-name
# or
git checkout -b fix/issue-description
```

### 4. Make Your Changes

Follow these conventions:

**Code:**
- Run `cargo fmt --all` before committing
- Run `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- Add or update tests for any changed behaviour
- Add SPDX licence headers to all new source files

**Commits:**
- Use conventional commit format: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`
- Keep commits small and focused — one logical change per commit
- Reference issues: `Closes #123`, `Related to #456`

**Tests:**
```bash
# Full test suite
cargo test --workspace --all-features

# Single crate
cargo test -p bleep-consensus

# Specific test
cargo test -p bleep-zkp stark_proof_roundtrip
```

### 5. Submit a Pull Request

- Target the `main` branch
- Provide a clear description of what the PR does and why
- Reference any related issues
- Ensure CI passes before requesting review
- Be patient and constructive in review discussions

---

## Contribution Standards

### For Cryptographic Code

Contributions touching `bleep-crypto`, `bleep-zkp`, or `bleep-wallet-core` are held to a higher standard:

- Any change to cryptographic primitive usage requires a written rationale in the PR description
- Secret key material must be wrapped in `zeroize::Zeroizing<Vec<u8>>` — no exceptions
- No classical public-key primitive (RSA, ECDSA, x25519, BLS) may be introduced on a sensitive path without an explicit governance proposal
- New ZKP constructions require a reference to a peer-reviewed paper or NIST standard

### For Consensus Code

- All consensus-critical computations must be deterministic — identical inputs must produce identical outputs on all honest nodes
- Changes to slashing parameters, finality thresholds, or epoch logic require a written safety argument
- New consensus paths must include adversarial test scenarios in `bleep-consensus/src/chaos_engine.rs`

### For Documentation

- Technical claims must reference the relevant source constant, function, or file
- Projected metrics (TPS, latency) must be labelled as simulated or measured
- Do not claim security properties that are not verified in the codebase or audit reports

---

## Licensing

By contributing, you agree that your contributions will be published under the licence applicable to the crate you are modifying:

| Crate / Directory | Licence |
|---|---|
| All crates except below | **Apache 2.0** |
| `bleep-vm` | **BSL-1.1** — converts to Apache 2.0 on 2028-07-13 |
| `bleep-ai` | **MIT** |
| `docs/` | **CC-BY-4.0** |

See [`LICENSE`](LICENSE), [`NOTICE`](NOTICE), and per-crate `LICENSE` files for full terms.

---

## Security Vulnerabilities

**Do not open a public GitHub issue for security vulnerabilities.**

Report them privately to **security@bleepecosystem.com**. See [`SECURITY.md`](SECURITY.md) for the full responsible disclosure policy, response timeline, and scope definition.

---

## Community

| Channel | Link |
|---|---|
| Discord | [discord.gg/bleepecosystem](https://discord.gg/bleepecosystem) |
| Telegram | [t.me/bleepecosystem](https://t.me/bleepecosystem) |
| Twitter / X | [@BleepEcosystem](https://twitter.com/BleepEcosystem) |
| GitHub Discussions | [github.com/BleepEcosystem/BLEEP-v1/discussions](https://github.com/BleepEcosystem/BLEEP-v1/discussions) |

---

*BLEEP · Quantum Trust Network · Protocol Version 5*
*© 2026 Muhammad Attahir — Apache 2.0 Licence*Please be kind, curious, and inclusive.

---

📜 Licensing

By contributing, you agree that your code will be published under the licenses used in this repository:

Directory           | License
--------------------|------------------------
/core               | Apache-2.0
/smart-contracts    | GPLv3
/sdk                | MIT
/vm                 | BSL 1.1 → Apache 2.0 (2028)
/docs               | CC-BY-4.0

---

🙌 Thank You

Your time and contributions help BLEEP evolve into a powerful public good.  
Together, we are building the future of intelligent, resilient, decentralized systems.

BLEEP ≡ Evolve Everything. 
