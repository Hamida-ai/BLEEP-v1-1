# Changelog

All notable changes to the BLEEP Quantum Trust Network are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Licence note:** Core protocol is Apache 2.0. `bleep-vm` is BSL-1.1 (converts to Apache 2.0 on 2028-07-13). `bleep-ai` is MIT. See [`LICENSE`](./LICENSE), [`NOTICE`](./NOTICE), and per-crate `LICENSE` files for full terms.

---

## [Unreleased]

### Changed
- Repository documentation updated to Protocol Version 5 narrative: Proven Execution · Quantum Foundation · Intent Native
- README, SECURITY, ROADMAP, CONTRIBUTING, NOTICE, BUILDING rewritten to reflect current architecture and strategic positioning
- Protocol version references corrected from v4 to v5 across all documentation
- ROADMAP expanded with explicit mainnet gate requirements and known limitations section

---

## [1.0.0] — Sprint 9 — 2026-04-10

### Highlights

Sprint 9 is the **security hardening and audit-preparation release** — Protocol Version 5. All consensus, state, and cryptographic paths have been subjected to chaos testing, property-based testing, and an internal security audit. The Winterfell STARK block validity circuit is wired to `BlockValidityProver` and `BlockValidityVerifier` and benchmarked against the 3,000ms slot budget. All Critical and High audit findings are resolved.

### Added

**STARK Proof System (Protocol Version 5)**
- `BlockValidityProver` and `BlockValidityVerifier` wired to consensus pipeline in `bleep-zkp`
- 48-column execution trace over Winterfell f128 field
- FRI cryptographic backend with BLAKE3 commitment hashing
- `STARKProofTamper` adversarial test scenario added to 72-hour test suite
- `LoadStress(10,000 TPS, 60s)` scenario with STARK proof timing verification
- Proof generation benchmarked at ~850ms on reference hardware (8-core, 32 GB RAM)
- Proof verification benchmarked at ~12ms — within 3,000ms slot budget

**Security & Auditing**
- `security_audit.rs` in `bleep-consensus`: on-demand `AuditReport` generation
- `SECURITY_AUDIT_SPRINT9.md` in `docs/`: internal audit report — 2 Critical, 3 High, 4 Medium, 3 Low, 2 Informational; all Critical and High resolved
- `THREAT_MODEL.md` in `docs/`: trust boundary map, 11 threat categories, per-crate audit priorities
- Fuzz targets in `bleep-state/src/fuzz/`: `fuzz_merkle_insert`, `fuzz_state_apply_tx`
- Fuzz targets in `bleep-crypto/fuzz/`: `fuzz_tx_sign`, `fuzz_merkle_commitment`
- `proptest_sprint8.rs` in `bleep-state`: 40+ property-based tests

**Integration Tests**
- `tests/sprint9_integration.rs`: end-to-end suite — validator lifecycle, cross-shard 2PC, governance, RPC
- `phase4_ai_integration_tests.rs` in `bleep-ai/tests/`: Phase 4 AI advisory integration tests

**Chaos Testing**
- `chaos_engine.rs` in `bleep-consensus`: `ChaosEngine` with `ChaosScenario` variants (network partition, validator crash, Byzantine vote, clock skew)
- `ContinuousChaosHarness` for sustained adversarial testing
- `CHAOS_TESTING.md` in `docs/`: runbook for chaos test execution

**Documentation**
- `THREAT_MODEL.md`, `SECURITY_AUDIT_SPRINT9.md`, `CI_CD_PIPELINE.md`, `CI_CD_QUICK_REFERENCE.md`
- `docs/phase4_shard_recovery.md`: Phase 4 advanced shard recovery orchestration
- `docs/specs/rpc_api_spec.md`: full RPC endpoint specification (46 endpoints)
- `docs/specs/state_transition.md`: state transition formal specification
- `docs/tutorials/build_node.md`, `docs/tutorials/write_contract.md`: complete tutorials
- `docs/glossary.md`: comprehensive ecosystem glossary
- Per-crate `README.md` for all 19 workspace crates
- `CHANGELOG.md` (this file)
- `LICENSE_BSL.md`: rendered BSL-1.1 licence for `bleep-vm`
- `WHITEPAPER.md` updated to Protocol Version 5

### Changed
- `bleep-consensus`: `ConsensusOrchestrator` surfaces `ConsensusMetrics` in `select_mode()` for deterministic replay
- `bleep-state`: `StateManager.apply()` upgraded to write batches with explicit fsync for crash safety
- `bleep-rpc`: rate-limit headers (`X-RateLimit-*`) added to all write endpoint responses
- `bleep-scheduler`: task timeout enforcement hardened; hung tasks emit `warn!` before cancellation
- `bleep-p2p`: anti-replay nonce cache enlarged from 8k to 64k slots (LRU eviction)

