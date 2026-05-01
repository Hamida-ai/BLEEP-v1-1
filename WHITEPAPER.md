# BLEEP: Quantum Trust Network

**A Post-Quantum Cryptographic Foundation for Secure Decentralized Execution**

**Muhammad Attahir — April 2026**
**Protocol Version 1**

---

> *This document is provided for informational purposes only. It does not constitute financial advice, investment advice, or an offer to sell securities or digital assets. All protocol parameters and source references correspond to Protocol Version 1.*

---

## Abstract

Existing decentralized systems derive their security from cryptographic assumptions — integer factorization hardness, discrete logarithm intractability, and elliptic-curve group structure — that are broken in polynomial time by Shor's algorithm on a sufficiently capable fault-tolerant quantum processor. Every transaction record on such a system constitutes a long-lived liability: an adversary may archive signed transactions and public keys today and apply quantum decryption retroactively when hardware of sufficient scale becomes available.

This paper presents BLEEP, a **Quantum Trust Network**: a distributed execution protocol in which trust is derived exclusively from post-quantum cryptographic guarantees. BLEEP enforces transaction validity, node identity, network message authentication, and zero-knowledge proof verification using NIST-finalized post-quantum primitives at Security Level 5, while maintaining deterministic execution, Byzantine fault-tolerant consensus, and verifiable state transitions. Block validity proofs and cross-chain bridge proofs use Winterfell STARK proofs — transparent, hash-based constructions requiring no trusted setup ceremony. No classical public-key primitive or pairing-based construction is present on any cryptographically sensitive path.

The protocol separates cryptography, execution, consensus, networking, and state management into modular, independently auditable components with acyclic dependency graphs. Constitutional protocol parameters are enforced via compile-time assertions, providing machine-verified invariants that no governance vote or software upgrade can override.

---

## Contents

1. Introduction
2. Background
3. System Overview
4. Design Principles
5. Architecture
6. Cryptographic Model
7. Execution Model
8. Networking and Consensus
9. Governance
10. Cross-Chain Interoperability
11. Economics and Tokenomics
12. AI Advisory and Inference Engine
13. Scalability under Deterministic Constraints
14. Use Cases
15. Target Users
16. Security Considerations
17. Limitations
18. Future Work
19. Conclusion
- References
- Appendix A: Protocol Parameters

---

## 1. Introduction

### 1.1 The Harvest-Now, Decrypt-Later Problem

Shor's algorithm, executed on a sufficiently large fault-tolerant quantum processor, reduces integer factorization and discrete-logarithm computation to polynomial time [1]. This breaks RSA, finite-field Diffie-Hellman, and all elliptic-curve schemes, including the secp256k1 curve used by Bitcoin and Ethereum for transaction signing and address derivation.

The operationally significant threat is not the existence of such a processor today — current quantum hardware is orders of magnitude below the qubit count required [2]. The threat is **archival**. Every transaction broadcast on a classical blockchain is a permanent public record. An adversary with sufficient storage capacity can archive ciphertexts and signed transactions now and apply quantum decryption retroactively when capable hardware becomes available. This is the **harvest-now, decrypt-later** threat model [3]. The historical record of a classical blockchain is a cryptographic liability that grows monotonically with time.

### 1.2 The Migration Problem

The conventional response is a planned migration: upgrade the signature scheme and key encapsulation mechanism before quantum hardware reaches the required scale. In practice, coordinated cryptographic migrations of deployed distributed systems are extremely difficult. Validators, wallets, bridges, indexers, relayers, and the ecosystem of tooling must upgrade simultaneously. History demonstrates that this does not occur cleanly under time pressure, even with years of advance notice [4].

A protocol that launches with classical cryptography and plans a post-quantum migration inherits the coordination problem. A protocol that is post-quantum from genesis avoids it. BLEEP is designed on the premise that the correct time to establish post-quantum cryptographic foundations is before the protocol accumulates economic value and ecosystem dependencies, not after.

### 1.3 Design Goals

BLEEP is designed to achieve the following properties, stated precisely to enable verification:

- **Post-quantum security at Security Level 5** on all signature, key-encapsulation, and zero-knowledge proof paths, with no classical fallback.
- **Deterministic protocol execution:** consensus transitions, epoch boundaries, recovery actions, and all consensus-critical computations must produce byte-identical outputs on every honest node running the same software version.
- **Constitutional parameter immutability:** token supply cap, minimum finality threshold, maximum inflation rate, and fee burn floor are enforced by compile-time assertions and cannot be altered by governance vote or software upgrade.
- **Modular separation of concerns:** cryptographic primitives, execution engines, networking, consensus, and state management are isolated in distinct crates with acyclic dependency graphs.
- **Trustless cross-chain verification** through a tiered bridge architecture requiring no permanently privileged operator.
- **Auditability by default:** every authentication event, slashing action, governance execution, and recovery operation is committed to a tamper-evident, restart-persistent audit log.

### 1.4 Scope of This Paper

This paper describes the BLEEP protocol at Protocol Version 4. Section 2 provides background. Section 3 defines the Quantum Trust Network. Sections 4 through 13 describe the principal subsystems, scalability, and security. Sections 14 and 15 describe use cases and target users. Section 16 addresses security considerations. Section 17 acknowledges limitations. Sections 18 and 19 cover future work and conclusions.

---

## 2. Background

### 2.1 Post-Quantum Cryptography

Post-quantum cryptography (PQC) refers to constructions believed to resist attacks by both classical and quantum computers. Grover's algorithm provides a quadratic speedup against unstructured search, reducing a 256-bit hash function to approximately 128 bits of quantum security — a weakening, not a break, addressed by larger output sizes. Shor's algorithm provides an exponential speedup against problems based on integer factorization or discrete logarithm, rendering RSA and all elliptic-curve schemes insecure regardless of parameter size [5].

In 2024, NIST finalized its first post-quantum cryptography standards [6]. The two standards used in BLEEP are: FIPS 205 (SLH-DSA, based on SPHINCS+), a stateless hash-based signature scheme whose security reduces to the one-wayness of the underlying hash function; and FIPS 203 (ML-KEM, based on Kyber), a lattice-based key encapsulation mechanism whose security reduces to the hardness of the Module Learning with Errors (MLWE) problem.

### 2.2 Byzantine Fault Tolerance

A Byzantine fault-tolerant (BFT) consensus protocol operates correctly when up to f < n/3 of n participants behave arbitrarily, including sending contradictory messages [7]. BLEEP uses a proof-of-stake BFT protocol where stake weight replaces uniform vote weight. Finality requires more than two-thirds of total staked supply to commit — not two-thirds of participant count.

