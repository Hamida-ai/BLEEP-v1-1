# Changelog

All notable changes to the BLEEP blockchain are documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) and the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

> **Licence note:** The core protocol is Apache-2.0. `bleep-vm` is BSL-1.1 (converts to Apache-2.0 on 2028-07-13). `bleep-ai` is MIT. See [`LICENSE`](./LICENSE), [`NOTICE`](./NOTICE), and per-crate `LICENSE` files for full terms.

---

## [Unreleased]

### Added
- Initial public evaluation release of BLEEP V1 on GitHub.

---

## [1.0.0] — Sprint 9 — 2026-04-10

### Highlights
Sprint 9 is the **security hardening and audit-preparation release**. All consensus, state, and cryptographic paths have been subjected to chaos testing, property-based testing, and an internal security audit. The sprint also introduces fuzz targets for the Merkle trie and transaction signer.

### Added

**Security & Auditing**
- `security_audit.rs` in `bleep-consensus`: on-demand `AuditReport` generation with severity-ranked findings (`AuditFinding`, `AuditSummary`, `Severity`, `FindingStatus`).
- `SECURITY_AUDIT_SPRINT9.md` in `docs/`: full internal audit report, findings, and mitigations.
- `THREAT_MODEL.md` in `docs/`: trust boundary map, threat catalogue (11 threat categories), and per-crate audit priorities.
- Fuzz targets in `bleep-state/src/fuzz/`: `fuzz_merkle_insert` and `fuzz_state_apply_tx` (cargo-fuzz).
- Fuzz targets in `bleep-crypto/fuzz/`: transaction signing and Merkle commitment paths.
- `proptest_sprint8.rs` in `bleep-state`: 40+ property-based tests covering state transitions, shard safety, and Merkle trie correctness.

**Integration Tests**
- `tests/sprint9_integration.rs`: end-to-end integration suite covering validator lifecycle, cross-shard 2PC, governance proposals, and RPC endpoints.
- `phase4_ai_integration_tests.rs` in `bleep-ai/tests/`: Phase 4 AI advisory integration tests.

**Chaos Testing**
- `chaos_engine.rs` in `bleep-consensus`: `ChaosEngine` with configurable `ChaosScenario`s (network partition, validator crash, Byzantine vote, clock skew). `ContinuousChaosHarness` for sustained adversarial testing.
- `CHAOS_TESTING.md` in `docs/`: runbook for chaos test execution and result interpretation.

**Documentation**
- `THREAT_MODEL.md`, `SECURITY_AUDIT_SPRINT9.md`, `CI_CD_PIPELINE.md`, `CI_CD_QUICK_REFERENCE.md` added to `docs/`.
- `docs/phase4_shard_recovery.md`: Phase 4 advanced shard recovery orchestration documentation.
- `docs/specs/rpc_api_spec.md`: full RPC endpoint specification (previously a placeholder).
- `docs/specs/state_transition.md`: state transition specification (previously a placeholder).
- `docs/tutorials/build_node.md`, `docs/tutorials/write_contract.md`: complete step-by-step tutorials (previously placeholders).
- `docs/glossary.md`: comprehensive ecosystem glossary (previously a placeholder).
- Per-crate `README.md` files for all 18 workspace crates.
- `CHANGELOG.md` (this file).
- `LICENSE_BSL.md`: rendered BSL-1.1 licence for `bleep-vm`.

### Changed
- `bleep-consensus`: `ConsensusOrchestrator` now surfaces `ConsensusMetrics` in `select_mode()` for deterministic replay.
- `bleep-state`: `StateManager.apply()` upgraded to write batches with explicit fsync for crash safety.
- `bleep-rpc`: rate-limit headers (`X-RateLimit-*`) added to all write endpoint responses.
- `bleep-scheduler`: task timeout enforcement hardened; hung tasks now emit a `warn!` log entry before cancellation.
- `bleep-p2p`: anti-replay nonce cache enlarged from 8k to 64k slots (LRU eviction).