### Fixed
- `cross_shard_2pc.rs`: coordinator no longer deadlocks when all shards abort simultaneously
- `bleep-crypto/zkp_verification.rs`: batch proof aggregation panic on empty proof list resolved
- `bleep-rpc`: `/rpc/validator/list` now reads atomically from `ValidatorRegistry` — previously returned stale data when a validator was slashed mid-epoch

---

## [0.9.0] — Sprint 8 — 2025-12-15

### Highlights

Sprint 8 delivers the **Phase 5 AI-driven protocol evolution layer** in `bleep-governance`, shard performance benchmarking, and the complete economic nervous system (`bleep-economics` Phase 5).

### Added

**AI Protocol Evolution (Phase 5)**
- `bleep-governance`: `governance_engine.rs`, `protocol_rules.rs`, `apip.rs` (Autonomous Protocol Improvement Proposals), `safety_constraints.rs`, `ai_reputation.rs`, `protocol_evolution.rs`, `ai_hooks.rs`, `invariant_monitoring.rs`, `governance_voting.rs`, `deterministic_activation.rs`
- `phase5_integration_tests.rs` and `phase5_comprehensive_tests.rs`

**Economics (Phase 5)**
- `bleep-economics`: `oracle_bridge.rs` (trust-minimised price aggregation), `game_theory.rs` (`SafetyVerifier` — build fails if any attack model returns `is_profitable = true`), `runtime.rs`
- `ALLOC_*` constants for all genesis allocation buckets published as public API
- `FEE_BURN_BPS`, `FEE_VALIDATOR_REWARD_BPS`, `FEE_TREASURY_BPS` constants

**Consensus Performance**
- `performance_bench.rs` in `bleep-consensus`: `PerformanceBenchmark`, `BenchmarkResult`, `TpsWindow`
- `shard_coordinator.rs`: cross-shard TPS stress testing harness

**Pre-testnet**
- Testnet faucet: 10 BLEEP per address per 24 hours (reduced from 1,000)
- Automatic 10 BLEEP credit on `bleep-cli wallet create`

### Changed
- `bleep-scheduler`: 20 built-in tasks across 7 categories (up from 14)
- `bleep-indexer`: `CrossShardIndex` and `AIEventIndex` added
- `bleep-p2p`: onion router enabled as opt-in (`enable_onion = true`)
- `bleep-auth`: `AuditLog` upgraded to SHA3-256 Merkle-chained entries

---

## [0.8.0] — Sprint 7 — 2025-09-20

### Highlights

Sprint 7 delivers **Phase 4 cross-shard atomicity and self-healing orchestration** — the advanced state management layer that enables BLEEP to operate across 10 shards with deterministic fault recovery.

### Added

**Cross-Shard 2PC**
- `cross_shard_2pc.rs`: `TwoPhaseCommitCoordinator` with deterministic coordinator selection from transaction hash
- `cross_shard_locking.rs`: advisory locking for cross-shard operations
- `cross_shard_recovery.rs`: coordinator recovery after crash
- `cross_shard_safety_invariants.rs`: invariant verification at commit and abort

**Self-Healing**
- `self_healing_orchestrator.rs`: Healthy → Degraded → Critical → Recovering state machine
- `advanced_fault_detector.rs`: multi-signal fault classification
- `rollback_engine.rs`: deterministic state rollback to last checkpoint
- `snapshot_engine.rs`: periodic state snapshots for recovery

**Shard Lifecycle**
- `shard_lifecycle.rs`: shard create, activate, deactivate with governance gating
- `shard_epoch_binding.rs`: epoch-aligned shard transitions

### Changed
- `bleep-p2p`: Kyber-768 session crypto upgraded to Kyber-1024

---

## [0.7.0] — Sprint 6 — 2025-07-08

### Highlights

Sprint 6 completes the **BLEEP Connect interoperability layer** — all four bridge tiers implemented across 10 sub-crates. Tier 4 instant intent pool is live on Ethereum Sepolia testnet.

### Added
- `bleep-interop`: all 10 BLEEP Connect sub-crates
- Tier 4: `bleep-connect-layer4-instant` — executor auction, 15s window, 120s timeout, 30% bond slash
- Tier 3: `bleep-connect-layer3-zkproof` — SPHINCS+-bound STARK commitment, 32-intent batches, `GlobalNullifierSet`
- Tier 2: `bleep-connect-layer2-fullnode` — multi-client verification spec
- Tier 1: `bleep-connect-layer1-social` — stakeholder governance recovery path
- `bleep-connect-commitment-chain`: cross-chain commitment anchoring
- `bleep-cli` Sprint 6: governance proposals, AI attestation, ZKP verification, faucet commands

---