### 2.3 Zero-Knowledge Proof Systems

A zero-knowledge proof (ZKP) allows a prover to convince a verifier of a statement's truth without revealing any information beyond the truth of the statement [8]. BLEEP uses Winterfell STARK proofs for block validity proofs and cross-chain intent batching. STARKs are constructed over hash functions rather than elliptic curves, providing two critical properties: **transparency** (no trusted setup ceremony is required) and **post-quantum security** (security reduces to collision resistance of the underlying hash function, not to algebraic assumptions broken by Shor's algorithm).

### 2.4 Sharding and Two-Phase Commit

Sharding partitions global state into disjoint subsets processed in parallel by different validator subsets, increasing throughput at the cost of cross-shard coordination. Cross-shard transactions require an atomic commitment protocol. BLEEP uses two-phase commit (2PC), with coordinator assignment derived deterministically from the transaction hash to eliminate privileged coordinator election.

---

## 3. System Overview

### 3.1 Definition: Quantum Trust Network

> **QUANTUM TRUST NETWORK**
>
> A Quantum Trust Network is a distributed execution system in which transaction validity, node identity, network message authentication, and zero-knowledge proof verification are enforced exclusively using cryptographic primitives believed to resist attacks by both classical probabilistic polynomial-time (PPT) adversaries and quantum polynomial-time (QPT) adversaries equipped with Shor's algorithm, as formalized in NIST post-quantum cryptography standards FIPS 203 and FIPS 205, and in hash-based transparent proof systems.

This definition has four operational consequences verifiable against the codebase:

1. Every transaction must carry a SPHINCS+-SHAKE-256f-simple signature (FIPS 205, Security Level 5).
2. Every secure channel must be established via Kyber-1024/ML-KEM-1024 (FIPS 203, Security Level 5).
3. No path determining transaction validity or network membership may fall back to a classical construction.
4. Block validity proofs and cross-chain bridge proofs are generated and verified using Winterfell STARK proofs — post-quantum secure and requiring no trusted setup.

### 3.2 High-Level Architecture

| Subsystem | Primary Crates | Responsibility |
|-----------|---------------|----------------|
| Cryptographic | `bleep-crypto`, `bleep-zkp`, `bleep-wallet-core` | Post-quantum signatures, key encapsulation, STARK proofs, key lifecycle |
| Consensus | `bleep-consensus`, `bleep-scheduler` | Block production, finality, slashing, epoch management, fault recovery |
| State and storage | `bleep-state`, `bleep-indexer` | Account ledger, Sparse Merkle Trie, RocksDB column families, shard lifecycle |
| Execution environment | `bleep-vm`, `bleep-pat`, `bleep-ai` | Modular execution engines, transaction routing, advisory inference |
| Peer-to-peer network | `bleep-p2p`, `bleep-rpc`, `bleep-auth` | Node discovery, message propagation, onion routing, authentication |

*Table 1 — Principal subsystems and primary crates*

### 3.3 Node Startup and Readiness

A node follows a 16-step dependency-ordered startup sequence. Post-quantum key pairs are generated first. `StateManager` opens its RocksDB instance — including nullifier store and audit log column families — before any block production logic activates. The Winterfell STARK proof system is ready immediately at startup; no structured reference string is fetched or verified. The node emits a readiness signal only after all 46 RPC endpoints are confirmed active. Any startup failure halts the node rather than leaving it partially initialized.

---

## 4. Design Principles

### 4.1 Safety over Liveliness

Where a choice must be made between safety and liveliness, BLEEP chooses safety. Finality requires a supermajority of stake. A node that cannot make progress safely halts rather than diverging. This follows the classical result of Fischer, Lynch, and Paterson [9]: no deterministic protocol can simultaneously guarantee safety, liveliness, and fault tolerance under asynchrony. BLEEP sacrifices liveliness under adversarial conditions.

### 4.2 Determinism as a Protocol Invariant

Every computation influencing chain state must produce byte-identical outputs on every honest node running the same software version. Non-determinism on any of these paths is classified as a protocol bug. STARK proof generation satisfies this invariant: given identical public inputs and witnesses, the Winterfell prover produces a deterministically verifiable proof on all correct nodes. This invariant also constrains AI-assisted components on consensus-critical paths: any deployed model must produce byte-identical outputs given identical inputs and must use deterministic feature extraction.

### 4.3 Constitutional Immutability

Four parameters cannot be altered by any governance vote or software upgrade:

| Parameter | Value | Enforcement |
|-----------|-------|-------------|
| Maximum supply | 200,000,000 BLEEP | `MAX_SUPPLY` compile-time const-assertion |
| Minimum finality threshold | 6,667 bps | `FinalityManager` |
| Maximum per-epoch inflation | 500 bps (5%) | `MAX_INFLATION_RATE_BPS` const-assertion |
| Fee burn floor | 2,500 bps | `distribution.rs` compile-time assertion |

A code change that violates a constitutional assertion does not compile.

### 4.4 Separation of Concerns

Each of the 19 workspace crates has a single defined responsibility. The inter-crate dependency graph is acyclic, enforced at build time. The cryptographic subsystem exposes only `sign`, `verify`, `encapsulate`, `decapsulate`, `prove`, and `verify_proof` operations. A change to the execution environment cannot inadvertently modify cryptographic behavior; a vulnerability in networking cannot directly access private key material.

### 4.5 Auditability by Default

Every security-relevant event is written to a tamper-evident audit log backed by RocksDB with synchronous writes (`sync=true`). Log entries are SHA3-256 Merkle-chained. Mutating any stored entry causes chain verification to return false. The log survives node restarts: on startup, the chain tip and sequence counter are restored from a dedicated column family and the most recent 10,000 entries warm the in-memory cache.

---

## 5. Architecture

### 5.1 Cryptographic Subsystem

The cryptographic subsystem (`bleep-crypto`, `bleep-zkp`) is the root dependency of the protocol. All post-quantum operations are performed here. The subsystem provides: SPHINCS+-SHAKE-256f-simple signatures via `pqcrypto-sphincsplus`; Kyber-1024/ML-KEM-1024 key encapsulation via `pqcrypto-kyber`; AES-256-GCM for symmetric encryption of onion routing hops and wallet key storage; SHA3-256 for state commitments, Merkle hashing, block hashing, audit log chaining, and AI model binary hashing; BLAKE3 for high-throughput content-addressing; and Winterfell STARK proofs for block validity and cross-chain bridge verification. Secret key types are wrapped in `zeroize::Zeroizing<Vec<u8>>`, zeroed on drop before deallocation.

### 5.2 Consensus Subsystem