### Fixed
- `cross_shard_2pc.rs`: coordinator no longer deadlocks when all shards abort simultaneously.
- `bleep-crypto/zkp_verification.rs`: batch proof aggregation panic on empty proof list resolved.
- `bleep-rpc`: `/rpc/validator/list` previously returned stale data when a validator was slashed mid-epoch; now reads atomically from `ValidatorRegistry`.

---

## [0.9.0] — Sprint 8 — 2025-12-15

### Highlights
Sprint 8 delivers the **Phase 5 AI-driven protocol evolution layer** in `bleep-governance`, shard performance benchmarking, and the complete economic nervous system (`bleep-economics` Phase 5).

### Added

**AI Protocol Evolution (Phase 5)**
- `bleep-governance`: `governance_engine.rs`, `protocol_rules.rs`, `apip.rs` (Autonomous Protocol Improvement Proposals), `safety_constraints.rs`, `ai_reputation.rs`, `protocol_evolution.rs`, `ai_hooks.rs`, `invariant_monitoring.rs`, `governance_voting.rs`, `deterministic_activation.rs`.
- `phase5_integration_tests.rs` and `phase5_comprehensive_tests.rs` in `bleep-governance`.

**Economics (Phase 5)**
- `bleep-economics`: `oracle_bridge.rs` (trust-minimised price aggregation), `game_theory.rs` (mechanism design proofs), `runtime.rs` (scheduler hooks).
- `ALLOC_*` constants for all genesis allocation buckets published as public API.
- `FEE_BURN_BPS`, `FEE_VALIDATOR_REWARD_BPS`, `FEE_TREASURY_BPS` constants.

**Consensus Performance**
- `performance_bench.rs` in `bleep-consensus`: `PerformanceBenchmark`, `BenchmarkResult`, `TpsWindow`, `TARGET_TPS`, `BENCHMARK_DURATION_SECS`.
- `shard_coordinator.rs` in `bleep-consensus`: cross-shard TPS stress testing harness.

**Shard AI Extension**
- `shard_ai_extension.rs` in `bleep-state`: AI advisory hooks for shard lifecycle decisions.
- `cross_shard_ai_hooks.rs`: AI routing recommendations for cross-shard transactions.

**Pre-testnet**
- Pre-testnet faucet drip amount changed from 1,000 BLEEP → **10 BLEEP** per address per 24 hours.
- Automatic 10 BLEEP credit on new wallet creation via `bleep-cli wallet create`.
- `AccountState::pretestnet_default()` provides 10 BLEEP to new accounts when DB record is absent.
- `PRETESTNET_FAUCET_CHANGES.md` documents all faucet changes.

### Changed
- `bleep-scheduler`: 20 built-in tasks across 7 categories (up from 14 across 5).
- `bleep-indexer`: added `CrossShardIndex` and `AIEventIndex`.
- `bleep-p2p`: onion router enabled as opt-in (`enable_onion = true` in config).
- `bleep-auth`: `AuditLog` upgraded to Merkle-chained entries for tamper-evidence.

### Fixed
- `bleep-state/shard_rollback.rs`: rollback past epoch boundaries now correctly restores shard validator assignments.
- `bleep-economics/fee_market.rs`: base fee calculation no longer underflows on empty blocks.

---

## [0.8.0] — Sprint 7 — 2025-09-20

### Highlights
Sprint 7 delivers the **Phase 4 self-healing and advanced recovery** layer, cross-shard 2PC, and the complete `bleep-vm` BSL-1.1 release.

### Added

**Phase 4 Self-Healing**
- `advanced_fault_detector.rs`, `self_healing_orchestrator.rs`, `phase4_recovery_orchestrator.rs`, `phase4_safety_invariants.rs`, `phase4_integration_tests.rs` in `bleep-state`.
- `rollback_engine.rs`, `snapshot_engine.rs` in `bleep-state`.

**Cross-Shard 2PC**
- `cross_shard_2pc.rs`, `cross_shard_locking.rs`, `cross_shard_recovery.rs`, `cross_shard_safety_invariants.rs`, `cross_shard_transaction.rs` in `bleep-state`.