## [0.6.0] — Sprint 5 — 2025-05-14

### Highlights

Sprint 5 delivers the **7-tier intent-driven VM** and **6-layer PAT engine** — the execution heart of BLEEP.

### Added
- `bleep-vm` v0.5: 7-tier intent VM (Native / Router / EVM / WASM / STARK / AI-Advised / Cross-Chain)
- `bleep-vm`: unified gas model normalising across EVM, WASM, ZK, Move gas semantics
- `bleep-vm`: deterministic execution sandbox — no filesystem, no network, no random
- `bleep-vm`: `StateDiff` — VM never writes state directly; all mutations via atomic `StateDiff` commit
- `bleep-vm`: circuit breaker per engine (5 failures → 30s backoff)
- `bleep-pat` v2: 6-layer intent-driven PAT engine; `PATRegistry`; `PATGasModel`
- `bleep-economics` Phase 1–4: tokenomics, EIP-1559-style fee market, validator incentives
- `bleep-auth`: salted SHA3-256 credential hashing, JWT sessions, RBAC, rate limiter

---

## [0.5.0] — Sprint 4 — 2025-03-22

### Highlights

Sprint 4 completes the **state layer** — 256-level Sparse Merkle Trie backed by RocksDB with multi-column-family layout and fixed 8,192-byte membership proofs.

### Added
- `bleep-state`: 256-level Sparse Merkle Trie with fixed-size membership and non-membership proofs
- `bleep-state`: RocksDB column families — `audit_log`, `audit_meta`, `nullifier_store` (all `sync=true`)
- `bleep-state`: shard manager and registry; protocol versioning
- `bleep-governance` Phase 2: on-chain governance core; deterministic executor; ZK voting
- `bleep-p2p`: Kademlia DHT (k=20), Plumtree gossip (fanout=8), onion routing

---

## [0.4.0] — Sprint 3 — 2025-02-01

### Highlights

Sprint 3 delivers **multi-mode BFT consensus** — PoS-Normal (primary), Emergency, and Recovery modes with AI-adaptive mode selection.

### Added
- `bleep-consensus`: PoS-BFT primary mode — stake-proportional proposer selection, 3,000ms slots
- `bleep-consensus`: PBFT emergency mode; PoW fallback
- `bleep-consensus`: `ConsensusOrchestrator` with AI-adaptive mode selection (linfa k-NN)
- `bleep-consensus`: `SlashingEngine` — double-sign 33%, equivocation 25%, downtime 0.1%/block
- `bleep-consensus`: `FinalityManager` — irreversible finalisation at >6,667 bps of total stake
- `bleep-consensus`: `EpochManager` — 1,000-block mainnet epochs, 100-block testnet epochs
- `bleep-ai` Phase 3: `AIConstraintValidator` (deterministic rule engine); `DeterministicInferenceEngine` (ONNX, 6 invariants)

---

## [0.3.0] — Sprint 2 — 2024-12-10

### Highlights

Sprint 2 establishes the **cryptographic foundation** — all post-quantum primitives at Security Level 5, with secret key zeroing on drop.

### Added
- `bleep-crypto`: SPHINCS+-SHAKE-256f-simple (FIPS 205, SL5) — 7,856-byte signatures
- `bleep-crypto`: Kyber-1024 / ML-KEM-1024 (FIPS 203, SL5) — 1,568-byte ciphertext
- `bleep-crypto`: AES-256-GCM symmetric encryption
- `bleep-crypto`: SHA3-256 (state, Merkle, block hash, audit log, AI model hash)
- `bleep-crypto`: BLAKE3 (high-throughput content-addressing, Winterfell FRI)
- `bleep-crypto`: `zeroize::Zeroizing<Vec<u8>>` wrapping for all secret key types
- `bleep-core`: `Block`, `ZKTransaction`, mempool, shared types
- Genesis configuration: `config/genesis.json`, `config/mainnet_config.json`, `config/testnet_config.json`

---

## [0.1.0] — Sprint 1 — 2024-10-05

### Highlights

Sprint 1 initialises the **Cargo workspace** and establishes the project structure, constitutional parameters, and initial documentation.

### Added
- 19-crate Cargo workspace with acyclic dependency graph
- Constitutional `const_assert!` parameters: `MAX_SUPPLY`, `MAX_INFLATION_RATE_BPS`, `FEE_BURN_BPS`
- `rust-toolchain.toml` pinning stable Rust channel
- Project documentation: `README.md`, `BUILDING.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE-OF-CONDUCT.md`, `NOTICE`, `LICENSE`
- `.github/` templates: bug report, feature request, CI/CD workflows

---

*BLEEP · Quantum Trust Network · Protocol Version 5*
*© 2026 Muhammad Attahir — Apache 2.0 Licence*