The consensus subsystem (`bleep-consensus`) implements a proof-of-stake BFT protocol in three modes selected deterministically from validator liveliness. PoS-Normal is primary: block production at 3,000 ms intervals with stake-proportional proposer selection. Emergency mode activates when fewer than 67% of validators are responsive. Recovery mode re-anchors to the most recent finalized checkpoint after long-range partitions. `BlockProducer` selects up to 4,096 transactions per slot by fee, computes the Sparse Merkle Trie root, generates a Winterfell STARK block validity proof, and signs the completed block with SPHINCS+ before broadcasting.

### 5.3 State and Storage Subsystem

Account state is maintained as a 256-level Sparse Merkle Trie (SMT) backed by RocksDB. The trie root appears in every block header. Membership and non-membership proofs are fixed-size at 8,192 bytes regardless of account count. Three RocksDB column families serve security-critical functions: `audit_log`, `audit_meta`, and `nullifier_store` (`WriteBatch sync=true`).

### 5.4 Execution Environment

| Tier | Engine | Scope | Gas model |
|------|--------|-------|-----------|
| 1 | Native | BLEEP Transfer, stake, unstake, governance vote | None |
| 2 | Router | Engine selection, gas validation, circuit breakers | Validation only |
| 3 | EVM (SputnikVM) | Ethereum-compatible contract execution | Ethereum gas semantics |
| 4 | WebAssembly (Wasmi) | WASM contract execution | Configurable fuel metering |
| 5 | STARK Proof | Zero-knowledge execution, public input verification | Fixed cost per verifier op |
| 6 | AI-Advised | Constraint validation before execution (advisory; off-chain) | Deterministic; no gas |
| 7 | Cross-Chain | BLEEP Connect Tier 4 instant intent dispatch | Protocol fee in basis points |

*Table 2 — Execution engine dispatch (source: `bleep-vm/src/router/vm_router.rs`)*

### 5.5 Peer-to-Peer Network

The P2P network (`bleep-p2p`) uses Kademlia DHT with k=20. Peer IDs are deterministic hashes of SPHINCS+ public keys. All inter-node messages are SPHINCS+-signed; unauthenticated messages are dropped before payload processing. A 2 MiB size gate is enforced at the receive boundary before any deserialization. Onion routing provides multi-hop anonymised delivery using AES-256-GCM keyed from Kyber-1024 per-hop shared secrets.

### 5.6 Identity and Access Control

`bleep-auth` provides credential hashing (salted SHA3-256, constant-time comparison), JWT session management (HS256 with Shannon entropy gate ≥ 3.5 bits/byte), role-based access control (O(1) DashMap permission check), Kyber-1024 validator binding, the tamper-evident audit log, and per-identity rate limiting.

---

## 6. Cryptographic Model

### 6.1 Algorithm Selection and Rationale

| Property | SPHINCS+-SHAKE-256f-simple | Kyber-1024 (ML-KEM-1024) |
|----------|---------------------------|--------------------------|
| NIST standard | FIPS 205 (SLH-DSA) | FIPS 203 (ML-KEM) |
| Role | Transaction signing, block signing, P2P authentication | Validator binding, peer KEM, wallet key management |
| Security assumption | One-wayness of SHAKE-256 (hash-based) | Hardness of Module-LWE (lattice-based) |
| NIST security level | Level 5 (≥256-bit post-quantum) | Level 5 (≥256-bit post-quantum) |
| Public key | 32 bytes | 1,568 bytes |
| Secret key | 64 bytes (`Zeroizing<>` on drop) | 3,168 bytes (`Zeroizing<>` on drop) |
| Output | 7,856-byte detached signature | 1,568-byte ciphertext + 32-byte shared secret |

*Table 3 — Post-quantum algorithm parameters (source: `bleep-crypto/src/pq_crypto.rs`)*

SPHINCS+ is selected for its conservative security assumptions: security reduces to the one-wayness of the hash function with no reliance on algebraic structure. The tradeoff is large signatures (7,856 bytes at Level 5).

### 6.2 STARK Proof System

BLEEP uses Winterfell STARK proofs for block validity and cross-chain bridge verification. STARKs provide **transparency** (no structured reference string or trusted setup ceremony is required; any party can generate or verify proofs) and **post-quantum security** (security reduces to the collision resistance of Blake3 and SHA3-256).

The `BlockValidityAir` circuit operates over Winterfell's 128-bit prime field and proves over five public inputs: `block_index`, `epoch_id`, `tx_count`, `merkle_root_hash`, and `validator_pk_hash`. Private witnesses are `block_hash` and `sk_seed`. Verification requires only the public inputs and the Winterfell verifier library — no pre-existing key material.

### 6.3 Key Material Lifecycle

Secret keys are wrapped in `zeroize::Zeroizing<Vec<u8>>` from allocation to deallocation. The `Zeroize` derive macro zeros the backing allocation before the allocator reclaims it, regardless of whether the key is dropped normally or through stack unwinding.

### 6.4 Hash Functions

SHA3-256 handles state commitments, Merkle node hashing, block hashing, audit log chaining, AI model binary hashing, and identity proof path computation. BLAKE3 handles high-throughput indexer content-addressing and Winterfell FRI commitment hashing. Grover's algorithm reduces quantum security of these functions from 256 bits to approximately 128 bits — a weakening accepted at Security Level 5.

### 6.5 The Post-Quantum Boundary

All operations within the boundary are post-quantum secure: transaction signing (SPHINCS+), block signing (SPHINCS+), P2P message authentication (SPHINCS+), key encapsulation (Kyber-1024), block validity proofs (Winterfell STARK), cross-chain bridge proofs (SPHINCS+-bound proof transcripts), and identity proofs (SHA3-256 Merkle paths over `SparseMerkleTrie`). No classical public-key primitive or pairing-based construction is present on any cryptographically sensitive path.

---

## 7. Execution Model

### 7.1 State Transition Semantics

> **STATE TRANSITION FUNCTION**
>
> Let S_t denote the complete protocol state at block index t, and let T = (t₁, t₂, …, tₙ) be the canonically ordered sequence of validated transactions in block t. The protocol defines a deterministic total function F such that S_{t+1} = F(S_t, T). Given identical S_t and identical T, every correct protocol implementation produces identical S_{t+1}, including the Sparse Merkle Trie root commitment recorded in the block header.

### 7.2 Transaction Lifecycle

A transaction enters through `POST /rpc/tx/submit` or P2P mempool gossip. The mempool applies four sequential filters: nonce validity, balance sufficiency, minimum base fee, and SPHINCS+ signature verification. `BlockProducer` selects transactions by fee in descending order up to 4,096. A Winterfell STARK `BlockValidityProof` is generated over the completed block before broadcast.