**bleep-vm v0.5**
- 7-layer intent-driven VM with EVM (revm), WASM (Wasmer 4.2 + Cranelift), and ZK (ark-groth16) engines.
- `vm_router.rs`: circuit breaker (5 failures → 30s backoff), per-chain VM overrides, routing metrics.
- `sandbox.rs`: WASM memory limit (16 MB), call stack depth (1,024 frames), host API whitelist.
- `state_transition.rs`: `StateDiff` with `commitment_hash()` and `simulate()`.
- `gas_model.rs`: unified gas normalisation across all engines.
- BSL-1.1 licence applied; Change Date: **2028-07-13**.
- `bleep-vm` `README.md` published.

**bleep-interop v0.1**
- All 10 BLEEP Connect sub-crates scaffolded and integrated.
- Layer 4 instant intent pool and executor node binary (`bleep-executor`).
- `nullifier_store.rs` for double-spend prevention.

**AI Phase 4**
- `feature_extractor.rs`, `ai_decision_module.rs`, `governance_integration.rs` in `bleep-ai`.
- `phase3_tests.rs`, `phase3_unit_tests.rs`, `phase4_ai_integration_tests.rs`.
- `bin/phase3_verify.rs`: standalone Phase 3 verification binary.

### Changed
- `bleep-consensus`: `GossipBridge` added for clean consensus ↔ P2P separation.
- `bleep-auth`: `ValidatorBinding` now uses Kyber-1024 (upgraded from Kyber-768).
- `bleep-rpc`: BLEEP Connect endpoints added (`/rpc/connect/*`).

---

## [0.7.0] — Sprint 6 — 2025-07-01

### Highlights
Sprint 6 delivers validator staking management in the CLI, the complete `bleep-auth` crate, and the governance Phase 4 constitutional layer.

### Added

**bleep-cli Sprint 6**
- `validator stake`, `validator unstake`, `validator list`, `validator status` commands.
- `faucet request`, `faucet status` commands.
- `ai status`, `ai recommend` commands.
- `governance propose`, `governance vote`, `governance list`, `governance status` commands.

**bleep-auth**
- Full production release: `credentials`, `session`, `rbac`, `identity`, `validator_binding`, `audit`, `rate_limiter`.

**Governance Phase 4**
- `constitution.rs`, `zk_voting.rs`, `proposal_lifecycle.rs`, `forkless_upgrades.rs`, `governance_binding.rs` in `bleep-governance`.
- `phase4_governance_tests.rs`.

**Shard Lifecycle (Phase 2)**
- `shard_lifecycle.rs`, `shard_epoch_binding.rs`, `shard_checkpoint.rs`, `shard_isolation.rs`, `shard_fault_detection.rs`, `shard_healing.rs`, `shard_validator_assignment.rs`, `shard_validator_slashing.rs` in `bleep-state`.
- `phase2_full_integration_tests.rs`, `phase2_integration_tests.rs`, `phase2_safety_invariants.rs`.

**bleep-zkp**
- STARK block validity circuit (`BlockValidityAir`) using Winterfell framework.
- `pq_proofs.rs`: post-quantum ZKP constructions.

### Changed
- `bleep-rpc`: `/rpc/validator/*` and `/rpc/oracle/*` endpoint group added.
- `bleep-consensus`: slashing engine activated (previously scaffolded but inactive).

---

## [0.6.0] — Sprint 5 — 2025-04-15

### Highlights
Sprint 5 completes the `bleep-pat` engine v2, the `bleep-economics` Phase 1–4 tokenomics, and the P2P networking stack.

### Added
- `bleep-pat` v2: 6-layer intent-driven engine with `PATEngine`, `PATRegistry`, `PATGasModel`, `PATStateDiff`.
- `bleep-economics`: `tokenomics.rs`, `distribution.rs`, `fee_market.rs`, `validator_incentives.rs`.
- `bleep-p2p`: `KademliaDHT`, `PeerManager`, `GossipProtocol` (Plumtree), `OnionRouter`, `MessageProtocol`, `QuantumCrypto`.
- `bleep-telemetry`: `metrics.rs`, `load_balancer.rs`.
- `bleep-scheduler`: initial 14 built-in tasks across 5 categories.
- `bleep-indexer`: `BlockIndex`, `TxIndex`, `AccountIndex`, `GovernanceIndex`, `ValidatorIndex`, `ShardIndex`.