### 7.3 State Transitions

Every state transition is applied atomically. The balance check-and-debit uses a RocksDB compare-and-swap loop with up to three retries, eliminating time-of-check-to-time-of-use races. The supply invariant — circulating supply ≤ 200,000,000 BLEEP — is verified at every epoch boundary. A violation halts the node.

### 7.4 Block Validity Proofs

`BlockValidityCircuit` generates STARK proof constraints proving: (a) the block hash is SHA3-256 of its fields; (b) the proposer knows the secret key whose hash equals `validator_pk_hash`; (c) the epoch ID is consistent with block index and `blocks_per_epoch`; (d) the SMT root commitment is non-zero. Both STARK proof and SPHINCS+ block signature are required for a valid block.

### 7.5 AI Advisory Components

Two AI-assisted components exist in the codebase, neither of which participates in block production, consensus voting, or any state-modifying operation without a prior governance vote. `AIConstraintValidator` (Phase 3) is a deterministic rule engine. `DeterministicInferenceEngine` (Phase 4) is an ONNX-based runtime enforcing six invariants including SHA3-256 model hash verification and CPU-only execution.

---

## 8. Networking and Consensus

### 8.1 Validator Model and Fault Assumptions

> **VALIDATOR SET AND FAULT MODEL**
>
> Let V = {v₁, …, vₙ} be the active validator set at epoch e. Each vᵢ carries a SPHINCS+ verification key vkᵢ, a Kyber-1024 encapsulation key ekᵢ, and a stake sᵢ in microBLEEP. Total staked supply S = Σ(sᵢ). Safety is guaranteed when Byzantine stake f < S/3.

Network model is partial synchrony; safety holds under full asynchrony while liveness requires eventual bounded message delivery. The adversary controls at most f < S/3 of staked supply. Clocks are assumed synchronised within NTP drift tolerance (warn >1 s, halt >30 s).

### 8.2 Message Propagation and Peer Trust

Blocks and transactions propagate via epidemic gossip with fanout 8. `PeerScoring` computes a composite trust score in [0.0, 100.0] from success ratio, message rate, latency, and diversity components. Scores decay at 0.99x per 300 seconds. Peers below 40 are excluded from gossip relay; below 55 from onion routing relay.

### 8.3 Consensus Protocol Flow

1. **Proposer selection:** at each 3,000 ms slot boundary, a validator is selected with probability proportional to stake fraction.
2. **Block proposal:** the proposer assembles a block, generates a Winterfell STARK proof, signs with SPHINCS+, and broadcasts.
3. **Block validation:** each validator independently verifies the STARK proof, the SPHINCS+ signature, and the SMT root transition.
4. **Vote:** accepting validators broadcast a SPHINCS+-signed prevote, then a signed precommit.
5. **Finalisation:** a block is finalised when precommits representing more than 6,667 basis points of S are received. Finalisation is irreversible.
6. **Epoch transition:** every 1,000 blocks (mainnet) / 100 blocks (pre-testnet), the `epoch_advance` task rotates the validator set, distributes rewards, and resets slashing counters.

### 8.4 Finality Guarantees

Finalisation is not probabilistic and does not use a challenge window. A block finalised by more than 6,667 basis points of S is permanent. The adversarial test suite confirms: `LongRangeReorg(10)` and `LongRangeReorg(50)` were each rejected at `FinalityManager` across the 72-hour run.

### 8.5 Slashing

| Violation | Penalty | Source |
|-----------|---------|--------|
| Double-sign | 33% of stake burned; tombstoned | `double_signing_penalty: 0.33` |
| Equivocation | 25% of stake burned | `equivocation_penalty: 0.25` |
| Downtime | 0.1% per consecutive missed block | `downtime_penalty_per_block` |
| Tier 4 bridge executor timeout | 30% of executor bond | `EXECUTION_TIMEOUT = 120 s` |

*Table 4 — Slashing parameters (source: `bleep-consensus/src/slashing_engine.rs`)*

### 8.6 Governance in Consensus

`LiveGovernanceEngine` processes proposals through: Submit → `AIConstraintValidator` pre-flight → Active → Tally → Execute → Record. `ZKVotingEngine` provides privacy-preserving stake-weighted voting. `ForklessUpgradeEngine` activates hash-committed upgrade payloads at epoch boundaries only.

---

## 9. Governance

### 9.1 Proposal Lifecycle

`LiveGovernanceEngine` processes typed proposals through a six-stage lifecycle: Submit → `AIConstraintValidator` pre-flight → Active → Tally → Execute → Record. A proposal that would set `MaxInflationBps` above 500 is rejected at the pre-flight stage and never reaches a vote.

| Parameter | Pre-testnet value | Notes |
|-----------|----------------|-------|
| `voting_period_blocks` | 1,000 blocks (~50 min) | At 3-second block time |
| `quorum_bps` | 1,000 bps (10%) | Minimum stake participation |
| `pass_threshold_bps` | 6,667 bps (66.67%) | Yes votes required of participating stake |
| `veto_threshold_bps` | 3,333 bps (33.33%) | Veto votes required to block passage |
| `min_deposit` | 10,000 BLEEP | Minimum deposit to submit a proposal |

*Table 5 — LiveGovernanceEngine configuration*

### 9.2 Zero-Knowledge Voting

`ZKVotingEngine` provides privacy-preserving stake-weighted voting. Three voter roles: Validator (1.0×), Delegator (0.5×), and Community token holder (0.1×). Votes are encrypted in `EncryptedBallot` structs. `VoteCommitment`-based double-vote prevention and nonce-based replay resistance are enforced at the voting engine.

### 9.3 Constitutional Constraints

- **Maximum token supply:** 200,000,000 BLEEP — `MAX_SUPPLY` compile-time const-assertion.
- **Minimum finality threshold:** 6,667 basis points — enforced at `FinalityManager`.
- **Maximum per-epoch inflation:** 500 basis points — `MAX_INFLATION_RATE_BPS` const-assertion.
- **Fee burn floor** — compile-time assertion in `distribution.rs`.

### 9.4 Forkless Protocol Upgrades

`ForklessUpgradeEngine` manages hash-committed, deterministic protocol upgrades activating at epoch boundaries only. `Version.is_valid_upgrade()` enforces monotonic version progression; a version mismatch halts the chain. Partial upgrades are rejected atomically.

### 9.5 Live Governance Record

`proposal-testnet-001` reduced `FeeBurnBps` from 2,500 to 2,000 and completed the full lifecycle in a pre-testnet pilot: `AIConstraintValidator` pre-flight, ZK vote casting by participating validators, quorum check at 70% stake participation, constitutional validation, on-chain execution at block 1,105, and event recording.

---

## 10. Cross-Chain Interoperability

BLEEP Connect is a four-tier cross-chain bridge architecture implemented across ten sub-crates within `bleep-interop`. Each tier provides a different latency and security tradeoff. No tier requires a permanently privileged operator or a trusted multisig key set.

### 10.1 Bridge Tier Overview

| Tier | Protocol | Latency | Security basis | Status |
|------|---------|---------|----------------|--------|
| 4 — Instant | Executor auction + escrow | 200 ms – 1 s | Economic: 30% executor bond slashed on timeout | Live — Ethereum Sepolia |
| 3 — ZK Proof | SPHINCS+-bound STARK commitment | 10 – 30 s | Cryptographic: PQ-secure; zero trusted operators | Live — Ethereum Sepolia |
| 2 — Full-Node | Multi-client verification | Hours | 90% consensus across ≥3 independent nodes; optional TEE | Implemented; mainnet target |
| 1 — Social | Stakeholder governance | 7 days / 24 h (emergency) | Full governance consensus | Implemented; mainnet target |

*Table 6 — BLEEP Connect bridge tiers*

### 10.2 Tier 4: Instant Relay

An `InstantIntent` enters a 15-second executor auction. The winning executor commits to fulfilling the intent within 120 seconds. A 30% executor bond is slashed on timeout. The protocol fee is 10 basis points of the transferred amount.

### 10.3 Tier 3: ZK Proof Bridge

The Tier 3 bridge batches up to 32 cross-chain intents into proof bundles. `ProofGenerator` constructs a deterministic transcript from `intent_id`, `source_state_root`, `dest_tx_hash`, and `dest_amount_delivered`, then binds it with a SPHINCS+ signature. `ProofVerifier` verifies using the corresponding post-quantum public key. No structured reference string or MPC ceremony is required. Double-spend prevention uses `GlobalNullifierSet` with atomic `WriteBatch (sync=true)`.

### 10.4 Tier 2: Full-Node Verification

Tier 2 requires 90% consensus across at least three independent verifier nodes running different blockchain client implementations. Optional TEE attestations provide additional integrity guarantees for high-value transfers.

### 10.5 Tier 1: Social Consensus

Tier 1 handles scenarios where cryptographic or economic guarantees are insufficient: chain reorganisations, detected quantum attacks, smart contract bugs, and protocol upgrades. Standard proposals use a 7-day voting window with a 66% approval threshold. Emergency proposals use a 24-hour window with an 80% threshold.

---

## 11. Economics and Tokenomics

### 11.1 Token Parameters

| Parameter | Value | Source |
|-----------|-------|--------|
| Maximum supply (†) | 200,000,000 BLEEP | `MAX_SUPPLY` in `tokenomics.rs` |
| Token decimals | 8 (1 BLEEP = 10⁸ microBLEEP) | `tokenomics.rs` |
| Initial circulating supply | 25,000,000 BLEEP (12.5%) | `INITIAL_CIRCULATING_SUPPLY` |
| Maximum per-epoch inflation (†) | 500 bps (5%) | `MAX_INFLATION_RATE_BPS` |
| Fee burn split (†) | 2,500 bps (25%) | `FEE_BURN_BPS` in `distribution.rs` |
| Validator fee split | 5,000 bps (50%) | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury fee split | 2,500 bps (25%) | `FEE_TREASURY_BPS` |
| Split integrity | Burn + Validator + Treasury = 10,000 bps | Compile-time const-assertion |
| Minimum base fee | 1,000 microBLEEP | `MIN_BASE_FEE` in `fee_market.rs` |
| Maximum base fee | 10,000,000,000 microBLEEP | `MAX_BASE_FEE` in `fee_market.rs` |
| Max base fee change per block | 1,250 bps (12.5%) | `max_increase_bps` |

*Table 7 — Token parameters († = constitutional)*

### 11.2 Fee Market

The base fee adjusts per block against a 50% block capacity target, following an EIP-1559-style mechanism. `FeeDistribution::compute()` splits each collected fee 25/50/25 across burn, validator rewards, and treasury. At sustained throughput above 10,000 TPS, the annual burn rate exceeds Year 5+ validator emission, creating net deflationary pressure.

### 11.3 Validator Emission Schedule

| Year | Rate | Annual emission (BLEEP) | Cumulative | Pool remaining |
|------|------|------------------------|------------|----------------|
| 1 | 12% | 7,200,000 | 7,200,000 | 52,800,000 |
| 2 | 10% | 6,000,000 | 13,200,000 | 46,800,000 |
| 3 | 8% | 4,800,000 | 18,000,000 | 42,000,000 |
| 4 | 6% | 3,600,000 | 21,600,000 | 38,400,000 |
| 5+ | 4% | 2,400,000/yr | — | Decreases annually |

*Table 8 — Validator emission schedule*

### 11.4 Token Distribution

| Allocation | Tokens | % | Launch unlock | Vesting terms |
|-----------|--------|---|--------------|---------------|
| Validator Rewards | 60,000,000 | 30% | 10,000,000 | Emission decay schedule |
| Ecosystem Fund | 50,000,000 | 25% | 5,000,000 | 10-year linear; disbursement by governance vote |
| Community Incentives | 30,000,000 | 15% | 5,000,000 | Governance-triggered release |
| Foundation Treasury | 30,000,000 | 15% | 5,000,000 | 6-year linear; spending by governance vote |
| Core Contributors | 20,000,000 | 10% | 0 | 1-year cliff + 4-year linear; immutable on-chain contract |
| Strategic Reserve | 10,000,000 | 5% | 0 | Governance-controlled unlock; proposal + vote required |

*Table 9 — Token distribution*

### 11.5 Game-Theoretic Safety Verifier

`SafetyVerifier` in `bleep-economics/src/game_theory.rs` formally evaluates five attack models: Equivocation, Censorship, NonParticipation, Griefing, and Cartel formation. A build fails if any model returns `is_profitable = true` at current parameters, providing a machine-verified economic safety property.

---

## 12. AI Advisory and Inference Engine

`bleep-ai` provides two systems operating at different maturity levels. Neither participates in block production, consensus voting, or any state-modifying operation without a prior governance vote. AI outputs are inputs to the governance process, not outputs of it.

### 12.1 Phase 3: Rule-Based Advisory

`AIConstraintValidator` is a deterministic rule engine that checks governance proposals against the four constitutional invariants before they enter the vote queue. It is not a learned model; it applies explicit rules derived from the constitutional parameter set.

### 12.2 Phase 4: DeterministicInferenceEngine