### Changed
- `bleep-rpc`: PAT endpoints (`/rpc/pat/*`) added.
- `bleep-core`: `mempool_bridge.rs` added for async mempool ↔ consensus handoff.

---

## [0.5.0] — Sprint 4 — 2025-02-01

### Highlights
Sprint 4 delivers the `bleep-consensus` multi-mode engine, `bleep-state` Merkle trie, and initial shard management.

### Added
- `bleep-consensus`: `pos_engine.rs`, `pbft_engine.rs`, `pow_engine.rs`, `orchestrator.rs`, `epoch.rs`, `validator_identity.rs`, `slashing_engine.rs`, `finality.rs`, `block_producer.rs`, `ai_adaptive_logic.rs`.
- `bleep-state`: `state_manager.rs`, `state_merkle.rs`, `state_storage.rs`, `shard_manager.rs`, `shard_registry.rs`, `protocol_versioning.rs`.
- `bleep-wallet-core`: `wallet_core.rs` with Falcon and SPHINCS+ key support.

### Changed
- `bleep-crypto`: Falcon signature support added alongside Ed25519.
- `bleep-core`: `ZKTransaction` type replaces plain `Transaction`.

---

## [0.4.0] — Sprint 3 — 2024-11-10

### Highlights
Sprint 3 delivers `bleep-governance` Phase 2 (on-chain governance core) and `bleep-crypto` ZKP module.

### Added
- `bleep-governance`: `governance_core.rs`, `deterministic_executor.rs`.
- `bleep-crypto`: `zkp_verification.rs` with Groth16 batch proofs and Bulletproofs.
- `bleep-ai` Phase 3: `deterministic_inference.rs`, `ai_attestation.rs`, `ai_constraint_validator.rs`, `ai_consensus_integration.rs`, `ai_feedback_loop.rs`, `ai_proposal_types.rs`.

---

## [0.3.0] — Sprint 2 — 2024-09-01

### Highlights
Sprint 2 delivers `bleep-core` protocol invariants and `bleep-vm` initial scaffolding.

### Added
- `bleep-core`: `protocol_invariants.rs`, `invariant_enforcement.rs`, `proof_of_identity.rs`, `anti_asset_loss.rs`, `decision_attestation.rs`.
- `bleep-vm`: initial scaffold (EVM engine stub, intent layer design).
- `bleep-rpc`: initial `warp`-based HTTP server with `/rpc/state` and `/rpc/proof` endpoints.

---

## [0.2.0] — Sprint 1 — 2024-06-15

### Highlights
Sprint 1 establishes the workspace, core data structures, and cryptographic foundation.

### Added
- Cargo workspace with all 19 crate members declared.
- `bleep-crypto`: `pq_crypto.rs` (Kyber, SPHINCS+), `bip39.rs`, `tx_signer.rs`, `merkletree.rs`, `quantum_secure.rs`.
- `bleep-core`: `block.rs`, `transaction.rs`, `blockchain.rs`, `mempool.rs`, `transaction_pool.rs`, `networking.rs`.
- `bleep-ai`: `feature_extractor.rs` (Phase 1 telemetry ingestion).
- `config/genesis.json`, `config/testnet_config.json`, `config/mainnet_config.json`.
- `rust-toolchain.toml` pinning the stable Rust version.
- `README.md`, `BUILDING.md`, `CONTRIBUTING.md`, `CODE-OF-CONDUCT.md`, `SECURITY.md`, `NOTICE`, `LICENSE` (Apache-2.0).
- `.github/ISSUE_TEMPLATE/bug_report.md`, `feature_request.md`.

---

## [0.1.0] — Genesis — 2024-04-01

### Added
- Repository initialised.
- Vision, architecture, and project charter established.
- BLEEP name, brand, and initial roadmap defined.

---

[Unreleased]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.9.0...v1.0.0
[0.9.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/BleepEcosystem/BLEEP-V1/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/BleepEcosystem/BLEEP-V1/releases/tag/v0.1.0