`DeterministicInferenceEngine` is an ONNX-based inference runtime enforcing six invariants: SHA3-256 model hash verification, deterministic input normalisation, deterministic output rounding, CPU-only execution, governance-approval gating, and no dynamic model loading. Every inference produces an `InferenceRecord` containing the model hash, normalised inputs, raw outputs, and a deterministic seed for reproducibility verification.

### 12.3 AI Attestation

`AIAttestationManager` records every AI output as an `AIAttestationRecord`. Each record contains an `AIOutputCommitment` computed as `SHA3-256(model_hash || inputs_hash || output_hash || epoch)` and a `ProofOfInference`. Records are queryable at `GET /rpc/ai/attestations/{epoch}`.

### 12.4 Safety Constraints and Scope Boundaries

- AI outputs are advisory only — no write access to chain state or block production pipeline.
- All AI outputs are signed and verifiable via `AIAttestationManager`.
- AI cannot override governance authority.
- Deterministic feature extraction is required for all consensus-critical inference.
- Failed inference returns an explicit, typed error — no silent degradation.

---

## 13. Scalability under Deterministic Constraints

BLEEP increases throughput exclusively through mechanisms that preserve the determinism invariant and the per-shard BFT safety bound.

### 13.1 Sharding Model

BLEEP partitions state across 10 shards (`NUM_SHARDS`) in the pre-testnet configuration. `ShardManager` routes transactions to shards by account address. Each shard maintains an independent RocksDB instance and processes transactions in parallel. The shard count is a governance parameter bounded by the BFT safety requirement: each shard must maintain f < S_shard/3.

### 13.2 Cross-Shard Transactions

Transactions modifying accounts on multiple shards use `TwoPhaseCommitCoordinator`. The coordinator shard is derived deterministically from the transaction hash. Coordinators exceeding a timeout height are force-aborted by the `cross_shard_timeout_sweep` task every 60 seconds, releasing all shard locks.

### 13.3 Observed Performance

| Metric | Observed value |
|--------|---------------|
| Configuration | 10 shards, 4,096 tx/block, 3,000 ms interval, 1-hour run |
| Average TPS | 10,921 (target ≥10,000 — pass) |
| Peak TPS | 13,200 |
| Sustained minimum TPS | 9,840 |
| Total transactions processed | 39,315,600 |
| Full-capacity block ratio | 82.3% |

*Table 10 — Benchmark record (source: `GET /rpc/benchmark/latest`, pre-testnet pilot)*

These figures reflect pre-testnet pilot conditions: controlled network latency, geographically concentrated nodes, and a uniform transaction workload. Throughput on a geographically distributed mainnet with heterogeneous transaction types and higher validator counts will differ.

### 13.4 Fault Recovery and State Repair

`SelfHealingOrchestrator` tracks protocol health across Healthy, Degraded, Critical, and Recovering states. Low and medium severity faults are self-correcting; high and critical severity faults require quorum approval before execution. `FaultDetector` detects seven fault types by rule. All recovery actions are deterministic: identical fault evidence produces identical recovery actions on all honest validators.

---

## 14. Use Cases

### 14.1 Sovereign Digital Asset Custody

**Long-Horizon Asset Management.** Institutions managing digital assets over multi-decade horizons face retroactive vulnerability from the harvest-now, decrypt-later threat model. BLEEP's SPHINCS+ transaction signing and Kyber-1024 key encapsulation ensure that asset custody records signed at genesis remain computationally opaque against a future quantum-capable adversary.

**Central Bank Digital Currency Infrastructure.** State-level monetary authorities issuing digital currency require cryptographic foundations that will remain secure across multiple decades. BLEEP's NIST-standardised post-quantum primitives at Security Level 5 provide a documented, auditable security basis aligned with government procurement requirements. The tamper-evident, SHA3-256 Merkle-chained audit log satisfies regulatory requirements for non-repudiable transaction records.

### 14.2 Cross-Chain Settlement Infrastructure

**High-Value Institutional Settlement.** BLEEP Connect's four-tier bridge architecture allows settlement systems to select the appropriate security level for each transfer. Tier 4 provides sub-second settlement; Tier 3 provides cryptographic verification via SPHINCS+-bound STARK commitments with no trusted operator; Tier 2 provides multi-client independent full-node consensus.

**Automated Cross-Chain Protocol Execution.** BLEEP's BFT finality guarantee — requiring more than 6,667 basis points of staked supply to commit — provides deterministic finality rather than probabilistic confirmation. The nullifier store with atomic writes ensures that cross-chain double-spend attempts are rejected at the protocol level.

### 14.3 Verifiable Computation and Proof Markets

**Post-Quantum ZK Application Layer.** Application developers building zero-knowledge applications require a proof system that will remain sound against quantum adversaries. BLEEP's Winterfell STARK execution engine (Tier 5 in the VM dispatch table) provides public input verification against post-quantum-secure proofs requiring no trusted setup.

**Auditable AI Decision Systems.** `AIAttestationManager` creates an on-chain record of each inference, including the SHA3-256 hash of the model binary, normalised inputs, and outputs, committed to the tamper-evident audit log. This allows post-hoc verification that a specific model version produced a specific decision from specific inputs.

### 14.4 Regulated Financial Infrastructure

**Compliance-Ready Transaction Processing.** BLEEP's constitutional compile-time invariants provide machine-verifiable evidence that the maximum token supply, inflation rate, and fee burn parameters cannot be altered without a code change that fails to compile. The tamper-evident audit log provides the non-repudiation records required for regulatory reporting under MiCA, DORA, and SEC digital asset guidance.

**Post-Quantum Securities Settlement.** Settlement records signed today with classical keys will remain on-chain for decades. BLEEP's post-quantum signatures ensure that settlement records produced today remain cryptographically valid against future adversaries. The BFT finality guarantee eliminates settlement uncertainty.

### 14.5 Decentralized Governance and DAO Infrastructure

**Privacy-Preserving Stakeholder Voting.** `ZKVotingEngine` votes are cast as `EncryptedBallot` structs; `EligibilityProof` establishes voting power without revealing validator identity; `TallyProof` allows independent tally verification without learning individual votes.

**Protocol-Governed Parameter Management.** `GovernableParam` enum and `ForklessUpgradeEngine` allow protocol parameters to be adjusted through a hash-committed, auditable governance process while constitutional invariants enforced at compile time ensure that fundamental parameters remain beyond governance reach.

---

## 15. Target Users

BLEEP is designed for participants who require long-term cryptographic integrity, deterministic execution guarantees, and auditable governance.

**Institutional Asset Managers and Custodians** operating under multi-decade investment horizons require cryptographic guarantees that remain valid regardless of future advances in quantum computing. BLEEP provides SPHINCS+-signed transaction records from genesis with no migration risk.

**Regulated Financial Institutions** require deterministic settlement finality, non-repudiable audit trails, and governance mechanisms preventing unilateral parameter changes. BLEEP's constitutional compile-time assertions and Merkle-chained audit log satisfy these requirements.

**Cross-Chain Protocol Developers** building bridges and interoperability infrastructure require trustless verification without permanently privileged operators. BLEEP Connect's Tier 3 STARK bridge and Tier 4 economic slash-bond design eliminate persistent trust assumptions.

**Application Developers Building ZK Systems** require a proof system that will remain sound against future quantum adversaries. BLEEP's Winterfell STARK execution tier provides this with no trusted setup.

**Security Researchers and Cryptographers** will find BLEEP's modular crate structure, acyclic dependency graph, documented post-quantum boundary, five fuzz targets, independent security audit, and 72-hour adversarial test suite a suitable subject for formal security analysis.

---

## 16. Security Considerations

### 16.1 Threat Model

BLEEP's security analysis considers three adversary classes:

- **Classical PPT adversary:** targets 256-bit security on all operations.
- **Quantum QPT adversary:** BLEEP's post-quantum boundary — SPHINCS+, Kyber-1024, and Winterfell STARK proofs — maintains 256-bit security across all protocol paths. No path within the boundary is broken by Shor's algorithm.
- **Byzantine validator adversary:** controls f < S/3 of staked supply and may direct those validators to behave arbitrarily. The BFT safety guarantee holds unconditionally under this model.

### 16.2 Independent Security Audit

An independent security audit reviewed 16,127 lines of Rust across six crates, identifying 14 findings.

| Severity | Count | Resolved | Acknowledged | Outcome |
|----------|-------|----------|-------------|---------|
| Critical | 2 | 2 | 0 | All resolved |
| High | 3 | 3 | 0 | All resolved |
| Medium | 4 | 3 | 1 | SA-M4: EIP-1559 design property; documented in `THREAT_MODEL.md` |
| Low | 3 | 3 | 0 | All resolved |
| Informational | 2 | 1 | 1 | SA-I2: NTP drift guard is a mainnet gate |
| **Total** | **14** | **12** | **2** | **Cleared for mainnet preparation** |

*Table 11 — Audit finding summary (source: `docs/SECURITY_AUDIT_SPRINT9.md`)*

### 16.3 Adversarial Test Suite

| Scenario | Result | Invariant verified |
|---------|--------|--------------------|
| `ValidatorCrash(1)` | Pass | f=1 < 2.33; consensus resumed |
| `ValidatorCrash(2)` | Pass | f=2 < 2.33; consensus resumed |
| `NetworkPartition(4/3)` | Pass | Majority partition continued; healed cleanly |
| `LongRangeReorg(10)` | Pass | Rejected at `FinalityManager` (invariant I-CON3) |
| `LongRangeReorg(50)` | Pass | Rejected at `FinalityManager` (invariant I-CON3) |
| `DoubleSign(validator-0)` | Pass | 33% slashed; evidence committed; tombstoned |
| `TxReplay` | Pass | Rejected by nonce check (invariant I-S5) |
| `InvalidBlockFlood(1000)` | Pass | Rejected at SPHINCS+ gate; peer rate-limited |
| `LoadStress(10,000 TPS, 60s)` | Pass | Block capacity saturated; max throughput reached |

*Table 12 — Selected adversarial test results (72-hour continuous run)*

### 16.4 Game-Theoretic Safety

`SafetyVerifier` evaluates five attack models against current parameters: Equivocation, Censorship, NonParticipation, Griefing, and Cartel formation. A build fails if any model returns `is_profitable = true`, providing machine-verified economic safety analogous to the compile-time constitutional invariants.

---

## 17. Limitations

### 17.1 Post-Quantum Primitives Introduce Measurable Overhead

The selection of post-quantum primitives at Security Level 5 introduces computational and bandwidth costs materially larger than classical alternatives.

**Signature size:** SPHINCS+-SHAKE-256f-simple produces 7,856-byte signatures, compared to 64 bytes for ECDSA. On a 4,096-transaction block, aggregate signature data is approximately 32 MB. At the 3,000 ms slot interval, this imposes a minimum bandwidth requirement of approximately 87 MB/s from signatures alone.

**Key sizes:** Kyber-1024 public keys are 1,568 bytes and ciphertexts are 1,568 bytes, compared to 32-byte Curve25519 keys.

**Verification throughput:** SPHINCS+ signature verification is measurably slower than ECDSA. STARK proofs are larger than compact SNARKs, with verification time scaling as O(log² n) in constraint count.

These overheads are the direct, quantified cost of the post-quantum security guarantee. A system using classical primitives accepts lower present overhead at the cost of long-term retroactive vulnerability. BLEEP accepts the overhead as an explicit design trade-off.

---

## 18. Future Work

### 18.1 Winterfell Proof Backend — Full Activation

The primary near-term engineering milestone is activating `winterfell::Prover::prove()` and `winterfell::verify()` in `BlockValidityProver` and `BlockValidityVerifier`. The AIR and constraint system are already defined; the work is wiring the existing trace construction to the FRI cryptographic backend and benchmarking proof generation times against the 3,000 ms slot budget.

### 18.2 ONNX Inference Pipeline

Phase 4 completes the `DeterministicInferenceEngine` training pipeline, model governance approval flow, and `AIConstraintValidator` v2 with a trained classification model.

### 18.3 Public Pre-testnet Expansion

Phase 4 targets at least 50 validators across at least 6 continents, with open registration, a public block explorer, a 30-day sustained run, and a 100,000 BLEEP bug bounty pool.

### 18.4 Mainnet Deployment

Mainnet requires: at least 21 validators; governance active from block 1; BLEEP Connect Tier 1 through Tier 4 operational on Ethereum and Solana; client SDKs; NTP drift guard active; and `GenesisAllocation` vesting contracts deployed.

### 18.5 Signature Aggregation

SPHINCS+ does not support aggregation: n validators produce n independent 7,856-byte signatures. Hash-based signature aggregation combining Merkle-based multi-signatures with the SPHINCS+ construction is a medium-term research direction.

---

## 19. Conclusion

BLEEP is a Quantum Trust Network: a decentralized execution protocol in which transaction signing, peer authentication, key encapsulation, and zero-knowledge proof verification are each secured by NIST-standardised post-quantum algorithms or hash-based transparent proof systems at Security Level 5. No classical public-key primitive or pairing-based construction is present on any cryptographically sensitive path.

Protocol Version 4 demonstrates the practical feasibility of the design: SPHINCS+-signed blocks at a 3,000 ms slot interval, Kyber-1024 key encapsulation for peer channels, STARK block validity proofs, a 72-hour adversarial test suite with no unresolved failures, an independent security audit with all Critical and High findings resolved, and a one-hour sustained benchmark averaging 10,921 transactions per second across 10 shards under pre-testnet pilot conditions.

The contribution of BLEEP is the demonstration that a practical, audited, and operationally tested foundation can be constructed on exclusively post-quantum primitives — including transparent, setup-free zero-knowledge proofs — with the determinism, governance, and economic machinery required for a deployable protocol, at a security level that provides meaningful long-term resistance to quantum adversaries.

---

## References

[1] Shor, P.W. (1994). Algorithms for quantum computation: discrete logarithms and factoring. *Proceedings of the 35th Annual Symposium on Foundations of Computer Science.*

[2] Banegas, G. et al. (2021). Concrete quantum cryptanalysis of binary elliptic curves. *IACR Transactions on Cryptographic Hardware and Embedded Systems.*

[3] Mosca, M. (2018). Cybersecurity in an era with quantum computers: will we be ready? *IEEE Security & Privacy*, 16(5), 38–41.

[4] Amann, J. et al. (2017). Mission accomplished? HTTPS security after DigiNotar. *ACM IMC 2017.*

[5] Grover, L.K. (1996). A fast quantum mechanical algorithm for database search. *Proceedings of the 28th ACM Symposium on Theory of Computing.*

[6] NIST. (2024). Post-Quantum Cryptography Standardization. FIPS 203, FIPS 204, FIPS 205.

[7] Lamport, L., Shostak, R., and Pease, M. (1982). The Byzantine generals problem. *ACM Transactions on Programming Languages and Systems*, 4(3), 382–401.

[8] Ben-Sasson, E. et al. (2018). Scalable, transparent, and post-quantum secure computational integrity. *IACR ePrint 2018/046.*

[9] Fischer, M.J., Lynch, N.A., and Paterson, M.S. (1985). Impossibility of distributed consensus with one faulty process. *Journal of the ACM*, 32(2), 374–382.

[10] Goldwasser, S., Micali, S., and Rackoff, C. (1989). The knowledge complexity of interactive proof systems. *SIAM Journal on Computing*, 18(1), 186–208.

[11] Winterfell STARK library. (2024). <https://github.com/novifinancial/winterfell>

[12] Bernstein, D.J. and Lange, T. (2017). Post-quantum cryptography. *Nature*, 549, 188–194.

[13] Buchman, E., Kwon, J., and Milosevic, Z. (2018). The latest gossip on BFT consensus. *arXiv:1807.04938.*

---

## Appendix A: Protocol Parameters

All values are drawn from the production Rust source at Protocol Version 4. Parameters marked (†) are constitutional and cannot be changed by governance vote or software upgrade.

### A.1 Consensus and Execution

| Parameter | Value | Source constant |
|-----------|-------|----------------|
| Block interval | 3,000 ms | `BLOCK_INTERVAL_MS` |
| Max transactions per block | 4,096 | `MAX_TXS_PER_BLOCK` |
| Blocks per epoch (mainnet) | 1,000 | `BLOCKS_PER_EPOCH` |
| Blocks per epoch (pre-testnet) | 100 | `testnet-genesis.toml` |
| Finality threshold (†) | >6,667 bps of total stake | `FinalityManager` |
| Active shards | 10 | `NUM_SHARDS` |
| Double-sign slash | 33% of stake | `double_signing_penalty` |
| Equivocation slash | 25% of stake | `equivocation_penalty` |
| Downtime slash | 0.1% per missed block | `downtime_penalty_per_block` |

### A.2 Cryptography and Networking

| Parameter | Value | Source constant |
|-----------|-------|----------------|
| SPHINCS+ signature size | 7,856 bytes | `SPHINCS_SIG_LEN` |
| SPHINCS+ public key size | 32 bytes | `pqcrypto-sphincsplus` |
| Kyber-1024 public key size | 1,568 bytes | `pqcrypto-kyber` |
| State trie depth | 256 levels | `TRIE_DEPTH` |
| Merkle proof size | 8,192 bytes | `SparseMerkleTrie` |
| Gossip max message size | 2,097,152 bytes (2 MiB) | `MAX_GOSSIP_MSG_BYTES` |
| Gossip fanout | 8 | `bleep-p2p` |
| Kademlia k-bucket size | 20 | `bleep-p2p` |
| Onion routing max hops | 6 | `MAX_HOPS` |
| ZK proof system | Winterfell STARK (FRI-based, f128 field) | `bleep-zkp` |
| Proof setup requirement | None (transparent) | `bleep-zkp` |
| JWT entropy minimum | 3.5 bits/byte (Shannon) | `session.rs` |

### A.3 Economics and Token

| Parameter | Value | Source constant |
|-----------|-------|----------------|
| Maximum supply (†) | 200,000,000 BLEEP | `MAX_SUPPLY` |
| Token decimals | 8 | `tokenomics.rs` |
| Initial circulating supply | 25,000,000 (12.5%) | `INITIAL_CIRCULATING_SUPPLY` |
| Maximum per-epoch inflation (†) | 500 bps (5%) | `MAX_INFLATION_RATE_BPS` |
| Fee burn split (†) | 2,500 bps (25%) | `FEE_BURN_BPS` |
| Validator fee split | 5,000 bps (50%) | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury fee split | 2,500 bps (25%) | `FEE_TREASURY_BPS` |
| Min base fee | 1,000 microBLEEP | `MIN_BASE_FEE` |
| Max base fee | 10,000,000,000 microBLEEP | `MAX_BASE_FEE` |

### A.4 Cross-Chain Bridge

| Parameter | Value | Source constant |
|-----------|-------|----------------|
| Tier 3 proof type | SPHINCS+-bound STARK commitment | `bleep-connect-layer3-zkproof` |
| Tier 3 batch size | 32 intents | `L3_BATCH_SIZE` |
| Tier 3 setup requirement | None (transparent) | `bleep-rpc` |
| Tier 2 consensus threshold | 90% | `CONSENSUS_THRESHOLD` |
| Tier 2 minimum verifiers | 3 | `MIN_VERIFIER_NODES` |
| Tier 4 execution timeout | 120 s | `EXECUTION_TIMEOUT` |
| Tier 4 protocol fee | 10 bps (0.1%) | `PROTOCOL_FEE_BPS` |

---

*© 2026 BLEEP Project · Quantum Trust Network · Protocol Version 1 · bleep-pretestnet-1*
