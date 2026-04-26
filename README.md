# BLEEP — Quantum Trust Network

**Protocol Version 3 · `bleep-testnet-1`**

Every transaction on a classical blockchain is a permanent public record signed with a key that Shor's algorithm breaks in polynomial time. An adversary does not need a quantum computer today — they need storage. They archive now and decrypt when the hardware catches up. For a live chain with real economic value, that window closes exactly once.

BLEEP uses exclusively NIST-finalised post-quantum primitives at Security Level 5 on every cryptographic path: transaction signing, block signing, peer-to-peer authentication, and key encapsulation. There is no classical fallback, no feature flag, no hybrid mode. The premise is that the right time to establish post-quantum foundations is before the protocol accumulates value and ecosystem dependencies, not after.

---

## Contents

- [How it works](#how-it-works)
- [Workspace](#workspace)
- [Cryptography](#cryptography)
- [Consensus](#consensus)
- [State layer](#state-layer)
- [Execution](#execution)
- [Networking](#networking)
- [Governance](#governance)
- [Economics](#economics)
- [Cross-chain bridge](#cross-chain-bridge)
- [AI advisory](#ai-advisory)
- [Running a node](#running-a-node)
- [Configuration](#configuration)
- [RPC reference](#rpc-reference)
- [CLI reference](#cli-reference)
- [Security](#security)
- [Known limitations](#known-limitations)
- [Roadmap](#roadmap)
- [Contributing](#contributing)

---

## How it works

BLEEP is a proof-of-stake BFT chain. Safety holds when Byzantine stake stays below one third of total staked supply. Finality is not probabilistic — a block receiving precommits from validators holding more than 6,667 basis points of total staked supply is irreversible. There is no challenge window, no reorganisation depth after which you are probably safe.

The state transition function is deterministic and total: given the same starting state and transaction set, every honest node running the same software version produces byte-identical results. Non-determinism on any consensus-critical path is a protocol bug.

Four parameters are enforced by Rust compile-time `const` assertions. A code change that violates any of them does not compile:

| Parameter | Value | Constant |
|---|---|---|
| Maximum token supply | 200,000,000 BLEEP | `MAX_SUPPLY` |
| Minimum finality threshold | > 6,667 bps of total stake | enforced in `FinalityManager` |
| Maximum per-epoch inflation | 500 bps | `MAX_INFLATION_RATE_BPS` |
| Fee burn floor | 2,500 bps | `FEE_BURN_BPS` |

Testnet results (1-hour sustained, 10 shards, 7 validators, `bleep-testnet-1`):

```
avg TPS:         10,921   (target ≥ 10,000 — pass)
peak TPS:        13,200
min TPS:         9,840    (sustained floor across full 60-minute window)
total txs:       39,315,600
STARK avg:     1500 ms
full-cap blocks: 82.3%
```

---

## Workspace

19 crates in a single Cargo workspace. The inter-crate dependency graph is acyclic and enforced at build time. `bleep-crypto` has no dependencies on other BLEEP crates. A vulnerability in networking cannot reach raw key material.

```
crates/
├── bleep-crypto        SPHINCS+, Kyber-1024, AES-256-GCM, SHA3-256, BLAKE3
├── bleep-zkp           STARK/BLS12-381 circuits, prover/verifier, transparent
├── bleep-wallet-core   EncryptedWallet, Zeroizing key storage, WalletManager
├── bleep-core          Block, Transaction, Blockchain, Mempool, BlockValidator
├── bleep-consensus     BlockProducer, ConsensusOrchestrator, FinalityManager,
│                       SlashingEngine, SelfHealingOrchestrator, EpochConfig
├── bleep-state         StateManager (RocksDB), SparseMerkleTrie (256-level),
│                       ShardManager, cross-shard 2PC, NullifierStore, AuditLog
├── bleep-vm            7-engine dispatcher: Native / EVM / WASM / ZK /
│                       AI-Advised / CrossChain, VmRouter, StateDiff
├── bleep-p2p           Kademlia (k=20), gossip (fanout 8), onion routing,
│                       PeerScoring, 2 MiB receive gate
├── bleep-auth          JWT sessions, RBAC, Kyber-1024 validator binding,
│                       salted SHA3-256 credentials, audit log, rate limiting
├── bleep-rpc           warp HTTP/JSON, 46 endpoints
├── bleep-scheduler     20-task Tokio scheduler (epoch, rewards, healing, ...)
├── bleep-governance    LiveGovernanceEngine, ZKVotingEngine,
│                       ForklessUpgradeEngine, proposal lifecycle
├── bleep-economics     EIP-1559 fee market, FeeDistribution, SafetyVerifier,
│                       validator emission schedule, oracle bridge
├── bleep-ai            AIConstraintValidator (Phase 3 — operational),
│                       DeterministicInferenceEngine (Phase 4 — in dev),
│                       AIAttestationManager
├── bleep-interop/      BLEEP Connect — 10 sub-crates, 4 bridge tiers
│   ├── bleep-connect-types
│   ├── bleep-connect-crypto
│   ├── bleep-connect-adapters
│   ├── bleep-connect-commitment-chain
│   ├── bleep-connect-executor
│   ├── bleep-connect-layer4-instant    live on Ethereum Sepolia
│   ├── bleep-connect-layer3-zkproof    live on Ethereum Sepolia
│   ├── bleep-connect-layer2-fullnode   implemented, mainnet target
│   ├── bleep-connect-layer1-social     implemented, mainnet target
│   └── bleep-connect-core
├── bleep-pat           Programmable Asset Token registry and ledger
├── bleep-indexer       DashMap chain indexes, reorg rollback, checkpoints
├── bleep-cli           clap async CLI
└── bleep-telemetry     tracing-subscriber, MetricCounter, Prometheus export
```

Node entrypoint: `src/bin/main.rs`. Startup follows a 16-step dependency-ordered sequence. Post-quantum keypairs are generated first. `StateManager` opens RocksDB — including `nullifier_store` and `audit_log` column families — before block production activates. STARK proofs are generated transparently without trusted setup. The node signals readiness only after all 46 RPC endpoints are confirmed active. Any failure halts rather than leaving the node partially initialised.

---

## Cryptography

`bleep-crypto` is the root dependency of the protocol. No other crate performs raw cryptographic operations.

### Algorithms

**Transaction and block signing — SPHINCS+-SHAKE-256f-simple (FIPS 205 / SLH-DSA, Security Level 5)**

Security reduces to the one-wayness of SHAKE-256. No algebraic structure means no Shor's-algorithm attack surface. The tradeoff is signature size: 7,856 bytes. At 4,096 transactions per block that is approximately 32 MB of signature data per block, roughly 87 MB/s of bandwidth from signatures alone before payloads and vote messages. Signature aggregation for hash-based schemes is an open research problem with no standardised solution. See [Known limitations](#known-limitations).

**Key encapsulation — Kyber-1024 / ML-KEM-1024 (FIPS 203, Security Level 5)**

Used for validator binding, inter-node session establishment, and wallet key encapsulation. Security reduces to Module-LWE hardness. 1,568-byte public keys — larger than Curve25519's 32 bytes — shows up in session handshakes and validator registry storage.

**Symmetric — AES-256-GCM**

Onion routing hops and wallet signing key encryption at rest. The 256-bit key gives approximately 128 bits of post-quantum security against Grover. Accepted.

**Hashing — SHA3-256, BLAKE3**

SHA3-256: state commitments, Merkle node hashing, block hashing, audit log chaining, AI model binary hashing. BLAKE3: indexer content-addressing. Grover reduces effective security from 256 bits to approximately 128 bits — accepted at Level 5.

### Key lifecycle

Secret keys are wrapped in `zeroize::Zeroizing<Vec<u8>>`. The `Zeroize` derive macro zeros the backing allocation before the allocator reclaims it, regardless of whether the drop is normal or through stack unwinding. This closes the class of vulnerability where key material persists in swap or core dumps (audit finding SA-L3).

```rust
use bleep_crypto::{generate_tx_keypair, tx_payload, sign_tx_payload, verify_tx_signature};

let (pk, sk) = generate_tx_keypair();
let payload  = tx_payload(&sender, &receiver, amount, timestamp);
let sig      = sign_tx_payload(&payload, &sk)?;  // 7,856-byte SPHINCS+ signature
assert!(verify_tx_signature(&payload, &sig, &pk));
// sk drops here — Zeroizing zeroes the Vec before dealloc
```

`sign_tx_payload` returns the signature, never key bytes. A previous implementation returned raw key material as the signature value; this was corrected before the independent audit.

### The post-quantum boundary

STARK over BLS12-381 is used for block validity proofs and the Tier 3 bridge. It is post-quantum secure — no trusted setup required, conjectured secure against quantum attacks based on hash collision resistance.

```
IN SCOPE — post-quantum secure            OUTSIDE SCOPE
─────────────────────────────────         ─────────────────────────────
transaction signing (SPHINCS+)            block validity proofs (STARK)
block signing (SPHINCS+)                  Tier 3 bridge proofs (STARK)
P2P authentication (SPHINCS+)
key encapsulation (Kyber-1024)
```

For block production, a quantum adversary needs to forge both a STARK proof and a SPHINCS+ block signature. Both are believed infeasible. For the Tier 3 bridge, the exposure is direct — a quantum adversary would need to break the STARK proof, which is conjectured secure.

### MPC ceremony

Five-participant public ceremony over BLS12-381 (`powers-of-tau-bls12-381-bleep-v1`). Transcript: [ceremony.bleep.network/transcript-v1.json](https://ceremony.bleep.network/transcript-v1.json). Sound if at least one participant destroyed their toxic waste contribution.

Audit finding SA-M1: the original ceremony accepted unsigned contributions, permitting substitution attacks. The fix requires each contribution to carry a SPHINCS+ signature over `(id || hash || timestamp)`. The ceremony running hash now uses SHA3-256 rather than the XOR mix that was in the original implementation.

The node verifies the SRS against the transcript on startup. A mismatch halts the node before any ZK operations.

### Fuzzing

Five fuzz targets in `bleep-crypto/fuzz` run on every CI build: hash determinism, sign/verify round-trips, Kyber encap/decap, Merkle insertion soundness, state transition fund conservation.

---

## Consensus

### Validator model

Let V = {v₁…vₙ} be the active validator set at epoch e. Each validator holds a SPHINCS+ verification key, a Kyber-1024 encapsulation key, and stake sᵢ in microBLEEP. S = Σsᵢ.

Safety holds when Byzantine stake f < S/3. Network model: partial synchrony. Safety holds under full asynchrony; liveness requires eventual delivery within Δ. The 3,000 ms slot timer is calibrated against observed testnet propagation latency.

### Block production

Each 3-second slot:

1. Proposer selected with probability proportional to stake — deterministic, no coordinator.
2. `BlockProducer` pulls up to 4,096 transactions from the mempool by fee in descending order and applies them to a draft state. Any transaction that causes an invariant violation — overdraft, nonce regression, supply cap breach — is evicted and the block rebuilt.
3. Sparse Merkle Trie root committed to the block header.
4. STARK `BlockValidityAir` proof generated (avg 1500 ms on testnet hardware). The circuit proves structural consistency and proposer possession. It does not prove full execution validity — every validator does that independently.
5. Block signed with SPHINCS+ and broadcast.
6. Receiving validators verify the STARK proof, SPHINCS+ signature, and SMT root transition independently.
7. Prevote then precommit — each message SPHINCS+-signed.
8. Finalisation at > 6,667 bps of S. Irreversible — not probabilistic, not subject to rollback.
9. Epoch boundary every 1,000 blocks (mainnet) / 100 blocks (testnet): validator rotation, reward distribution, slashing counter reset, governance events.

`ConsensusOrchestrator` selects mode deterministically at epoch boundaries — identical computation on every honest node, no coordinator required.

| Mode | Condition | Behaviour |
|---|---|---|
| `PoS-Normal` | Default | 3 s slots, stake-proportional proposer |
| `Emergency` | < 67% validator liveness | Reduced quorum; halts safely if BFT bound is at risk |
| `Recovery` | Post-partition | Re-anchors to most recent finalised checkpoint |

### Slashing

| Violation | Penalty | Notes |
|---|---|---|
| Double-sign | 33% burned, tombstoned | `saturating_sub` throughout — SA-M2 |
| Equivocation | 25% burned | |
| Downtime | 0.1% per consecutive missed block | |
| Tier 4 executor timeout | 30% of executor bond | `EXECUTION_TIMEOUT = 120 s` |

Balance debits use a RocksDB compare-and-swap loop (up to 3 retries), closing the TOCTOU race from audit finding SA-H2.

`LongRangeReorg(10)` and `LongRangeReorg(50)` were rejected at `FinalityManager` in every iteration of the 72-hour adversarial run. Once a block is final, rolling it back requires Byzantine stake ≥ S/3, which triggers the slashing cascade.

### Chaos suite results

72-hour continuous run, 7-validator testnet, 14 scenarios:

| Scenario | Result |
|---|---|
| `ValidatorCrash(1)`, `ValidatorCrash(2)` | Pass — consensus resumed within expected recovery window |
| `ValidatorCrash(3)` | Correctly halts — f=3 ≥ 2.33 violates BFT bound |
| `NetworkPartition(4/3)`, `NetworkPartition(5/2)` | Pass — majority partition continued; healed cleanly |
| `LongRangeReorg(10)`, `LongRangeReorg(50)` | Pass — rejected at `FinalityManager` (I-CON3) |
| `DoubleSign(validator-0)`, `DoubleSign(validator-3)` | Pass — 33% slashed, tombstoned |
| `TxReplay` | Pass — rejected by nonce check (I-S5) |
| `EclipseAttack(validator-6)` | Pass — Kademlia k=20 and DNS seeds |
| `InvalidBlockFlood(1000)` | Pass — rejected at SPHINCS+ gate; peer rate-limited |
| `LoadStress(1,000 / 5,000 / 10,000 TPS, 60 s)` | Pass — block capacity saturated without drops |

---

## State layer

### Storage layout

Account state lives in a 256-level Sparse Merkle Trie backed by RocksDB. The trie root is committed in every block header.

```
Key:   b"acct:" + address_utf8
Value: bincode( AccountState { balance: u128, nonce: u64, code_hash: Option<[u8;32]> } )
```

`advance_block()` is the commit boundary. All writes buffer until it is called. A crash before `advance_block()` leaves the previous block's state intact.

Three column families handle security-critical operations:

| Column family | Purpose |
|---|---|
| `nullifier_store` | Bridge nullifier hashes — `WriteBatch` with `sync=true`. The original implementation used an in-memory `HashSet` that did not survive restarts, permitting double-spend after a node crash (SA-C1). |
| `audit_log` | SHA3-256 Merkle-chained entries. Each entry's hash covers the previous hash, sequence number, and event fields. Mutating any stored entry fails chain verification. |
| `audit_meta` | Chain tip and sequence counter for log recovery on restart. Warms the in-memory cache of the most recent 10,000 entries. |

### SMT proofs

The 256-level SMT gives fixed-size proofs — 8,192 bytes for both membership and non-membership regardless of account count. This matters for light clients.

```
Leaf key   = SHA3-256( address_utf8 )
Leaf value = SHA3-256( abi_encode(address, balance, nonce) )
Interior   = SHA3-256( left_child || right_child )
Empty node = [0u8; 32]
```

```rust
// full node
let proof = state.prove_account("BLEEP1...");

// light client — no node required
assert!(proof.verify(&known_state_root));
```

`GET /rpc/proof/{address}` serves the proof as JSON.

### Sharding

10 shards on testnet. `ShardManager` routes by account address hash. Each shard is an independent RocksDB instance. `ShardValidatorAssignment` maps validators to shards per epoch using a deterministic function of the epoch randomness beacon.

Cross-shard transactions go through `TwoPhaseCommitCoordinator`. The coordinator shard is derived from the transaction hash — no coordinator election. Stalled coordinators are force-aborted by `cross_shard_timeout_sweep` every 60 seconds. `ShardEpochBinding` commits each shard's root to the epoch Merkle tree at every boundary.

Increasing shard count beyond 10 reduces per-shard validator assignment and weakens per-shard BFT tolerance. The right number for mainnet depends on final validator set size.

---

## Execution

`VmRouter` dispatches to one of seven engines and returns a `StateDiff`. `BlockProducer` applies the diff under a single state lock after all calls complete. The VM never touches `StateManager` directly.

| Tier | Engine | Scope | Gas model |
|---|---|---|---|
| 1 | Native | Transfer, stake, unstake, governance vote | none |
| 2 | Router | Engine selection, gas validation, circuit breakers | validation only |
| 3 | EVM (SputnikVM) | Ethereum-compatible contracts | Ethereum semantics |
| 4 | WASM (Wasmi) | WebAssembly contracts | configurable fuel |
| 5 | ZK Proof | ZK execution, public input verification | fixed per verifier op |
| 6 | AI-Advised | Pre-execution constraint validation | deterministic; no gas |
| 7 | Cross-Chain | BLEEP Connect Tier 4 intent dispatch | protocol fee in bps |

Each engine runs behind independent circuit breakers and gas budgets. A failure in one engine does not affect the others.

---

## Networking

Peer IDs are deterministic hashes of SPHINCS+ public keys — network identity is bound to post-quantum key material.

Every inter-node message is a SPHINCS+-signed `SecureMessage`. Unauthenticated messages are dropped before payload processing. A 2 MiB gate is enforced at the receive boundary before any deserialisation (SA-H3), closing memory exhaustion attacks that operate by sending large blobs to trigger allocation before signature verification.

Onion routing uses AES-256-GCM keyed from Kyber-1024 per-hop shared secrets, up to 6 hops. Route selection filters to peers scoring above 55.0 (`MIN_RELAY_TRUST`).

`PeerScoring` computes a composite trust score in [0.0, 100.0] from success ratio, message rate, latency, and diversity. Scores decay at 0.99× per 300 seconds. Below 40: excluded from gossip relay. Below 55: excluded from onion relay.

| Component | Detail |
|---|---|
| DHT | Kademlia, k=20, XOR metric |
| Gossip | Epidemic dissemination, fanout 8 |
| Onion routing | Kyber-1024 KEM per hop, AES-256-GCM payload, max 6 hops |
| Receive gate | 2 MiB enforced before deserialisation |
| Trust scoring | Composite, exponential decay, floor at `MIN_REPUTATION_FLOOR` |

---

## Governance

**Proposal lifecycle:** Submit → AIConstraintValidator pre-flight → Active → Tally → Execute → Record

`AIConstraintValidator` runs before a proposal enters the vote queue. A proposal that would push `MaxInflationBps` above 500 is rejected here and never reaches a vote. The `constitutional_violation_rejected_at_submission` test covers this on every CI build.

Governance parameters on testnet:

| Parameter | Value |
|---|---|
| Voting window | 1,000 blocks (~50 min at 3 s/block) |
| Quorum | 1,000 bps — minimum stake participation |
| Pass threshold | 6,667 bps of participating stake |
| Veto threshold | 3,333 bps |
| Minimum deposit | 10,000 BLEEP |

`ZKVotingEngine` provides privacy-preserving stake-weighted votes. Validator weight: 1.0×. Delegator: 0.5×. Community holder: 0.1×. `EligibilityProof` establishes voting power without revealing identity. `TallyProof` allows independent verification without learning individual votes.

`ForklessUpgradeEngine` activates hash-committed upgrades at epoch boundaries only. Validators know exactly what code will activate before casting a vote. `Version.is_valid_upgrade()` enforces monotonic version progression. Partial upgrade payloads are rejected atomically.

`proposal-testnet-001` ran the full lifecycle on `bleep-testnet-1`: pre-flight, ZK votes from 7 validators, 70% quorum, on-chain execution at block 1,105. It reduced `FeeBurnBps` from 2,500 to 2,000.

---

## Economics

### Token parameters

Parameters marked (†) are enforced by compile-time `const` assertions. A code change that violates them does not compile.

| Parameter | Value | Source constant |
|---|---|---|
| Max supply (†) | 200,000,000 BLEEP | `MAX_SUPPLY` |
| Decimals | 8 (1 BLEEP = 10⁸ microBLEEP) | `tokenomics.rs` |
| Initial circulating supply | 25,000,000 (12.5%) | `INITIAL_CIRCULATING_SUPPLY` |
| Max per-epoch inflation (†) | 500 bps | `MAX_INFLATION_RATE_BPS` |
| Fee burn (†) | 2,500 bps (25%) | `FEE_BURN_BPS` |
| Validator reward | 5,000 bps (50%) | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury | 2,500 bps (25%) | `FEE_TREASURY_BPS` |
| Min base fee | 1,000 microBLEEP | `MIN_BASE_FEE` |
| Max base fee | 10,000,000,000 microBLEEP | `MAX_BASE_FEE` |
| Max base fee change per block | 1,250 bps (12.5%) | `max_increase_bps` |

A `const` assertion in `distribution.rs` verifies that Burn + Validator + Treasury = 10,000 bps exactly. The sum of all six allocation buckets is verified to equal `MAX_SUPPLY` at compile time.

### Fee market

EIP-1559-style, targeting 50% block capacity. Above target: base fee increases up to 12.5% per block. Below: decreases up to 12.5%. The 25% burn creates deflationary pressure under load — at sustained throughput above 10,000 TPS, annual burn exceeds Year 5+ validator emission (2,400,000 BLEEP/year).

Audit finding SA-M4 (acknowledged, Medium): a proposer with consecutive slots can pin the base fee near maximum by filling blocks. PoS rotation limits the duration any single validator can sustain this. Documented in `THREAT_MODEL.md`.

### Emission schedule

| Year | Rate | Annual emission |
|---|---|---|
| 1 | 12% | 7,200,000 BLEEP |
| 2 | 10% | 6,000,000 |
| 3 | 8% | 4,800,000 |
| 4 | 6% | 3,600,000 |
| 5+ | 4% | 2,400,000/yr |

Encoded as `VALIDATOR_EMISSION_YEAR` in `tokenomics.rs`. Changing it requires a software upgrade, not a governance vote.

### Token distribution

| Allocation | Tokens | Launch unlock | Vesting |
|---|---|---|---|
| Validator Rewards | 60,000,000 (30%) | 10,000,000 | Emission schedule |
| Ecosystem Fund | 50,000,000 (25%) | 5,000,000 | 10-year linear; governance disbursement |
| Community Incentives | 30,000,000 (15%) | 5,000,000 | Governance-triggered |
| Foundation Treasury | 30,000,000 (15%) | 5,000,000 | 6-year linear; governance spending |
| Core Contributors | 20,000,000 (10%) | 0 | 1-year cliff + 4-year linear; immutable contract |
| Strategic Reserve | 10,000,000 (5%) | 0 | Governance unlock; proposal + vote required |

`LinearVestingSchedule` contracts for core contributors are immutable from deployment — terms cannot be modified after genesis.

### Game-theoretic safety

`SafetyVerifier` in `bleep-economics/src/game_theory.rs` evaluates five attack models at current protocol parameters: equivocation, censorship, non-participation, griefing, cartel formation. Each returns `attacker_profit`, `network_cost`, `is_profitable`. The CI build fails if any model returns `is_profitable = true` — the economic equivalent of the compile-time constitutional assertions.

---

## Cross-chain bridge

Four bridge tiers with different trust models. No tier requires a permanently privileged operator. These are parallel options for the same transfer, not sequential deployment stages.

| Tier | Mechanism | Latency | Security basis | Status |
|---|---|---|---|---|
| 4 — Instant | Executor auction + escrow | 200 ms – 1 s | Economic: 30% bond slashed on timeout | Live — Ethereum Sepolia |
| 3 — ZK Proof | STARK batch proof | 10 – 30 s | Cryptographic: STARK (post-quantum) | Live — Ethereum Sepolia |
| 2 — Full-Node | Multi-client verification | Hours | 90% consensus across ≥ 3 independent nodes | Implemented; mainnet target |
| 1 — Social | Stakeholder governance | 7 days / 24 h (emergency) | Full governance consensus | Implemented; mainnet target |

**Tier 4** — `InstantIntent` enters a 15-second executor auction. The winner fulfils within 120 seconds or loses 30% of their bond (`EXECUTION_TIMEOUT`). Protocol fee: 10 bps. Security is economic — do not route transfers approaching executor bond size through Tier 4.

**Tier 3** — Batches up to 32 intents into a single STARK proof submitted to `BleepL3Bridge` on Sepolia (approximately 250,000 gas). No trusted operator. Post-quantum secure — STARK proofs are transparent and conjectured secure against quantum attacks.

Double-spend prevention: `GlobalNullifierSet` performs an atomic `WriteBatch sync=true` on first submission and returns `Err(NullifierAlreadySpent)` on any duplicate. The original implementation used an in-memory `HashSet` that did not survive restarts (SA-C1).

**Tier 2** — Requires 90% consensus across at least 3 independent verifier nodes running different client implementations. Each node queries the actual on-chain state root independently. Optional Intel SGX attestation for high-value transfers. Avoids pairing-based cryptography entirely.

**Tier 1** — Stakeholder governance for scenarios that defeat lower tiers: chain reorganisations, detected quantum attacks, smart contract bugs at bridge scale. Standard window: 7 days, 66% threshold. Emergency (`EmergencyPause`, `StateRollback`): 24-hour window, 80% threshold.

---

## AI advisory

Two components with clearly separated scopes. Neither has write access to chain state, the governance queue, or the block production pipeline.

### AIConstraintValidator

A deterministic rule engine, not a trained model. Checks governance proposals against the four constitutional invariants before they enter the vote queue. Rejects proposals that would violate compile-time invariants before they consume any validator attention. The `constitutional_violation_rejected_at_submission` test runs on every CI build.

`AIConsensusOrchestrator` produces advisory healing proposals for the consensus layer to act on — it has no unilateral authority.

### DeterministicInferenceEngine

ONNX runtime (enabled under the `onnx` feature flag via `tract-onnx`). Enforces six invariants for any model on a consensus-critical path:

1. **Model hash verification** — SHA3-256 of the model binary must match `ModelMetadata.model_hash` before inference. Mismatch is a hard error.
2. **Deterministic input normalisation** — fixed mean, standard deviation, and clamp applied to all inputs.
3. **Deterministic output rounding** — configurable decimal precision; floating-point variance does not propagate.
4. **CPU-only execution** — no GPU paths. GPU floating-point produces non-determinism across hardware.
5. **Governance-approval gating** — `approval_epoch` must be set by governance vote before a model runs on any consensus-critical path.
6. **No dynamic loading** — model versions are immutable once deployed. Replacement requires a governance proposal.

Every inference produces an `InferenceRecord` with model hash, normalised inputs, raw outputs, and deterministic seed. `AIAttestationManager` records each output as `SHA3-256(model_hash || inputs_hash || output_hash || epoch)`.

**No trained model is currently deployed on any governance-critical or consensus-critical path.** The Phase 4 architecture above is the design specification, not the current deployment state.

---

## Running a node

### Prerequisites

```bash
# Rust stable
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Ubuntu / Debian
sudo apt-get install -y build-essential clang libclang-dev librocksdb-dev

# macOS
brew install rocksdb llvm
export LIBRARY_PATH="$(brew --prefix rocksdb)/lib:$LIBRARY_PATH"
```

### Build

```bash
git clone https://github.com/bleep-project/bleep.git
cd bleep

# Full workspace
cargo build --release --workspace

# Node binary only
cargo build --release --bin bleep

# With ONNX inference support
cargo build --release --workspace --features onnx
```

### Start a node

```bash
./target/release/bleep
# 16-step startup sequence
# RPC at :8545, P2P at :7700

curl -s http://127.0.0.1:8545/rpc/health | jq .
```

### Wallet and transactions

```bash
# Generate a SPHINCS+ + Kyber-1024 keypair, encrypted with AES-256-GCM
./target/release/bleep wallet create

# Check balance
./target/release/bleep wallet balance

# Import from BIP-39 mnemonic
./target/release/bleep wallet import "word1 word2 ... word12"

# Send — uses the SPHINCS+ signing key; set BLEEP_WALLET_PASSWORD if encrypted
./target/release/bleep tx send \
  --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 \
  --amount 1000
```

Wallet file: `~/.bleep/wallets.json`. The `signing_key` field is `nonce(12 bytes) || ciphertext || GCM-tag(16 bytes)`. The nonce is freshly generated on every lock operation — two encryptions of the same key produce different blobs.

### Run tests

```bash
cargo test --workspace

# Fuzz targets (requires nightly + cargo-fuzz)
cargo fuzz run hash_determinism
cargo fuzz run sign_verify_roundtrip
cargo fuzz run kyber_encap_decap
cargo fuzz run merkle_insertion_soundness
cargo fuzz run state_transition_fund_conservation
```

### Executor node (Tier 4 bridge)

```bash
BLEEP_EXECUTOR_KEY=<32-byte-hex-seed>            \
BLEEP_EXECUTOR_CAPITAL_BLEEP=100000000000        \
BLEEP_EXECUTOR_CAPITAL_ETH=10000000000000000000  \
BLEEP_EXECUTOR_RISK=Medium                       \
BLEEP_RPC=http://your-node:8545                  \
./target/release/bleep-executor
```

---

## Configuration

```toml
# bleep.toml

[node]
p2p_port  = 7700
rpc_port  = 8545
state_dir = "/var/lib/bleep/state"
log_level = "info"

[consensus]
block_interval_ms = 3000
max_txs_per_block = 4096
blocks_per_epoch  = 1000   # 100 on testnet
validator_id      = "validator-0"
```

| Environment variable | Default | Purpose |
|---|---|---|
| `BLEEP_RPC` | `http://127.0.0.1:8545` | RPC endpoint for CLI commands |
| `BLEEP_STATE_DIR` | `/tmp/bleep-state` | Local RocksDB path |
| `BLEEP_WALLET_PASSWORD` | (empty) | Wallet decryption passphrase |
| `RUST_LOG` | `info` | tracing log filter |
| `SEPOLIA_BLEEP_FULFILL_ADDR` | (required) | Ethereum Sepolia contract address for BLEEP fulfillment relay |

Cargo feature flags: `mainnet` (on by default), `testnet`, `onnx` (enables `tract-onnx` for ONNX inference).

---

## RPC reference

All endpoints on port **8545**. Responses are JSON. Errors carry an `"error"` string field.

### Node

| Method | Path | Response |
|---|---|---|
| GET | `/rpc/health` | `{ status, height, epoch, peers, uptime_secs, version }` |
| GET | `/rpc/telemetry` | `{ blocks_produced, transactions_processed, uptime_secs }` |
| GET | `/rpc/block/latest` | Block summary |
| GET | `/rpc/block/{height}` | Block by height |
| POST | `/rpc/tx` | Submit signed transaction |
| GET | `/rpc/tx/history` | Recent transactions |
| GET | `/rpc/state/{address}` | `{ address, balance, nonce, state_root, block_height }` |
| GET | `/rpc/proof/{address}` | 8,192-byte SMT inclusion/exclusion proof |

Balance is returned as a decimal string to avoid JSON integer overflow on `u128` values.

```bash
curl http://localhost:8545/rpc/health
# { "status": "ok", "height": 1024, "epoch": 10, "peers": 8, ... }

curl http://localhost:8545/rpc/state/BLEEP1a3f7b...
# { "balance": "10000000000", "nonce": 4, "state_root": "8a3f...", ... }
```

Verify a proof without a node:

```rust
let proof: MerkleProof = serde_json::from_str(&response)?;
assert!(proof.verify(&known_state_root));
```

### Validators

| Method | Path | Description |
|---|---|---|
| POST | `/rpc/validator/stake` | Register or increase stake |
| POST | `/rpc/validator/unstake` | Initiate graceful exit |
| GET | `/rpc/validator/list` | Active validators with stake |
| GET | `/rpc/validator/status/{id}` | Status and slashing history |
| POST | `/rpc/validator/evidence` | Submit double-sign evidence; triggers immediate slashing |

### Economics and oracle

| Method | Path | Description |
|---|---|---|
| GET | `/rpc/economics/supply` | Circulating supply, minted, burned, current base fee |
| GET | `/rpc/economics/fee` | Current base fee and last epoch |
| GET | `/rpc/economics/epoch/{n}` | Epoch output: emissions, burns, price |
| GET | `/rpc/oracle/price/{asset}` | Aggregated price (median, sources, confidence) |
| POST | `/rpc/oracle/update` | Submit price update |

### BLEEP Connect

| Method | Path | Description |
|---|---|---|
| POST | `/rpc/connect/intent` | Submit a Tier 4 instant intent |
| GET | `/rpc/connect/intent/{id}` | Intent status |
| GET | `/rpc/connect/intent/{id}/relay_tx` | Build Sepolia relay transaction |
| GET | `/rpc/connect/intents/pending` | All pending Tier 4 intents |
| POST | `/rpc/layer3/intent` | Submit a Tier 3 ZK bridge intent |
| GET | `/rpc/layer3/intents` | Tier 3 intent list and bridge stats |

### PAT (Programmable Asset Tokens)

| Method | Path | Description |
|---|---|---|
| POST | `/rpc/pat/create` | Create a PAT |
| POST | `/rpc/pat/mint` | Mint tokens |
| POST | `/rpc/pat/burn` | Burn tokens |
| POST | `/rpc/pat/transfer` | Transfer with auto-burn |
| POST | `/rpc/pat/approve` | Set allowance |
| POST | `/rpc/pat/freeze` | Freeze or unfreeze a token |
| POST | `/rpc/pat/set-burn-rate` | Update transfer burn rate |
| POST | `/rpc/pat/set-owner` | Transfer token ownership |
| GET | `/rpc/pat/balance/{symbol}/{address}` | Balance |
| GET | `/rpc/pat/info/{symbol}` | Token metadata |
| GET | `/rpc/pat/list` | All registered PATs |

### Governance

| Method | Path | Description |
|---|---|---|
| GET | `/rpc/governance/proposals` | All proposals with vote tallies |
| POST | `/rpc/governance/propose` | Submit a proposal (requires 10,000 BLEEP deposit) |
| POST | `/rpc/governance/vote` | Cast a stake-weighted ZK vote |

### Diagnostics

| Method | Path | Description |
|---|---|---|
| GET | `/rpc/benchmark/latest` | Live TPS from `BlockProducer` — real measurements, not static values |
| GET | `/rpc/audit/report` | Security audit findings and resolutions |
| GET | `/rpc/chaos/status` | Chaos suite scenario results |
| GET | `/rpc/ceremony/status` | MPC ceremony state and transcript |
| GET | `/faucet/{address}` | Dispense 1,000 test BLEEP (rate-limited per address and IP) |
| GET | `/faucet/status` | Faucet balance and drip stats |
| POST | `/rpc/auth/rotate` | Rotate JWT signing secret |
| GET | `/rpc/auth/audit` | Export Merkle-chained audit log (NDJSON) |
| GET | `/rpc/explorer/blocks` | Block feed for the explorer |
| GET | `/rpc/explorer/validators` | Validator feed for the explorer |
| GET | `/explorer` | Block explorer web UI |
| GET | `/metrics` | Prometheus text-format metrics |

---

## CLI reference

```
bleep <COMMAND>

  wallet create                        SPHINCS+ + Kyber-1024 keypair, AES-256-GCM encrypted
  wallet balance                       Query /rpc/state; fall back to local RocksDB
  wallet import <phrase>               BIP-39 → PBKDF2 → SPHINCS+ keypair
  wallet export                        Print wallet addresses

  tx send --to <addr> --amount <n>     Sign with SPHINCS+ and POST to /rpc/tx
  tx history                           Recent transaction history

  validator stake --amount <n>         Register or increase stake
  validator unstake                    Initiate exit
  validator list                       Active validators
  validator status <id>                Status and slashing history
  validator submit-evidence <file>     Double-sign evidence file

  governance propose <text>            Submit proposal (10,000 BLEEP deposit required)
  governance vote <id> --yes/--no      Stake-weighted ZK vote
  governance list                      All proposals
  governance status <id>               Proposal detail and tally

  block latest                         Latest block
  block get <height>                   Block by height
  block validate <hash>                Validate block hash

  pat create/mint/burn/transfer        PAT token operations
  pat balance <symbol> <address>       Token balance

  oracle price <asset>                 Latest aggregated oracle price
  oracle submit <asset> <price>        Submit a price update

  economics supply                     Circulating supply and burn stats
  economics fee                        Current base fee
  economics epoch <n>                  Epoch economic output

  ai ask <prompt>                      AI advisory query (advisory only)
  ai status                            Inference engine status and approved model hashes

  zkp verify <proof>                   Verify a STARK proof (transparent, post-quantum secure)
  telemetry                            Live node metrics
  info                                 Node version and RPC health
```

---

## Security

### Threat model

**Classical PPT adversary** — targets 256-bit security. All primitives in scope, including STARKs, meet this.

**Quantum QPT adversary (Shor's algorithm)** — SPHINCS+-SHAKE-256f-simple, Kyber-1024, and STARK proofs all hold at Security Level 5. No vulnerable elliptic-curve or pairing-based primitives remain on any consensus-critical path.

**Byzantine validator** — controls f < S/3 of staked supply; may behave arbitrarily including equivocation and selective silence. BFT safety holds unconditionally under this model.

### Independent security audit

Sprint 9 audit: 16,127 lines of Rust across six crates.

| Severity | Total | Resolved |
|---|---|---|
| Critical | 2 | 2 |
| High | 3 | 3 |
| Medium | 4 | 3 (SA-M4 acknowledged — EIP-1559 design property) |
| Low | 3 | 3 |
| Informational | 2 | 1 (SA-I2 acknowledged — NTP guard is a mainnet gate) |

Key findings:

- **SA-C1** (Critical) — `NullifierStore` used an in-memory `HashSet`; double-spend possible after restart → RocksDB `WriteBatch sync=true`
- **SA-C2** (Critical) — JWT rotation accepted low-entropy secrets → Shannon entropy gate ≥ 3.5 bits/byte
- **SA-H2** (High) — Balance check-and-debit had a TOCTOU race → RocksDB compare-and-swap loop, up to 3 retries
- **SA-H3** (High) — No message size limit before deserialisation → 2 MiB gate at receive boundary
- **SA-M1** (Medium) — MPC ceremony accepted unsigned contributions → SPHINCS+ signature over `(id || hash || timestamp)` required
- **SA-L3** (Low) — Secret keys not zeroed on drop → `Zeroizing<Vec<u8>>` on all secret key types

Full report: `docs/SECURITY_AUDIT_SPRINT9.md`

---

## Known limitations

**SPHINCS+ bandwidth.** 7,856-byte signatures (FIPS 205, SLH-DSA, Security Level 5) versus 64 bytes for ECDSA. On a 4,096-transaction block: approximately 32 MB of signatures per block, roughly 87 MB/s minimum bandwidth from signatures alone before payloads and vote messages. This is a real operational cost — standardized aggregation for hash-based signatures remains an open research problem.

**Per-shard BFT tolerance.** Increasing shard count beyond 10 reduces per-shard validator assignment and weakens per-shard fault tolerance. The safe maximum for mainnet depends on final validator set size.

**Tier 2 and Tier 1 bridges.** Implemented and tested against mock verifier sets. Live deployment under governance approval pending.

---

## Roadmap

| Phase | Status | Definition of done |
|---|---|---|
| 1 — Foundation | ✅ Complete | 19-crate workspace compiles; post-quantum crypto active; STARK proofs (transparent, no trusted setup); 4-node devnet; Tier 4 live on Sepolia |
| 2 — Testnet Alpha | ✅ Complete | 7-validator `bleep-testnet-1`; public faucet; block explorer; full CI pipeline |
| 3 — Hardening | ✅ Complete | Security audit (all Critical/High resolved); 72-hour chaos suite; 5-participant MPC; ≥ 10,000 TPS benchmark |
| 4 — AI Training | ⏳ Active | `DeterministicInferenceEngine` passes determinism suite; governance pre-flight ≥ 95% accuracy on constitutional violations |
| 5 — Public Testnet | ⏳ Upcoming | ≥ 50 validators; ≥ 6 continents; 30 consecutive days without manual intervention |
| 6 — Pre-Sale / ICO | ⏳ Upcoming | ICO complete; all vesting contracts deployed and verified on-chain |
| 7 — Mainnet | 🔜 Planned | ≥ 21 validators; governance active from block 1; BLEEP Connect Tier 1–4 live on Ethereum and Solana; NTP guard active; `GenesisAllocation` contracts deployed |

---

## Contributing

```bash
git checkout -b feat/your-feature
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

Open a pull request against `main`. Changes to `bleep-consensus`, `bleep-crypto`, or `bleep-state` receive extended review — these crates sit on the security boundary.

If you are proposing changes to constitutional constants (`MAX_SUPPLY`, `MAX_INFLATION_RATE_BPS`, `FEE_BURN_BPS`, finality threshold), the answer is no unless it comes through a formal protocol amendment. The point of compile-time enforcement is that these do not change at all.

**Security disclosures:** do not open public issues for vulnerabilities. Email `security@bleep.network` with a description, reproduction steps, and a proposed fix. 72-hour acknowledgment; 14-day patch timeline for Critical and High.

---

## License

MIT OR Apache-2.0, at your option. See [LICENSE](LICENSE).

---

*BLEEP · Quantum Trust Network · Protocol Version 3 · `bleep-testnet-1`*

*This document describes Protocol Version 3. It is not financial or investment advice. Protocol parameters may change before mainnet deployment.*├── bleep-interop/        # BLEEP Connect — 10 sub-crates, 4 bridge tiers
│   ├── bleep-connect-types
│   ├── bleep-connect-crypto
│   ├── bleep-connect-adapters
│   ├── bleep-connect-commitment-chain
│   ├── bleep-connect-executor
│   ├── bleep-connect-layer4-instant    # live on Ethereum Sepolia
│   ├── bleep-connect-layer3-zkproof    # live on Ethereum Sepolia
│   ├── bleep-connect-layer2-fullnode   # implemented, mainnet target
│   ├── bleep-connect-layer1-social     # implemented, mainnet target
│   └── bleep-connect-core
├── bleep-pat             # Programmable Asset Token registry
├── bleep-indexer         # DashMap chain indexes, reorg rollback, checkpoints
├── bleep-cli             # clap async CLI
└── bleep-telemetry       # tracing-subscriber, MetricCounter, Prometheus
```

Node entrypoint: `src/bin/main.rs`. Startup follows a 16-step dependency-ordered sequence. Post-quantum keypairs are generated first. `StateManager` opens RocksDB — including `nullifier_store` and `audit_log` column families — before block production logic activates. STARK provers and verifiers are initialized (transparent, no trusted setup required). The node signals readiness only after all 46 RPC endpoints are confirmed active. Any failure in the sequence halts rather than leaving the node partially initialised.

---

## Cryptography

`bleep-crypto` is the root dependency of the protocol. No other crate performs raw cryptographic operations.

### Algorithms

**Transaction and block signing — SPHINCS+-SHAKE-256f-simple (FIPS 205, SLH-DSA, Security Level 5)**

Security reduces to the one-wayness of SHAKE-256. No algebraic structure means no Shor's-algorithm attack surface. The tradeoff is size: 7,856-byte signatures. At 4,096 transactions per block that's ~32 MB of signature data per block — roughly 87 MB/s of bandwidth floor from signatures alone before you count payloads and vote messages. This is a real operational cost, and signature aggregation for hash-based schemes remains an open research problem. See [Limitations](#limitations).

**Key encapsulation — Kyber-1024 / ML-KEM-1024 (FIPS 203, Security Level 5)**

Used for validator binding, inter-node session establishment, and wallet key management. Security reduces to Module-LWE hardness. 1,568-byte public keys are larger than Curve25519's 32 bytes — this shows up in per-session handshake overhead and validator registry storage.

**Symmetric — AES-256-GCM**

Onion routing hops, wallet signing key encryption at rest. The 256-bit key gives ~128 bits of post-quantum security against Grover. Accepted.

**Hashing — SHA3-256, BLAKE3**

SHA3-256: state commitments, Merkle node hashing, block hashing, audit log chaining, AI model binary hashing. BLAKE3: indexer content-addressing (throughput-sensitive path). Grover halves the effective security — accepted at Level 5.

### Key lifecycle

Secret keys are wrapped in `zeroize::Zeroizing<Vec<u8>>` from allocation to deallocation. The `Zeroize` derive macro zeros the backing allocation before the allocator reclaims it, regardless of whether the drop is normal or through stack unwinding. This closes the class of vulnerability where key material lingers in swap or core dumps (audit finding SA-L3).

```rust
use bleep_crypto::{generate_tx_keypair, tx_payload, sign_tx_payload, verify_tx_signature};

let (pk, sk) = generate_tx_keypair();
let payload  = tx_payload(&sender, &receiver, amount, timestamp);
let sig      = sign_tx_payload(&payload, &sk)?;  // 7,856-byte SPHINCS+ signature
assert!(verify_tx_signature(&payload, &sig, &pk));
// sk drops here — Zeroizing zeroes the backing Vec before dealloc
```

`sign_tx_payload` returns the signature, never key bytes. A prior implementation returned raw key material as the signature value; this was corrected before the independent audit.

### Post-Quantum ZK Proofs via Winterfell STARKs

Block validity proofs and cross-chain bridge proofs use **Winterfell STARK proofs** — transparent, hash-based constructions requiring **no trusted setup ceremony**. Security reduces to collision resistance of SHA3-256 and BLAKE3, both resistant to Shor's algorithm. All zero-knowledge proof paths are now post-quantum secure.

```
IN SCOPE (post-quantum secure)              
────────────────────────────────────────────
transaction signing (SPHINCS+-SHAKE-256)   
block signing (SPHINCS+-SHAKE-256)         
block validity proofs (Winterfell STARK)   
Tier 3 bridge proofs (Winterfell STARK)    
P2P authentication (SPHINCS+)              
key encapsulation (Kyber-1024 / ML-KEM)   
```

No classical public-key primitive or pairing-based construction is present on any cryptographically sensitive path. STARK proofs are transparent — no MPC ceremony, no toxic waste, no ceremony participants to compromise.

Five fuzz targets run on every CI build: hash determinism, sign/verify round-trips, Kyber encap/decap, Merkle insertion soundness, state transition fund conservation.

---

## Consensus

### Validator model

Let V = {v₁…vₙ} be the active validator set at epoch e. Each validator holds a SPHINCS+ verification key, a Kyber-1024 encapsulation key, and a stake sᵢ in microBLEEP. S = Σsᵢ.

Safety holds when Byzantine stake f < S/3 — this is the stake-weighted BFT bound, not a participant count. The network model is partial synchrony: safety holds under full asynchrony, liveness requires eventual delivery within Δ. The 3,000 ms slot timer is calibrated against observed testnet propagation latency.

### Block production

Each 3-second slot:

1. Proposer selected with probability ∝ stake fraction — deterministic, no coordinator.
2. `BlockProducer` pulls up to 4,096 transactions from the mempool by fee (descending), applies them to a draft state copy. Any transaction that trips an invariant — overdraft, nonce regression, supply cap breach — is evicted and the block rebuilt.
3. SMT root computed and committed to the block header.
4. Winterfell STARK block validity proof generated (transparent, no trusted setup). The proof attests to structural consistency and proposer possession — full execution validity is independently verified by every validator.
5. Block signed with SPHINCS+ and broadcast.
6. Receiving validators verify STARK proof + SPHINCS+ signature + SMT root transition independently.
7. Prevote, then precommit — each message SPHINCS+-signed.
8. Finalisation at > 6,667 bps of S. Irreversible.
9. Epoch boundary every 1,000 blocks (mainnet) / 100 blocks (testnet): validator rotation, reward distribution, slashing counter reset, governance events.

`ConsensusOrchestrator` selects the mode deterministically at epoch boundaries — the same computation on every honest node. It doesn't coordinate; it computes.

| Mode | Condition | Behaviour |
|---|---|---|
| `PoS-Normal` | Default | 3s slots, stake-proportional proposer |
| `Emergency` | < 67% validator liveness | Reduced quorum, halts if safety bound at risk |
| `Recovery` | Post-partition | Re-anchor to most recent finalised checkpoint |

### Slashing

| Violation | Penalty | Note |
|---|---|---|
| Double-sign | 33% burned, tombstoned | `saturating_sub` throughout — SA-M2 |
| Equivocation | 25% burned | |
| Downtime | 0.1% per consecutive missed block | |
| Tier 4 executor timeout | 30% of executor bond | `EXECUTION_TIMEOUT = 120 s` |

Balance debits use a RocksDB compare-and-swap loop (up to 3 retries), closing the TOCTOU race from audit finding SA-H2.

`LongRangeReorg(10)` and `LongRangeReorg(50)` were rejected at `FinalityManager` in every iteration across the 72-hour adversarial run. Once a block is final, the only path to undoing it requires Byzantine stake ≥ S/3, which triggers the slashing cascade.

---

## State layer

### Storage

Account state lives in a 256-level Sparse Merkle Trie backed by RocksDB. The trie root goes in every block header.

```
Key:   b"acct:" + address_utf8
Value: bincode( AccountState { balance: u128, nonce: u64, code_hash: Option<[u8; 32]> } )
```

`advance_block()` is the commit boundary — all writes buffer until it's called. A crash before `advance_block()` leaves the previous block's state intact.

Three column families handle security-critical operations:

- **`nullifier_store`** — bridge nullifier hashes, `WriteBatch` with `sync=true`. The original implementation used an in-memory `HashSet` that didn't survive restarts, permitting double-spend after a node crash (SA-C1). Fixed.
- **`audit_log`** — SHA3-256 Merkle-chained entries. Each entry's hash covers the previous hash, sequence number, and event fields. Mutating any stored entry fails the chain verification. Survives restarts: chain tip and sequence counter are restored from `audit_meta` on startup, and the most recent 10,000 entries warm the in-memory cache.
- **`audit_meta`** — chain tip and counter for log recovery.

### SMT proofs

The 256-level SMT gives fixed-size proofs — 8,192 bytes for both membership and non-membership, regardless of how many accounts exist. This matters for light clients.

```
Leaf key    = SHA3-256( address_utf8 )
Leaf value  = SHA3-256( abi_encode(address, balance, nonce) )
Interior    = SHA3-256( left_child || right_child )
Empty node  = [0u8; 32]
```

```rust
// full node
let proof = state.prove_account("BLEEP1...");

// light client, no node required
assert!(proof.verify(&known_state_root));
```

`MerkleProof` serialises to JSON and is served at `GET /rpc/proof/{address}`.

### Sharding

10 shards in testnet configuration (`NUM_SHARDS`). `ShardManager` routes by account address hash. Each shard is an independent RocksDB instance. `ShardValidatorAssignment` maps validators to shards per epoch using a deterministic function of the epoch randomness beacon — no privileged authority in the loop.

Cross-shard transactions go through `TwoPhaseCommitCoordinator`. The coordinator shard is derived from the transaction hash, eliminating coordinator election entirely. Stalled coordinators are force-aborted by `cross_shard_timeout_sweep` every 60 seconds. `ShardEpochBinding` commits each shard's state root to the epoch Merkle tree at every boundary.

Increasing shard count beyond 10 reduces per-shard validator assignment and weakens per-shard BFT tolerance. The right number for mainnet depends on the final validator set size — this gets validated in Phase 5.

---

## Execution

`VmRouter` dispatches to one of seven engines. The VM never touches `StateManager` directly — it returns a `StateDiff` that `BlockProducer` applies under a single lock after all calls complete.

| Tier | Engine | Scope | Gas |
|---|---|---|---|
| 1 | Native | Transfer, stake, unstake, governance vote | none |
| 2 | Router | Engine selection, gas validation, circuit breakers | validation only |
| 3 | EVM (SputnikVM) | Ethereum-compatible contracts | Ethereum semantics |
| 4 | WASM (Wasmi) | WASM contracts | configurable fuel |
| 5 | ZK Proof | ZK execution, public input verification | fixed per verifier op |
| 6 | AI-Advised | Pre-execution constraint validation (advisory, off-chain) | deterministic; no gas |
| 7 | Cross-Chain | BLEEP Connect Tier 4 intent dispatch | protocol fee in bps |

Each engine runs behind independent circuit breakers and gas budgets. A failure in one engine doesn't contaminate the others.

---

## Networking

`bleep-p2p` is built on libp2p 0.53. Peer IDs are deterministic hashes of SPHINCS+ public keys — network identity is bound to post-quantum key material.

Every inter-node message is a SPHINCS+-signed `SecureMessage`. Unauthenticated messages are dropped before payload processing. A 2 MiB gate is enforced at the receive boundary before any deserialisation (SA-H3) — this closes the class of memory exhaustion attacks that operate by sending large blobs to trigger allocation before signature verification.

Onion routing uses AES-256-GCM keyed from Kyber-1024 per-hop shared secrets, up to 6 hops.

`PeerScoring` computes a composite trust score in [0.0, 100.0] from success ratio, message rate, latency, and diversity. Scores decay at 0.99× per 300 seconds. Below 40: excluded from gossip relay. Below 55: excluded from onion relay.

| Component | Detail |
|---|---|
| DHT | Kademlia, k=20, XOR metric |
| Gossip | Epidemic dissemination, fanout 8 — O(log n) rounds |
| Onion routing | Kyber-1024 KEM per hop, AES-256-GCM payload, max 6 hops |
| Message gate | 2 MiB enforced before deserialisation |
| Trust | Composite score, exponential decay, Sybil resistance |

---

## Governance

Proposal lifecycle: **Submit → AIConstraintValidator pre-flight → Active → Tally → Execute → Record**

The `AIConstraintValidator` pre-flight runs before a proposal enters the vote queue. It checks proposals against the four constitutional invariants — a proposal that would push `MaxInflationBps` above 500 is rejected at this stage and never reaches a vote. The `constitutional_violation_rejected_at_submission` test covers this on every CI build.

Governance parameters on testnet:

| Parameter | Value |
|---|---|
| Voting window | 1,000 blocks (~50 min) |
| Quorum | 1,000 bps (10% minimum stake participation) |
| Pass threshold | 6,667 bps (66.67% of participating stake) |
| Veto threshold | 3,333 bps |
| Minimum deposit | 10,000 BLEEP |

`ZKVotingEngine` provides privacy-preserving stake-weighted votes. Validator weight: 1.0×. Delegator: 0.5×. Community holder: 0.1×. `EligibilityProof` establishes voting power without revealing identity. `TallyProof` allows independent verification without learning individual votes.

`ForklessUpgradeEngine` activates hash-committed upgrades at epoch boundaries only — validators know exactly what code will activate before casting a vote. `Version.is_valid_upgrade()` enforces monotonic version progression. Partial upgrade payloads are rejected atomically.

`proposal-testnet-001` ran the full lifecycle on `bleep-testnet-1`: pre-flight, ZK votes from 7 validators, 70% quorum, on-chain execution at block 1,105. It reduced `FeeBurnBps` from 2,500 to 2,000.

---

## Economics

### Token parameters

| Parameter | Value | Source constant |
|---|---|---|
| Max supply (†) | 200,000,000 BLEEP | `MAX_SUPPLY` |
| Decimals | 8 (1 BLEEP = 10⁸ microBLEEP) | |
| Initial circulating supply | 25,000,000 (12.5%) | `INITIAL_CIRCULATING_SUPPLY` |
| Max per-epoch inflation (†) | 500 bps | `MAX_INFLATION_RATE_BPS` |
| Fee burn (†) | 2,500 bps (25%) | `FEE_BURN_BPS` |
| Validator reward | 5,000 bps (50%) | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury | 2,500 bps (25%) | `FEE_TREASURY_BPS` |
| Min base fee | 1,000 microBLEEP | `MIN_BASE_FEE` |
| Max base fee | 10,000,000,000 microBLEEP | `MAX_BASE_FEE` |
| Max base fee change/block | 1,250 bps | `max_increase_bps` |

*(†) enforced by compile-time `const` assertion — violating the assertion fails the build*

A `const` assertion in `distribution.rs` also verifies that Burn + Validator + Treasury = 10,000 bps exactly. The token distribution sum across all six allocation buckets is verified to equal `MAX_SUPPLY` at compile time.

### Fee market

EIP-1559-style, targeting 50% block capacity. Above target: base fee increases up to 12.5% per block. Below: decreases up to 12.5%. The 25% burn creates deflationary pressure under load — at sustained throughput above 10,000 TPS, annual burn exceeds Year 5+ validator emission (2,400,000 BLEEP/year).

Audit finding SA-M4 (acknowledged, Medium): a proposer with consecutive slots can pin the base fee near maximum by filling blocks. PoS rotation limits the duration any single validator can sustain this. Documented in `THREAT_MODEL.md`.

### Emission schedule

| Year | Rate | Annual emission |
|---|---|---|
| 1 | 12% | 7,200,000 BLEEP |
| 2 | 10% | 6,000,000 |
| 3 | 8% | 4,800,000 |
| 4 | 6% | 3,600,000 |
| 5+ | 4% | 2,400,000/yr |

Encoded as `VALIDATOR_EMISSION_YEAR` in `tokenomics.rs`. Not a governance parameter — changing it requires a software upgrade.

### Token distribution

| Allocation | Tokens | Launch unlock | Vesting |
|---|---|---|---|
| Validator Rewards | 60,000,000 (30%) | 10,000,000 | Emission schedule |
| Ecosystem Fund | 50,000,000 (25%) | 5,000,000 | 10-year linear, governance disbursement |
| Community Incentives | 30,000,000 (15%) | 5,000,000 | Governance-triggered |
| Foundation Treasury | 30,000,000 (15%) | 5,000,000 | 6-year linear, governance spending |
| Core Contributors | 20,000,000 (10%) | 0 | 1-year cliff + 4-year linear, immutable contract |
| Strategic Reserve | 10,000,000 (5%) | 0 | Governance unlock, proposal + vote required |

`LinearVestingSchedule` contracts for core contributors are immutable from deployment — terms can't be modified after genesis.

### Game-theoretic safety verifier

`SafetyVerifier` in `bleep-economics/src/game_theory.rs` evaluates five attack models at current protocol parameters: equivocation, censorship, non-participation, griefing, cartel formation. Each returns `attacker_profit`, `network_cost`, `is_profitable`. **The CI build fails if any model returns `is_profitable = true`.** This is the economic equivalent of the compile-time constitutional assertions.

---

## BLEEP Connect

Four bridge tiers with different trust models. No tier requires a permanently privileged operator.

| Tier | Mechanism | Latency | Security basis | Status |
|---|---|---|---|---|
| 4 — Instant | Executor auction + escrow | 200 ms – 1 s | Economic: 30% bond slashed on timeout | Live — Ethereum Sepolia |
| 3 — ZK Proof | STARK batch proof | 10 – 30 s | Cryptographic: Winterfell STARK (transparent, post-quantum secure) | Live — Ethereum Sepolia |
| 2 — Full-Node | Multi-client verification | Hours | 90% consensus across ≥ 3 independent nodes | Implemented, mainnet target |
| 1 — Social | Stakeholder governance | 7 days / 24 h (emergency) | Full governance consensus | Implemented, mainnet target |

These are parallel options with different trust models, not deployment stages. Choose based on transfer value and time sensitivity.

**Tier 4** — `InstantIntent` enters a 15-second executor auction (`EXECUTOR_AUCTION_DURATION`). Winner fulfils within 120 seconds or loses 30% of their bond. Protocol fee: 10 bps. Security is economic — don't route transfers that approach executor bond size through Tier 4.

**Tier 3** — Batches up to 32 intents (`L3_BATCH_SIZE`) into a single Winterfell STARK proof, submitted to `BleepL3Bridge` on Sepolia (~250,000 gas). No trusted operator. **Post-quantum secure** — transparent STARK proofs require no ceremony, security reduces to hash collision resistance.

Double-spend prevention in Tier 3: `GlobalNullifierSet` performs an atomic `WriteBatch sync=true` on first submission and returns `Err(NullifierAlreadySpent)` on any duplicate. The original implementation used an in-memory `HashSet` that didn't survive restarts (SA-C1). Fixed.

**Tier 2** — Requires 90% consensus (`CONSENSUS_THRESHOLD`) across ≥ 3 independent verifier nodes (`MIN_VERIFIER_NODES`) running different client implementations. Nodes query the actual on-chain state root independently. Optional Intel SGX attestation for high-value transfers. This tier avoids pairing-based cryptography entirely.

**Tier 1** — Stakeholder governance for scenarios that defeat lower tiers: chain reorganisations, detected quantum attacks, smart contract bugs at bridge scale. Standard: 7-day window, 66% threshold. Emergency (`EmergencyPause`, `StateRollback`): 24-hour window, 80% threshold.

---

## AI advisory

Two components with clearly separated scopes. Neither has write access to chain state, the governance queue, or the block production pipeline.

### AIConstraintValidator (operational)

A deterministic rule engine. Not a trained model. Checks governance proposals against the four constitutional invariants before they enter the vote queue. Rejects proposals that would violate compile-time invariants before they consume any validator attention. The `constitutional_violation_rejected_at_submission` test runs on every CI build.

`AIConsensusOrchestrator` produces advisory healing proposals (six typed proposal types: `ConsensusModeProposal`, `ShardRollbackProposal`, `ShardRebalanceProposal`, `ValidatorSecurityProposal`, `TokenomicsProposal`, `GovernancePriorityProposal`). The consensus layer decides whether to act on them. The orchestrator has no unilateral authority.

### DeterministicInferenceEngine (under development)

ONNX-based runtime. Enforces six invariants for any model operating on a consensus-critical path — these aren't quality controls, they're determinism requirements:

1. **Model hash verification** — SHA3-256 of the model binary must match `ModelMetadata.model_hash` before any inference. Mismatch returns an explicit error.
2. **Deterministic input normalisation** — fixed mean, standard deviation, clamp applied to all inputs before inference.
3. **Deterministic output rounding** — configurable decimal precision; floating-point variance doesn't propagate to outputs.
4. **CPU-only execution** — no GPU paths. GPU floating-point produces non-determinism across different hardware even with identical inputs.
5. **Governance-approval gating** — `approval_epoch` must be set by governance before any model touches a consensus-critical path. Ungated models are rejected at the call site.
6. **No dynamic loading** — model versions are immutable once deployed. Replacement requires a governance proposal and vote.

Every inference produces an `InferenceRecord` with model hash, normalised inputs, raw outputs, and deterministic seed. `AIAttestationManager` records each output as an `AIAttestationRecord` with commitment `SHA3-256(model_hash || inputs_hash || output_hash || epoch)`.

**No trained model is currently deployed in any governance-critical or consensus-critical path.** The Phase 4 description above is the design, not the current deployment state.

---

## Running a node

### Prerequisites

```bash
# Rust stable toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Ubuntu / Debian
sudo apt-get install -y build-essential clang libclang-dev librocksdb-dev

# macOS
brew install rocksdb llvm
export LIBRARY_PATH="$(brew --prefix rocksdb)/lib:$LIBRARY_PATH"
```

### Build

```bash
git clone https://github.com/bleep-project/bleep.git
cd bleep

# Full workspace
cargo build --release --workspace

# Node binary only
cargo build --release --bin bleep

# CLI only
cargo build --release --bin bleep-cli
```

### Start a local node

```bash
./target/release/bleep
# 16-step startup sequence, RPC at :8545, P2P at :7700

curl -s http://127.0.0.1:8545/rpc/health | jq .
```

### Create a wallet and send a transaction

```bash
# Generate SPHINCS+ + Kyber-1024 keypair, encrypted with AES-256-GCM
./target/release/bleep-cli wallet create

# Check balance (falls back to local RocksDB if node unreachable)
./target/release/bleep-cli wallet balance

# Import from BIP-39 mnemonic (PBKDF2-HMAC-SHA512, 2,048 rounds)
./target/release/bleep-cli wallet import "word1 word2 ... word12"

# Send — prompts for passphrase to decrypt the SPHINCS+ signing key
./target/release/bleep-cli tx send \
  --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 \
  --amount 1000
```

Wallet file layout (`~/.bleep/wallets.json`):

```json
[
  {
    "falcon_keys":  "<hex: SPHINCS+ public key>",
    "kyber_keys":   "<hex: Kyber-1024 public key>",
    "signing_key":  "<hex: AES-256-GCM( SPHINCS+ secret key )>",
    "address":      "BLEEP1<40-hex-chars>",
    "label":        null
  }
]
```

`signing_key` blob: `nonce(12 bytes) || ciphertext || GCM-tag(16 bytes)`. Nonce is freshly generated on every `lock()` call — two encryptions of the same key produce different blobs.

Address format: `BLEEP1` + lower_hex(SHA256(SHA256(public_key))[..20])

### Run tests

```bash
cargo test --workspace

# Security-critical crates
cargo test -p bleep-crypto
cargo test -p bleep-state
cargo test -p bleep-consensus
cargo test -p bleep-wallet-core

# Fuzz (requires nightly + cargo-fuzz)
cargo fuzz run hash_determinism
cargo fuzz run sign_verify_roundtrip
cargo fuzz run kyber_encap_decap
cargo fuzz run merkle_insertion_soundness
cargo fuzz run state_transition_fund_conservation
```

### Executor node (Tier 4 bridge)

```bash
BLEEP_EXECUTOR_KEY=<32-byte-hex-seed>            \
BLEEP_EXECUTOR_CAPITAL_BLEEP=100000000000        \
BLEEP_EXECUTOR_CAPITAL_ETH=10000000000000000000  \
BLEEP_EXECUTOR_RISK=Medium                       \
BLEEP_RPC=http://your-node:8545                  \
./target/release/bleep-executor
```

---

## Configuration

```toml
# bleep.toml

[node]
p2p_port  = 7700
rpc_port  = 8545
state_dir = "/var/lib/bleep/state"
log_level = "info"

[consensus]
block_interval_ms = 3000
max_txs_per_block = 4096
blocks_per_epoch  = 1000   # 100 on testnet
validator_id      = "validator-0"

[features]
quantum = true   # do not disable on any network-connected node
```

| Env var | Default | Purpose |
|---|---|---|
| `BLEEP_RPC` | `http://127.0.0.1:8545` | RPC endpoint for CLI commands |
| `BLEEP_STATE_DIR` | `/tmp/bleep-state` | Local RocksDB path (offline balance fallback) |
| `RUST_LOG` | `info` | tracing log filter |

Cargo feature flags: `mainnet` (default, on), `testnet` (off), `quantum` (default, on — enables `pqcrypto` and `pqcrypto-kyber`).

---

## RPC reference

Port **8545**. All responses are JSON. Errors carry an `"error"` string field.

### Node

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/health` | Status, height, peer count, uptime, version |
| `GET` | `/rpc/telemetry` | Blocks produced, transactions processed, uptime |
| `GET` | `/rpc/block/latest` | Latest block height and hash |
| `GET` | `/rpc/block/{id}` | Block by height or hash |
| `POST` | `/rpc/tx/submit` | Submit a signed transaction |
| `GET` | `/rpc/state/{address}` | Balance, nonce, state root, block height |
| `GET` | `/rpc/proof/{address}` | 256-level SMT inclusion/exclusion proof (8,192 bytes) |

### Validators

| Method | Path | Description |
|---|---|---|
| `POST` | `/rpc/validator/stake` | Register or increase stake |
| `POST` | `/rpc/validator/unstake` | Initiate graceful exit |
| `GET` | `/rpc/validator/list` | Active validators with stake |
| `GET` | `/rpc/validator/status/{id}` | Status and slashing history |
| `POST` | `/rpc/validator/evidence` | Submit double-sign evidence — triggers immediate slashing |

### Economics and oracle

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/economics/supply` | Circulating supply, minted, burned, base fee |
| `GET` | `/rpc/economics/fee` | Current base fee + last epoch |
| `GET` | `/rpc/economics/epoch/{n}` | Epoch output: emissions, burns, price |
| `GET` | `/rpc/oracle/price/{asset}` | Aggregated oracle price (median, sources) |
| `POST` | `/rpc/oracle/update` | Submit price update |

### BLEEP Connect

| Method | Path | Description |
|---|---|---|
| `POST` | `/rpc/connect/intent` | Submit instant intent (Tier 4) |
| `GET` | `/rpc/connect/intent/{id}` | Intent status |
| `GET` | `/rpc/connect/intents/pending` | All pending Tier 4 intents |

### AI

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/ai/attestations/{epoch}` | AI attestation records for epoch |

### Example responses

```bash
GET /rpc/health
{
  "status": "ok",
  "height": 1024,
  "peers": 8,
  "uptime_secs": 3600,
  "version": "3.0.0"
}

GET /rpc/state/BLEEP1...
{
  "address": "BLEEP1a3f7b...",
  "balance": "10000000000",   # decimal string — avoids JSON u128 overflow
  "nonce": 4,
  "state_root": "8a3f2c1d...",
  "block_height": 1024
}
```

Verify a proof offline:

```rust
let proof: MerkleProof = serde_json::from_str(&response_body)?;
assert!(proof.verify(&known_state_root));
```

---

## CLI reference

```
bleep-cli <COMMAND>

  start-node                           Start a full node

  wallet create                        New SPHINCS+ + Kyber-1024 keypair, AES-256-GCM encrypted
  wallet balance                       Query /rpc/state, fall back to local RocksDB
  wallet import <phrase>               BIP-39 -> PBKDF2 -> SPHINCS+ keypair
  wallet export                        Print wallet addresses

  tx send --to <addr> --amount <n>     Sign (SPHINCS+) and POST /rpc/tx/submit
  tx history                           Transaction history

  validator stake --amount <n>         Register or increase stake
  validator unstake                    Initiate exit
  validator list                       Active validators
  validator status <id>                Status + slashing history
  validator submit-evidence <file>     Double-sign evidence

  governance propose <text>            Submit a proposal (requires deposit)
  governance vote <id> --yes/--no      Cast stake-weighted ZK vote
  governance list                      All proposals
  governance status <id>               Proposal detail

  block latest                         Latest block
  block get <id>                       Block by hash or height
  block validate <hash>                Validate block hash

  state snapshot                       RocksDB snapshot
  state restore <path>                 Restore from snapshot

  zkp <proof>                          Verify a STARK proof (transparent, post-quantum)
  ai ask <prompt>                      AI advisory query (advisory only)
  ai status                            Engine status + approved model hashes
  ai attestations <epoch>              Attestation records for epoch

  pat mint/burn/transfer/balance       PAT token operations
  oracle price/submit                  Oracle price queries and updates
  economics supply/fee/epoch           Tokenomics queries
  telemetry                            Live node metrics
  info                                 Node version and RPC health
```

---

## Security

### Threat model

Three adversary classes:

**Classical PPT** — targets 256-bit security. All primitives in scope, including STARKs, meet this.

**Quantum QPT (Shor's algorithm)** — SPHINCS+-SHAKE-256f-simple, Kyber-1024/ML-KEM-1024, and STARK proofs all hold at Security Level 5. No vulnerable elliptic-curve or pairing-based primitives remain.

**Byzantine validator** — controls f < S/3 of staked supply, may behave arbitrarily including equivocation and selective silence. BFT safety holds unconditionally under this model.

### Independent audit

Sprint 9 audit covered 16,127 lines of Rust across six crates.

| Severity | Total | Resolved | Status |
|---|---|---|---|
| Critical | 2 | 2 | ✓ |
| High | 3 | 3 | ✓ |
| Medium | 4 | 3 | SA-M4 acknowledged — EIP-1559 design property |
| Low | 3 | 3 | ✓ |
| Informational | 2 | 1 | SA-I2 — NTP drift guard is a mainnet activation gate |

Key findings and resolutions:

- **SA-C1** (Critical) — `NullifierStore` used an in-memory `HashSet`; double-spend possible after restart → replaced with RocksDB `WriteBatch sync=true`
- **SA-C2** (Critical) — JWT rotation accepted low-entropy secrets → Shannon entropy gate ≥ 3.5 bits/byte enforced on all rotation
- **SA-H2** (High) — Balance check-and-debit had a TOCTOU race → RocksDB compare-and-swap loop, up to 3 retries
- **SA-H3** (High) — No message size limit before deserialisation → 2 MiB gate at receive boundary
- **SA-M1** (Medium) — STARK proofs are transparent and require no ceremony. Previous implementation planning removed.
- **SA-M2** (Medium) — Slash arithmetic could underflow → all slash arithmetic uses `saturating_sub`
- **SA-L3** (Low) — Secret keys persisted in memory after drop → all secret key types wrapped in `Zeroizing<Vec<u8>>`

Full report: `docs/SECURITY_AUDIT_SPRINT9.md`

### Adversarial test suite (72-hour continuous, 7 validators)

| Scenario | Result |
|---|---|
| `ValidatorCrash(1)`, `ValidatorCrash(2)` | Pass — consensus resumed within expected recovery window |
| `ValidatorCrash(3)` | Correctly halts — f=3 ≥ 2.33 violates BFT bound |
| `NetworkPartition(4/3)`, `NetworkPartition(5/2)` | Pass — majority partition continued, healed cleanly |
| `LongRangeReorg(10)`, `LongRangeReorg(50)` | Pass — rejected at `FinalityManager` (invariant I-CON3) |
| `DoubleSign(validator-0)`, `DoubleSign(validator-3)` | Pass — 33% slashed, tombstoned |
| `TxReplay` | Pass — rejected by nonce check (I-S5) |
| `EclipseAttack(validator-6)` | Pass — Kademlia k=20 + DNS seeds |
| `InvalidBlockFlood(1000)` | Pass — rejected at SPHINCS+ gate, peer rate-limited |
| `LoadStress(1,000 / 5,000 / 10,000 TPS, 60s)` | Pass — block capacity saturated at max without drops |

---

## Limitations

These are stated plainly. If you're evaluating BLEEP for production use, these are the things that matter most.

## Limitations

**SPHINCS+ bandwidth** — 7,856-byte signatures when 64 bytes for ECDSA. On a 4,096-transaction block: ~32 MB of signature data. This is real operational cost. Standardized aggregation for hash-based signatures remains open research.

**Per-shard BFT tolerance** — Increasing shard count beyond 10 reduces per-shard validator assignment and weakens fault tolerance.

**SPHINCS+ bandwidth.** 7,856-byte signatures vs 64 bytes for ECDSA. On a 4,096-tx block: ~32 MB of signatures per block, ~87 MB/s minimum bandwidth from signatures alone before payloads and vote messages. Signature aggregation for hash-based schemes is an open research problem. No standardised solution exists.

**Trusted setup.** STARKs require no trusted setup ceremony. Proofs are transparent and do not rely on any ceremony.

**Per-shard BFT tolerance.** Increasing shard count beyond 10 reduces per-shard validator assignment. The safe maximum for mainnet depends on final validator count and needs validation at realistic network size (Phase 5 milestone).

**AI components are pre-production.** No trained ONNX model is deployed in any consensus-critical path. The `DeterministicInferenceEngine` description in this document is the design, not the deployment state. The determinism invariants can't be empirically confirmed until a model is deployed.

**Tier 2 and Tier 1 bridge not yet live.** Implemented and tested against mock verifier sets. Not yet deployed in a live multi-party environment. Mainnet target.

**NTP drift guard not enforced at testnet startup.** Implemented (warn > 1 s, halt > 30 s). Activated as a mainnet gate (SA-I2).

---

## Roadmap

| Phase | Status | Definition of done |
|---|---|---|
| 1 — Foundation | ✅ Complete | 19 crates compile; post-quantum crypto active; STARK proofs (transparent, no trusted setup); 4-node devnet; Tier 4 live on Sepolia |
| 2 — Testnet Alpha | ✅ Complete | 7-validator `bleep-testnet-1`, public faucet, block explorer, full CI pipeline |
| 3 — Protocol Hardening | ✅ Complete | Security audit (all critical/high resolved), 72-hour chaos suite, MPC ceremony, ≥10,000 TPS benchmark |
| 4 — AI Model Training | ⏳ Active | `DeterministicInferenceEngine` passes determinism suite, governance pre-flight ≥ 95% accuracy |
| 5 — Public Testnet | ⏳ Upcoming | ≥ 50 validators, ≥ 6 continents, 30 consecutive days without manual intervention |
| 6 — Pre-Sale / ICO | ⏳ Upcoming | ICO complete, all vesting contracts deployed and verified on-chain |
| 7 — Mainnet | 🔜 Planned | ≥ 21 validators, governance active from block 1, BLEEP Connect Tier 1–4 live, NTP guard active |

Mainnet hard requirements: ≥ 21 validators with geographic diversity, governance active from block 1, BLEEP Connect Tier 1–4 live on Ethereum and Solana, NTP drift guard activated (SA-I2), `GenesisAllocation` vesting contracts deployed, client SDKs shipped.

---

## Contributing

```bash
git checkout -b feat/your-feature
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

Open a PR against `main` with a clear description of the change and which crates it affects.

Changes to `bleep-consensus`, `bleep-crypto`, or `bleep-state` get extended review. These crates sit on the security boundary. If you're making changes to constitutional constants (`MAX_SUPPLY`, `MAX_INFLATION_RATE_BPS`, `FEE_BURN_BPS`, finality threshold), the answer is no unless it comes through a formal protocol amendment — the point of compile-time enforcement is that these don't change at all.

**Security disclosures:** Don't open public issues for vulnerabilities. Email `security@bleep.network` with a description, steps to reproduce, and a proposed fix. 72-hour acknowledgment target, 14-day patch timeline for Critical and High.

---

## License

MIT OR Apache-2.0, at your option. See [LICENSE](LICENSE).

---

*BLEEP · Quantum Trust Network · Protocol Version 3 · `bleep-testnet-1`*

*This document describes Protocol Version 3. It is not financial or investment advice. Protocol parameters may change before mainnet deployment.*
This invariant constrains the use of AI-assisted components: any model operating on a consensus-critical path must produce byte-identical outputs given identical inputs, be identified by a governance-approved SHA3-256 hash, and use deterministic feature extraction and output rounding.

### Constitutional Immutability

Four parameters cannot be altered by any governance vote or software upgrade. They are enforced by Rust `const` assertions that prevent compilation if violated:

| Parameter | Value | Constant |
|---|---|---|
| Maximum token supply | 200,000,000 BLEEP | `MAX_SUPPLY` in `tokenomics.rs` |
| Minimum finality threshold | > 6,667 bps of total stake | `FinalityManager` |
| Maximum per-epoch inflation | 500 bps (5%) | `MAX_INFLATION_RATE_BPS` |
| Fee burn floor | 2,500 bps (25%) | `FEE_BURN_BPS` in `distribution.rs` |

A code change violating any of these assertions does not compile.

### Separation of Concerns

Each of the 19 workspace crates has a single defined responsibility. The inter-crate dependency graph is **acyclic**, enforced at build time. The cryptographic subsystem (`bleep-crypto`) has no dependencies on other BLEEP crates. Consensus depends on cryptography and state. The execution environment depends on state but not on consensus directly. A vulnerability in the networking component cannot directly access private key material.

### Auditability by Default

Every security-relevant event is written to a tamper-evident audit log backed by RocksDB with synchronous writes (`sync=true`). Log entries are SHA3-256 Merkle-chained: each entry's hash is computed over the concatenation of the previous hash, sequence number, and event fields. Mutating any stored entry causes chain verification to return `false`. The log survives node restarts.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         BLEEP Node  (src/bin/main.rs)                       │
│                                                                             │
│  ┌──────────────┐  ┌─────────────────────────┐  ┌────────────────────────┐ │
│  │  bleep-cli   │  │       bleep-rpc          │  │   bleep-telemetry      │ │
│  │  clap async  │  │  warp HTTP/JSON :8545    │  │  tracing + metrics     │ │
│  └──────┬───────┘  └───────────┬─────────────┘  └────────────────────────┘ │
│         │                      │                                            │
│  ┌──────▼──────────────────────▼──────────────────────────────────────────┐ │
│  │                        bleep-consensus                                 │ │
│  │  BlockProducer (3,000 ms slots · 4,096 tx/block)                      │ │
│  │  ConsensusOrchestrator — PoS-Normal | Emergency                       │ │
│  │  FinalityManager (>6,667 bps) · SlashingEngine · EpochConfig          │ │
│  │  SelfHealingOrchestrator · FaultDetector                              │ │
│  └──────┬────────────────────────────────┬───────────────────────────────┘ │
│         │                                │                                  │
│  ┌──────▼──────────┐          ┌──────────▼──────────┐                      │
│  │   bleep-core    │          │      bleep-vm        │                      │
│  │  Block · Tx     │          │  7-engine dispatcher │                      │
│  │  Blockchain     │          │  EVM (SputnikVM)     │                      │
│  │  BlockValidator │          │  WASM (Wasmi)        │                      │
│  │  Mempool        │          │  ZK · AI · CrossChain│                      │
│  └──────┬──────────┘          └──────────┬───────────┘                      │
│         │                                │                                  │
│  ┌──────▼────────────────────────────────▼──────────────────────────────┐  │
│  │                          bleep-state                                  │  │
│  │  StateManager (RocksDB · sync writes)                                │  │
│  │  SparseMerkleTrie (256-level · 8,192-byte fixed-size proofs)         │  │
│  │  Sharding · Cross-shard 2PC · NullifierStore · AuditLog              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────────────┐  │
│  │    bleep-p2p     │  │   bleep-crypto   │  │     bleep-interop       │  │
│  │  Kademlia k=20   │  │  SPHINCS+-SHAKE  │  │  BLEEP Connect (4 tiers)│  │
│  │  Gossip fanout 8 │  │  256f-simple     │  │  10 sub-crates          │  │
│  │  Onion routing   │  │  Kyber-1024      │  │  ETH Sepolia live       │  │
│  │  64 MiB msg gate  │  │  AES-256-GCM     │  │  Winterfell STARK bridge   │  │
│  └──────────────────┘  │  SHA3-256/BLAKE3 │  └─────────────────────────┘  │
│                         └──────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Dependency Order (Acyclic)

```
bleep-crypto  ←  bleep-zkp  ←  bleep-wallet-core
     ↓
bleep-state   ←  bleep-indexer
     ↓
bleep-consensus  ←  bleep-scheduler
     ↓
bleep-vm  ←  bleep-pat  ←  bleep-ai
     ↓
bleep-p2p  ←  bleep-auth  ←  bleep-rpc
     ↓
bleep-interop (10 sub-crates)
     ↓
bleep-economics  ←  bleep-governance
     ↓
bleep-cli  ←  bleep-telemetry
```

### Node Startup Sequence

A node follows a 16-step dependency-ordered startup sequence. Post-quantum keypairs are generated first. `StateManager` opens its RocksDB instance — including `nullifier_store` and `audit_log` column families — before any block production logic activates. STARK provers and verifiers are initialized (transparent, no ceremony required). The node emits a readiness signal only after all 46 RPC endpoints are confirmed active. **Any startup failure halts the node rather than leaving it partially initialised.**

---

## Crate Map

| Crate | Responsibility |
|---|---|
| `bleep-crypto` | SPHINCS+-SHAKE-256f-simple signatures, Kyber-1024 KEM, AES-256-GCM, SHA3-256, BLAKE3 — the root dependency of the entire protocol |
| `bleep-zkp` | Winterfell STARK circuit definitions (transparent, hash-based), prover/verifier API, no trusted setup required |
| `bleep-wallet-core` | `EncryptedWallet`, `Zeroizing<Vec<u8>>` key storage, AES-256-GCM key-at-rest, `WalletManager` |
| `bleep-consensus` | `BlockProducer`, `ConsensusOrchestrator` (PoS-Normal / Emergency), `FinalityManager`, `SlashingEngine`, `EpochConfig`, `SelfHealingOrchestrator` |
| `bleep-scheduler` | 20-task Tokio scheduler: epoch, rewards, healing, governance, fee market, supply invariant, shard rebalancing, timeout sweeps, indexer checkpoints, audit rotation |
| `bleep-state` | `StateManager` (RocksDB), `SparseMerkleTrie` (256-level), sharding, cross-shard 2PC, `NullifierStore` (WriteBatch `sync=true`), `AuditLog` |
| `bleep-indexer` | `DashMap`-backed chain indexes, reorg rollback, checkpoint engine |
| `bleep-vm` | 7-engine intent dispatcher: Native, Router, EVM (SputnikVM), WASM (Wasmi), ZK, AI-Advised, Cross-Chain |
| `bleep-pat` | Programmable Asset Token registry |
| `bleep-ai` | `AIConstraintValidator` (Phase 3, deterministic rule engine), `DeterministicInferenceEngine` (Phase 4, ONNX-based, under development), `AIAttestationManager` |
| `bleep-p2p` | Kademlia DHT (k=20), epidemic gossip (fanout 8), onion routing (AES-256-GCM per hop, Kyber-1024 KEM), `PeerScoring`, 2 MiB message gate |
| `bleep-auth` | Salted SHA3-256 credentials, JWT sessions, RBAC, Kyber-1024 validator binding, tamper-evident audit log, per-identity rate limiting |
| `bleep-rpc` | warp HTTP/JSON server, 46 endpoints, `RpcState` with live `StateManager` and `ValidatorRegistry` |
| `bleep-core` | `Block`, `Transaction`, `Blockchain`, `Mempool`, `TransactionPool`, `BlockValidator` |
| `bleep-governance` | `LiveGovernanceEngine`, `ZKVotingEngine`, `ForklessUpgradeEngine`, proposal lifecycle |
| `bleep-economics` | EIP-1559 base fee market, `FeeDistribution`, `SafetyVerifier`, validator emission schedule, oracle bridge |
| `bleep-interop` | BLEEP Connect (10 sub-crates, 4 tiers): Tier 4 executor auction live on Ethereum Sepolia, Tier 3 STARK bridge live on Ethereum Sepolia (post-quantum secure, transparent) |
| `bleep-cli` | `clap` async CLI — full operator interface for all subsystems |
| `bleep-telemetry` | `tracing-subscriber`, `MetricCounter`, `MetricGauge`, Prometheus export |

---

## Cryptographic Model

### Algorithm Selection

| Property | SPHINCS+-SHAKE-256f-simple | Kyber-1024 (ML-KEM-1024) |
|---|---|---|
| NIST standard | FIPS 205 (SLH-DSA) | FIPS 203 (ML-KEM) |
| Role | Transaction signing, block signing, P2P authentication | Validator binding, peer KEM, wallet key management |
| Security assumption | One-wayness of SHAKE-256 (hash-based; no algebraic structure) | Hardness of Module-LWE (lattice-based) |
| NIST security level | Level 5 (≥ 256-bit post-quantum) | Level 5 (≥ 256-bit post-quantum) |
| Public key size | 32 bytes | 1,568 bytes |
| Secret key | 64 bytes (`Zeroizing<Vec<u8>>` on drop) | 3,168 bytes (`Zeroizing<Vec<u8>>` on drop) |
| Output | 7,856-byte detached signature | 1,568-byte ciphertext + 32-byte shared secret |

SPHINCS+ is selected for its conservative security assumptions: security reduces to the one-wayness of the hash function with no reliance on algebraic structure. The tradeoff is large signatures (7,856 bytes at Level 5). At sustained 10,000 TPS with 4,096 transactions per block, the per-block SPHINCS+ overhead is approximately 32 MB — a significant bandwidth constraint documented in [Known Limitations](#known-limitations).

### Key Material Lifecycle

Secret keys are wrapped in `zeroize::Zeroizing<Vec<u8>>` from allocation to deallocation. The `Zeroize` derive macro zeros the backing allocation before the allocator reclaims it, regardless of whether the key is dropped normally or through stack unwinding. This prevents key material from persisting in swap or core dumps (audit finding SA-L3).

```rust
// bleep-crypto/src/pq_crypto.rs
// Secret keys are never exposed as raw bytes outside this crate.
// sign_tx_payload invokes sphincsshake256fsimple::detached_sign
// and returns a 7,856-byte signature — never key material.
use bleep_crypto::{sign_tx_payload, verify_tx_signature, tx_payload, generate_tx_keypair};

let (pk, sk) = generate_tx_keypair();
let payload  = tx_payload(&sender, &receiver, amount, timestamp);
let sig      = sign_tx_payload(&payload, &sk)?;
assert!(verify_tx_signature(&payload, &sig, &pk));
```

### Hash Functions

SHA3-256 handles state commitments, Merkle node hashing, block hashing, audit log chaining, and AI model binary hashing. BLAKE3 handles high-throughput indexer content-addressing. Grover's algorithm reduces quantum security of these functions from 256 bits to approximately 128 bits — a weakening accepted at Security Level 5.

Five fuzz targets in `bleep-crypto/fuzz` run on every CI build: hash determinism, sign/verify round-trips, Kyber encap/decap, Merkle insertion soundness, and state transition fund conservation.

### Winterfell STARK Proofs — Transparent, Post-Quantum Secure

Block validity proofs and cross-chain bridge proofs use Winterfell STARK proofs: transparent, hash-based constructions requiring **no trusted setup ceremony**. Security reduces to collision resistance of SHA3-256 and BLAKE3 — both resistant to Shor's algorithm.

Audit finding SA-M1 identified that the original ceremony accepted unsigned contributions, permitting substitution attacks. The corrected implementation requires each contribution to carry a SPHINCS+ signature over `(id || hash || timestamp)`.

The node verifies the SRS against the MPC transcript on startup before any ZK operations. A mismatch halts the node.

### The Post-Quantum Boundary

```
┌─────────────────────────────────────────────────┐
│            POST-QUANTUM BOUNDARY                │
│                                                 │
│  Transaction signing      SPHINCS+-SHAKE-256f   │
│  Block signing            SPHINCS+-SHAKE-256f   │
│  P2P authentication       SPHINCS+-SHAKE-256f   │
│  Key encapsulation        Kyber-1024/ML-KEM-1024│
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│         OUTSIDE POST-QUANTUM BOUNDARY           │
│                                                 │
│  Block validity proofs    Winterfell STARK    │
│  Tier 3 bridge proofs     Winterfell STARK    │
│                                                 │
│  A QPT adversary running Shor's algorithm       │
│  can forge STARK proofs using quantum computers, but  │
│  production, forging the proof is insufficient  │
│  without also breaking SPHINCS+. For the Tier 3│
│  bridge, a QPT adversary could forge a          │
│  cross-chain intent proof directly.             │
└─────────────────────────────────────────────────┘
```

---

## Consensus

### Validator Model and Fault Assumptions

Let V = {v₁, …, vₙ} be the active validator set at epoch e. Each validator carries a SPHINCS+ verification key, a Kyber-1024 encapsulation key, and a stake in microBLEEP. Total staked supply S = Σsᵢ.

Safety is guaranteed when Byzantine stake f < S/3. Liveness additionally requires eventual message delivery within a known bound Δ. The 3,000 ms slot timer is calibrated against observed testnet propagation latency.

Network model: partial synchrony. Safety holds under full asynchrony; liveness requires partial synchrony. The adversary may control at most f < S/3 of staked supply and may direct those validators arbitrarily, including equivocation and selective silence.

### Block Production Flow

1. **Proposer selection** — at each 3,000 ms slot boundary, a validator is selected with probability proportional to stake fraction.
2. **Block assembly** — `BlockProducer` selects up to 4,096 transactions by fee in descending order and applies them to a draft state copy. Any transaction triggering an invariant violation (overdraft, nonce regression, supply cap breach) is removed and the block recomputed.
3. **State commitment** — the Sparse Merkle Trie root is computed and committed to the block header.
4. **Proof generation** — a Winterfell STARK block validity proof is generated (transparent, no ceremony). The proof attests to: (a) block hash is SHA3-256 of its fields; (b) the proposer possesses the key whose hash equals `validator_pk_hash`; (c) epoch ID is consistent with block index and `blocks_per_epoch`; (d) the SMT root commitment is non-zero. Full transaction execution validity is established by each validator's independent state transition function.
5. **Block signing** — the completed block is signed with SPHINCS+ and broadcast.
6. **Validation** — each receiving validator independently verifies the STARK proof, the SPHINCS+ signature, and the SMT root transition.
7. **Voting** — accepting validators broadcast SPHINCS+-signed prevote, then precommit messages.
8. **Finalisation** — a block is finalised when precommit messages representing more than 6,667 bps of S are received. Finalisation is irreversible.
9. **Epoch transition** — every 1,000 blocks (mainnet) / 100 blocks (testnet), the `epoch_advance` task rotates the validator set, distributes rewards, resets slashing counters, and emits governance events.

### Consensus Modes

`ConsensusOrchestrator` selects the consensus mode deterministically from validator liveness metrics at epoch boundaries — identical computation on every honest node, with no coordinator.

| Mode | Trigger | Description |
|---|---|---|
| `PoS-Normal` | Default | Block production at 3,000 ms intervals with stake-proportional proposer selection |
| `Emergency` | < 67% validator liveness | Reduced quorum mode; halts safely if safety bound cannot be maintained |
| `Recovery` | Post-partition | Re-anchors to the most recent finalised checkpoint after long-range partitions |

### Slashing Parameters

| Violation | Penalty | Implementation |
|---|---|---|
| Double-sign | 33% of stake burned; tombstoned | `double_signing_penalty: 0.33`; `saturating_sub` (SA-M2) |
| Equivocation | 25% of stake burned | `equivocation_penalty: 0.25` |
| Downtime | 0.1% per consecutive missed block | `downtime_penalty_per_block` |
| Tier 4 bridge executor timeout | 30% of executor bond | `EXECUTION_TIMEOUT = 120 s` |

The balance check-and-debit uses a RocksDB compare-and-swap loop with up to three retries, eliminating the time-of-check-to-time-of-use race identified in audit finding SA-H2.

### Finality Guarantee

`LongRangeReorg(10)` and `LongRangeReorg(50)` were each rejected at `FinalityManager` in every iteration across the 72-hour adversarial test run (invariant I-CON3). A finalised block cannot be rolled back without more than S/3 of staked supply being Byzantine — which would trigger slashing that reduces those validators below the minimum stake.

---

## State and Storage

### StateManager

Account state is maintained as a 256-level Sparse Merkle Trie (SMT) backed by RocksDB. Three RocksDB column families serve security-critical functions:

| Column Family | Purpose |
|---|---|
| `audit_log` | Tamper-evident log entries (SHA3-256 Merkle-chained) |
| `audit_meta` | Chain tip and sequence counter for restart recovery; warms the in-memory cache of the most recent 10,000 entries on startup |
| `nullifier_store` | Cross-chain bridge nullifier hashes; `WriteBatch` with `sync=true` (SA-C1: prevents double-spend after node restart) |

```
Key:   b"acct:" + address_utf8
Value: bincode( AccountState { balance: u128, nonce: u64, code_hash: Option<[u8; 32]> } )
```

`advance_block()` is the commit boundary. All writes before it are buffered. A crash before `advance_block()` leaves the previous block's state intact.

### Sparse Merkle Trie

The 256-level SMT provides fixed-size membership and non-membership proofs at **8,192 bytes regardless of account count**. The trie root is committed in every block header.

```
Leaf key   = SHA3-256( address_utf8 )
Leaf value = SHA3-256( abi_encode(address, balance, nonce) )
Interior   = SHA3-256( left_child || right_child )
Empty node = [0u8; 32]
```

```rust
// Node side — generate an inclusion or exclusion proof
let proof = state.prove_account("BLEEP1...");

// Light-client side — verify without a node
assert!(proof.verify(&known_state_root));
```

### State Transition Semantics

Let Sₜ denote the complete protocol state at block index t, and T the canonically ordered sequence of validated transactions in block t. The protocol defines a deterministic total function F such that:

**Sₜ₊₁ = F(Sₜ, T)**

F is total and injective over valid inputs: distinct ordered sequences T ≠ T' applied to the same Sₜ yield distinct Sₜ₊₁, assuming no SHA3-256 collision in the SMT. Combined with per-account nonce ordering, this makes transaction replay detectable and rejectable at the state-transition level.

All components of Sₜ are updated atomically; partial application of F is treated as a failure and rolled back entirely.

### Sharding

State is partitioned across **10 shards** (`NUM_SHARDS`) in the testnet configuration. `ShardManager` routes transactions to shards by account address. Each shard maintains an independent RocksDB instance. `ShardEpochBinding` commits each shard's state root to the epoch Merkle tree at every epoch boundary. `ShardValidatorAssignment` maps validators to shards per epoch using a deterministic function of the epoch randomness beacon — no privileged assignment authority.

Cross-shard transactions use `TwoPhaseCommitCoordinator`. The coordinator shard is derived deterministically from the transaction hash, eliminating coordinator election. Coordinators exceeding a timeout height are force-aborted by the `cross_shard_timeout_sweep` task every 60 seconds.

---

## Execution Environment

The execution environment routes transactions to one of seven engines via `VmRouter`. Each engine is independently gated by circuit breakers and gas budgets. The VM never acquires the `StateManager` lock directly; it returns a `StateDiff` that `BlockProducer` applies under a single lock after all VM calls complete.

| Tier | Engine | Scope | Gas model |
|---|---|---|---|
| 1 | Native | BLEEP Transfer, stake, unstake, governance vote | None |
| 2 | Router | Engine selection, gas validation, circuit breakers | Validation only |
| 3 | EVM (SputnikVM) | Ethereum-compatible contract execution | Ethereum gas semantics |
| 4 | WebAssembly (Wasmi) | WASM contract execution | Configurable fuel metering |
| 5 | ZK Proof | Zero-knowledge execution, public input verification | Fixed cost per verifier op |
| 6 | AI-Advised | Constraint validation before execution (advisory; off-chain) | Deterministic; no gas |
| 7 | Cross-Chain | BLEEP Connect Tier 4 instant intent dispatch | Protocol fee in basis points |

---

## P2P Networking

The P2P network (`bleep-p2p`) uses Kademlia DHT with k=20. Peer IDs are deterministic hashes of SPHINCS+ public keys, binding network identity to post-quantum key material. All inter-node messages are SPHINCS+-signed `SecureMessage` objects; unauthenticated messages are dropped before payload processing.

A **2 MiB size gate** is enforced at the receive boundary before any deserialisation (audit finding SA-H3), preventing memory exhaustion attacks.

Onion routing provides multi-hop anonymised delivery using AES-256-GCM keyed from Kyber-1024 per-hop shared secrets (maximum 6 hops).

`PeerScoring` computes a composite trust score in [0.0, 100.0] from success ratio, message rate, latency, and diversity components. Scores decay at 0.99× per 300 seconds. Peers below 40 are excluded from gossip relay; peers below 55 from onion routing relay.

| Component | Description |
|---|---|
| `KademliaDHT` | 256 K-buckets, XOR metric, k=20 replication factor |
| `GossipProtocol` | Epidemic dissemination (fanout 8); O(log n) rounds to reach n nodes |
| `OnionRouter` | 6-hop max; Kyber-1024 KEM per hop; AES-256-GCM payload encryption |
| `PeerScoring` | Composite trust score; decay-based Sybil resistance |
| `MessageProtocol` | TCP framing; SPHINCS+ message authentication; anti-replay nonce cache |

---

## Governance

`LiveGovernanceEngine` processes typed proposals through a six-stage lifecycle:

**Submit → AIConstraintValidator pre-flight → Active → Tally → Execute → Record**

The `AIConstraintValidator` pre-flight checks proposals against the four constitutional invariants before they enter the vote queue. A proposal that would set `MaxInflationBps` above 500 is rejected at this stage and never reaches a vote. The `constitutional_violation_rejected_at_submission` test verifies this on every CI build.

### Governance Parameters (Testnet)

| Parameter | Value |
|---|---|
| Voting period | 1,000 blocks (~50 min at 3-second block time) |
| Quorum threshold | 1,000 bps (10% minimum stake participation) |
| Pass threshold | 6,667 bps (66.67% of participating stake) |
| Veto threshold | 3,333 bps (33.33% veto blocks passage) |
| Minimum deposit | 10,000 BLEEP |

### Zero-Knowledge Voting

`ZKVotingEngine` provides privacy-preserving stake-weighted voting. Voter weight multipliers:

| Role | Multiplier |
|---|---|
| Validator | 1.0× (10,000 bps per unit stake) |
| Delegator | 0.5× (5,000 bps) |
| Community token holder | 0.1× (1,000 bps) |

`VoteCommitment`-based double-vote prevention and nonce-based replay resistance are enforced at the voting engine. `EligibilityProof` establishes voting power without revealing voter identity. `TallyProof` allows independent tally verification without learning individual votes.

### Forkless Protocol Upgrades

`ForklessUpgradeEngine` manages hash-committed protocol upgrades that activate at epoch boundaries only. `Version.is_valid_upgrade()` enforces monotonic version progression; a version mismatch halts the chain. Partial upgrades are rejected atomically — either the entire payload activates or nothing does. Every upgrade is hash-committed before the governance vote, so validators know exactly what code will activate before they vote.

### Live Governance Record

`proposal-testnet-001` completed the full lifecycle on `bleep-testnet-1`: `AIConstraintValidator` pre-flight, ZK vote casting by seven validators, quorum check at 70% stake participation, constitutional validation, on-chain execution at block 1,105, and event recording. It reduced `FeeBurnBps` from 2,500 to 2,000.

---

## Economics and Tokenomics

### Constitutional Token Parameters

| Parameter | Value | Source |
|---|---|---|
| Maximum supply (†) | 200,000,000 BLEEP | `MAX_SUPPLY` in `tokenomics.rs` |
| Token decimals | 8 (1 BLEEP = 10⁸ microBLEEP) | `tokenomics.rs` |
| Initial circulating supply | 25,000,000 BLEEP (12.5%) | `INITIAL_CIRCULATING_SUPPLY` |
| Maximum per-epoch inflation (†) | 500 bps (5%) | `MAX_INFLATION_RATE_BPS` |
| Fee burn split (†) | 2,500 bps (25%) | `FEE_BURN_BPS` in `distribution.rs` |
| Validator fee split | 5,000 bps (50%) | `FEE_VALIDATOR_REWARD_BPS` |
| Treasury fee split | 2,500 bps (25%) | `FEE_TREASURY_BPS` |
| Split integrity | Burn + Validator + Treasury = 10,000 bps | Compile-time `const` assertion in `distribution.rs` |
| Minimum base fee | 1,000 microBLEEP | `MIN_BASE_FEE` |
| Maximum base fee | 10,000,000,000 microBLEEP | `MAX_BASE_FEE` |
| Max base fee change per block | 1,250 bps (12.5%) | `max_increase_bps` in `BaseFeeParams` |

*(†) = constitutional; enforced by compile-time `const` assertion*

### Fee Market

The base fee adjusts per block against a 50% block capacity target following an EIP-1559-style mechanism. `FeeDistribution::compute()` splits each collected fee 25/50/25 across burn, validator rewards, and treasury. At sustained throughput above 10,000 TPS, the annual burn rate exceeds Year 5+ validator emission (2,400,000 BLEEP per year), creating net deflationary pressure.

Audit finding SA-M4 (acknowledged, Medium severity) notes that an adversarial block proposer with sufficient consecutive proposer slots can pin the base fee near its maximum. Proof-of-stake proposer rotation limits the duration. This is documented in `THREAT_MODEL.md`.

### Validator Emission Schedule

| Year | Rate | Annual emission (BLEEP) |
|---|---|---|
| 1 | 12% | 7,200,000 |
| 2 | 10% | 6,000,000 |
| 3 | 8% | 4,800,000 |
| 4 | 6% | 3,600,000 |
| 5+ | 4% | 2,400,000 per year |

The emission schedule is encoded as `VALIDATOR_EMISSION_YEAR` in `tokenomics.rs`. It is not a governance parameter and cannot be changed without a software upgrade.

### Token Distribution

| Allocation | Tokens | % | Launch unlock | Vesting |
|---|---|---|---|---|
| Validator Rewards | 60,000,000 | 30% | 10,000,000 | Emission decay schedule |
| Ecosystem Fund | 50,000,000 | 25% | 5,000,000 | 10-year linear; governance-controlled disbursement |
| Community Incentives | 30,000,000 | 15% | 5,000,000 | Governance-triggered release |
| Foundation Treasury | 30,000,000 | 15% | 5,000,000 | 6-year linear; governance-controlled spending |
| Core Contributors | 20,000,000 | 10% | 0 | 1-year cliff + 4-year linear; immutable on-chain contract |
| Strategic Reserve | 10,000,000 | 5% | 0 | Governance-controlled unlock; proposal + vote required |

A compile-time `const` assertion in `distribution.rs` verifies the sum equals `MAX_SUPPLY` exactly.

### Game-Theoretic Safety Verifier

`SafetyVerifier` in `bleep-economics/src/game_theory.rs` formally evaluates five attack models against current protocol parameters, returning `attacker_profit`, `network_cost`, and `is_profitable` for each: Equivocation, Censorship, NonParticipation, Griefing, and Cartel formation.

`SafetyVerifier` runs in CI. **A build fails if any model returns `is_profitable = true`.** This provides a machine-verified economic safety property analogous to the compile-time constitutional invariants.

---

## BLEEP Connect

BLEEP Connect is a four-tier cross-chain bridge architecture implemented across ten sub-crates within `bleep-interop`. No tier requires a permanently privileged operator or a trusted multisig key set.

### Bridge Tier Overview

| Tier | Protocol | Latency | Security basis | Status |
|---|---|---|---|---|
| 4 — Instant | Executor auction + escrow | 200 ms – 1 s | Economic: 30% executor bond slashed on timeout | Live — Ethereum Sepolia |
| 3 — ZK Proof | STARK batch proof | 10 – 30 s | Cryptographic: Winterfell STARK (transparent, post-quantum) | Live — Ethereum Sepolia |
| 2 — Full-Node | Multi-client verification | Hours | 90% consensus across ≥ 3 independent nodes; optional TEE | Implemented; mainnet target |
| 1 — Social | Stakeholder governance | 7 days / 24 h (emergency) | Full governance consensus | Implemented; mainnet target |

### Tier 4 — Instant Relay

An `InstantIntent` submitted via `POST /rpc/connect/intent` enters a 15-second executor auction (`EXECUTOR_AUCTION_DURATION`). The winning executor commits to fulfilling the intent within 120 seconds (`EXECUTION_TIMEOUT`). A 30% executor bond is slashed on timeout. Protocol fee: 10 bps (`PROTOCOL_FEE_BPS`).

The security model is economic, not cryptographic. Transfers whose value approaches or exceeds the executor bond require Tier 3 or higher.

### Tier 3 — ZK Proof Bridge

Batches up to 32 cross-chain intents (`L3_BATCH_SIZE`) into a single Winterfell STARK proof. Submitted to the `BleepL3Bridge` contract on Ethereum Sepolia; verified in approximately 250,000 gas (`L3_VERIFICATION_GAS`). Transparent, post-quantum secure — no ceremony participants to compromise.

**As stated in the post-quantum boundary section, Tier 3 is not post-quantum secure.** A QPT adversary could forge a cross-chain intent proof without breaking any other component. Users requiring post-quantum-secure cross-chain transfers must use Tier 2 or await migration of the bridge to a post-quantum-secure proof system.

Double-spend prevention: `GlobalNullifierSet` performs an atomic `WriteBatch` with `sync=true` on first submission and returns `Err(NullifierAlreadySpent)` on any duplicate. Audit finding SA-C1 identified that the original implementation used an in-memory `HashSet` lost on node restart; this is resolved in Protocol Version 3.

### Tier 2 — Full-Node Verification

Requires 90% consensus (`CONSENSUS_THRESHOLD = 0.90`) across at least 3 independent verifier nodes (`MIN_VERIFIER_NODES = 3`) running different blockchain client implementations. Nodes independently query the actual on-chain state root for the claimed block number. Optional Trusted Execution Environment attestations (`TEEType::IntelSGX`) provide additional integrity guarantees.

Tier 2 avoids pairing-based cryptography entirely by requiring cryptographic verification only on SPHINCS+ signatures and hash preimages.

### Tier 1 — Social Consensus

Handles scenarios where cryptographic or economic guarantees are insufficient: chain reorganisations, detected quantum attacks, smart contract bugs at bridge scale, and protocol upgrades. Standard proposals use a 7-day voting window (`VOTING_PERIOD_NORMAL`) with 66% approval. Emergency proposals use a 24-hour window (`VOTING_PERIOD_EMERGENCY`) with an 80% threshold.

---

## AI Advisory Components

Two AI-assisted components exist in the Protocol Version 3 codebase. **Neither participates in block production, consensus voting, or any state-modifying operation without a prior governance vote.** AI outputs are inputs to the governance process, not outputs of it.

### Phase 3 — AIConstraintValidator (Operational)

A deterministic rule engine — not a learned model — that checks governance proposals against the four constitutional invariants before they enter the vote queue. A proposal that would set `MaxInflationBps` above 500 is rejected at this stage with a descriptive error. The `constitutional_violation_rejected_at_submission` test verifies this on every CI build.

`AIConsensusOrchestrator` coordinates six typed healing proposal types with the consensus layer. Each proposal is advisory; the consensus layer retains authority to accept or reject.

### Phase 4 — DeterministicInferenceEngine (Under Development)

An ONNX-based inference runtime that enforces six invariants before any model output is used on a consensus-critical path. These invariants exist to satisfy the determinism requirement of Section 4.2 — they are not optional quality controls.

| Invariant | Description |
|---|---|
| Model hash verification | SHA3-256 of the model binary must match `ModelMetadata.model_hash` before any inference runs |
| Deterministic input normalisation | Fixed mean, standard deviation, and clamp parameters applied to all inputs |
| Deterministic output rounding | Configurable decimal precision applied to all outputs; no floating-point variance propagates |
| CPU-only execution | No GPU or platform-specific computation paths; ONNX Runtime configured to deterministic CPU backend |
| Governance-approval gating | `approval_epoch` must be set by a governance vote before a model runs on any consensus-critical path |
| No dynamic model loading | Model versions are immutable once deployed; replacement requires a governance proposal and vote |

Every inference produces an `InferenceRecord` containing the model hash, normalised inputs, raw outputs, and a deterministic seed for reproducibility verification.

**No trained model is currently deployed in any governance-critical or consensus-critical path.** The claims about `DeterministicInferenceEngine` describe the design of the Phase 4 system.

### AI Attestation

`AIAttestationManager` records every AI output as an `AIAttestationRecord` containing an `AIOutputCommitment` computed as `SHA3-256(model_hash || inputs_hash || output_hash || epoch)`. Attestations are queryable by epoch at `GET /rpc/ai/attestations/{epoch}`.

---

## Protocol Parameters

All values are drawn from the production Rust source at Protocol Version 3. Parameters marked (†) are constitutional and cannot be changed by governance vote or software upgrade.

### Consensus and Execution

| Parameter | Value | Source |
|---|---|---|
| Block interval | 3,000 ms | `BLOCK_INTERVAL_MS` |
| Max transactions per block | 4,096 | `MAX_TXS_PER_BLOCK` |
| Blocks per epoch (mainnet) | 1,000 | `BLOCKS_PER_EPOCH` |
| Blocks per epoch (testnet) | 100 | `testnet-genesis.toml` |
| Finality threshold (†) | > 6,667 bps of total stake | `FinalityManager` |
| Active shards | 10 | `NUM_SHARDS` |
| Double-sign slash | 33% of stake | `double_signing_penalty` |
| Equivocation slash | 25% of stake | `equivocation_penalty` |
| Downtime slash | 0.1% per missed block | `downtime_penalty_per_block` |

### Cryptography and Networking

| Parameter | Value | Source |
|---|---|---|
| SPHINCS+ signature size | 7,856 bytes | `SPHINCS_SIG_LEN` |
| SPHINCS+ public key size | 32 bytes | `pqcrypto-sphincsplus` |
| Kyber-1024 public key size | 1,568 bytes | `pqcrypto-kyber` |
| State trie depth | 256 levels | `TRIE_DEPTH` |
| Merkle proof size | 8,192 bytes | `SparseMerkleTrie` |
| Gossip max message size | 2,097,152 bytes (2 MiB) | `MAX_GOSSIP_MSG_BYTES` |
| Gossip fanout | 8 | `bleep-p2p` |
| Kademlia k-bucket size | 20 | `bleep-p2p` |
| Onion routing max hops | 6 | `MAX_HOPS` |
| MPC ceremony participants | 5 (minimum 3) | `MIN_PARTICIPANTS` |
| JWT entropy minimum | 3.5 bits/byte (Shannon) | `session.rs` |

### Cross-Chain Bridge

| Parameter | Value | Source |
|---|---|---|
| Tier 3 proof size | 192 bytes | `L3_PROOF_SIZE_BYTES` |
| Tier 3 batch size | 32 intents | `L3_BATCH_SIZE` |
| Tier 3 verification gas (Sepolia) | 250,000 | `L3_VERIFICATION_GAS` |
| Tier 2 consensus threshold | 90% | `CONSENSUS_THRESHOLD` |
| Tier 2 minimum verifiers | 3 | `MIN_VERIFIER_NODES` |
| Tier 4 execution timeout | 120 s | `EXECUTION_TIMEOUT` |
| Tier 4 protocol fee | 10 bps (0.1%) | `PROTOCOL_FEE_BPS` |

---

## Getting Started

### Prerequisites

```bash
# Rust stable toolchain, edition 2021
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Ubuntu / Debian
sudo apt-get install -y build-essential clang libclang-dev librocksdb-dev

# macOS (Homebrew)
brew install rocksdb llvm
export LIBRARY_PATH="$(brew --prefix rocksdb)/lib:$LIBRARY_PATH"
```

### Build

```bash
git clone https://github.com/bleep-project/bleep.git
cd bleep

# Full workspace — all 19 crates
cargo build --release --workspace

# Node binary only
cargo build --release --bin bleep

# CLI only
cargo build --release --bin bleep-cli
```

### Run a Local Single-Validator Node

```bash
./target/release/bleep
# Node completes 16-step startup sequence
# RPC ready at :8545, P2P at :7700

# Verify the node is live
curl -s http://127.0.0.1:8545/rpc/health | jq .
```

### Create a Wallet and Send a Transaction

```bash
# 1. Generate SPHINCS+ + Kyber-1024 keypair; prompts for encryption passphrase
./target/release/bleep-cli wallet create

# 2. Check balance (live from node via GET /rpc/state)
./target/release/bleep-cli wallet balance

# 3. Import from BIP-39 mnemonic (PBKDF2-HMAC-SHA512, 2,048 rounds)
./target/release/bleep-cli wallet import \
  "abandon abandon abandon abandon abandon abandon \
   abandon abandon abandon abandon abandon about"

# 4. Send a transfer (prompts for passphrase to unlock SPHINCS+ signing key)
./target/release/bleep-cli tx send \
  --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 \
  --amount 1000
```

### Run the Test Suite

```bash
# Full workspace
cargo test --workspace

# Security-critical crates
cargo test -p bleep-crypto
cargo test -p bleep-state
cargo test -p bleep-consensus
cargo test -p bleep-wallet-core

# With output
RUST_LOG=debug cargo test -p bleep-state -- --nocapture

# Fuzz targets (requires cargo-fuzz)
cargo fuzz run hash_determinism
cargo fuzz run sign_verify_roundtrip
cargo fuzz run kyber_encap_decap
cargo fuzz run merkle_insertion_soundness
cargo fuzz run state_transition_fund_conservation
```

---

## Configuration

```toml
# bleep.toml (all values shown are defaults)

[node]
p2p_port  = 7700
rpc_port  = 8545
state_dir = "/var/lib/bleep/state"
log_level = "info"

[consensus]
block_interval_ms = 3000
max_txs_per_block = 4096
blocks_per_epoch  = 1000   # use 100 for testnet
validator_id      = "validator-0"

[features]
quantum = true   # must remain true for post-quantum security
```

### Cargo Feature Flags

| Flag | Default | Effect |
|---|---|---|
| `mainnet` | on | Mainnet protocol constants |
| `testnet` | off | Testnet constants (reduced epoch length, etc.) |
| `quantum` | on | Enables `pqcrypto` and `pqcrypto-kyber`; required for SPHINCS+ and Kyber-1024 |

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `BLEEP_RPC` | `http://127.0.0.1:8545` | RPC endpoint for all CLI commands |
| `BLEEP_STATE_DIR` | `/tmp/bleep-state` | Local RocksDB path (offline fallback for balance queries) |
| `RUST_LOG` | `info` | `tracing` log filter |

---

## RPC API Reference

All responses are JSON. Errors carry an `"error"` string field. The server listens on port **8545**.

### Core Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/health` | Status, chain height, peer count, uptime, version |
| `GET` | `/rpc/telemetry` | Blocks produced, transactions processed, uptime |
| `GET` | `/rpc/block/latest` | Latest block height and hash |
| `GET` | `/rpc/block/{id}` | Block by height or hash |
| `POST` | `/rpc/tx/submit` | Submit a signed transaction |
| `GET` | `/rpc/state/{address}` | Live balance, nonce, state root, block height |
| `GET` | `/rpc/proof/{address}` | 256-level SMT inclusion/exclusion proof (8,192 bytes) |

### Validator Endpoints

| Method | Path | Description |
|---|---|---|
| `POST` | `/rpc/validator/stake` | Register validator or increase stake |
| `POST` | `/rpc/validator/unstake` | Initiate graceful validator exit |
| `GET` | `/rpc/validator/list` | All active validators with stake |
| `GET` | `/rpc/validator/status/{id}` | Validator status and slashing history |
| `POST` | `/rpc/validator/evidence` | Submit double-sign evidence (auto-executes slashing) |

### Economics and Oracle Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/economics/supply` | Circulating supply, minted, burned, base fee |
| `GET` | `/rpc/economics/fee` | Current EIP-1559 base fee + last epoch |
| `GET` | `/rpc/economics/epoch/{n}` | Full epoch output (emissions, burns, price) |
| `GET` | `/rpc/oracle/price/{asset}` | Aggregated oracle price (median, sources) |
| `POST` | `/rpc/oracle/update` | Submit oracle price update |

### BLEEP Connect Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/connect/intents/pending` | Pending Tier 4 instant intents |
| `POST` | `/rpc/connect/intent` | Submit a new instant intent |
| `GET` | `/rpc/connect/intent/{id}` | Query intent status |

### AI Attestation Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/ai/attestations/{epoch}` | All `AIAttestationRecord` entries for a given epoch |

### Example Responses

```bash
# GET /rpc/health
{
  "status":      "ok",
  "height":      1024,
  "peers":       8,
  "uptime_secs": 3600,
  "version":     "3.0.0"
}

# GET /rpc/state/{address}
{
  "address":      "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "balance":      "10000000000",
  "nonce":        4,
  "state_root":   "8a3f2c1d9e7b4a0f5c6d7e8f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
  "block_height": 1024
}

# GET /rpc/proof/{address}
{
  "address":  "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "exists":   true,
  "leaf":     "3f2a1b0c...",
  "root":     "8a3f2c1d...",
  "siblings": ["00000000...", "4a2b1c3d...", "..."],
  "is_right": [false, true, false]
}
```

```rust
// Verify a proof offline — no node required
let proof: MerkleProof = serde_json::from_str(&json_body)?;
assert!(proof.verify(&known_state_root));
```

---

## CLI Reference

```
bleep-cli [OPTIONS] <COMMAND>

Environment variables:
  BLEEP_RPC        RPC endpoint     default: http://127.0.0.1:8545
  BLEEP_STATE_DIR  Local DB path    default: /tmp/bleep-state
  RUST_LOG         Log filter       default: info

Commands:
  start-node                           Start a full BLEEP node (all subsystems)

  wallet create                        Generate SPHINCS+ + Kyber-1024 keypair; encrypt SK with AES-256-GCM
  wallet balance                       Query balance from /rpc/state; falls back to local RocksDB
  wallet import <phrase>               Import from BIP-39 mnemonic (PBKDF2-HMAC-SHA512, 2,048 rounds)
  wallet export                        Print wallet addresses

  tx send --to <addr> --amount <n>     Sign with SPHINCS+; POST /rpc/tx/submit
  tx history                           GET /rpc/tx/history

  validator stake --amount <n>         Register as validator or increase stake
  validator unstake                    Initiate graceful exit
  validator list                       List all active validators
  validator status <id>                Validator status + slashing history
  validator submit-evidence <file>     Submit double-sign evidence

  governance propose <text>            Create a governance proposal (requires min deposit)
  governance vote <id> --yes/--no      Cast a stake-weighted ZK vote
  governance list                      List all proposals and their lifecycle stage
  governance status <id>               Detailed proposal status

  block latest                         Print the latest block
  block get <id>                       Get a block by hash or height
  block validate <hash>                Validate a block hash via node

  state snapshot                       Create a RocksDB state snapshot
  state restore <path>                 Restore from snapshot

  zkp <proof>                          Verify a STARK proof (transparent, post-quantum)
  ai ask <prompt>                      Query the AI advisory engine (advisory only)
  ai status                            AI engine status and approved model hashes
  ai attestations <epoch>              List AI attestation records for an epoch

  pat status                           PAT engine status
  pat list                             List registered asset tokens
  pat mint --to <addr> --amount <n>    Mint PAT tokens (owner only)
  pat burn --amount <n>                Burn PAT tokens
  pat transfer --to <addr> --amount <n> Transfer PAT tokens
  pat balance <address>                Query PAT balance

  oracle price <asset>                 Query aggregated oracle price
  oracle submit --asset ... --price <n> Submit oracle price update

  economics supply                     Circulating supply, minted, burned
  economics fee                        Current EIP-1559 base fee
  economics epoch <n>                  Epoch emissions, burns, price

  telemetry                            Print live node metrics
  info                                 Node version and RPC health
```

### Executor Node

The `bleep-executor` binary participates in the Tier 4 instant intent market:

```bash
# Production usage
BLEEP_EXECUTOR_KEY=<32-byte-hex-seed>             \
BLEEP_EXECUTOR_CAPITAL_BLEEP=100000000000         \
BLEEP_EXECUTOR_CAPITAL_ETH=10000000000000000000   \
BLEEP_EXECUTOR_RISK=Medium                        \
BLEEP_RPC=http://your-node:8545                   \
./target/release/bleep-executor
```

---

## Security Model

### Threat Model

BLEEP's security analysis considers three adversary classes:

**Classical PPT adversary** — targets 256-bit security on all operations. All post-quantum primitives and STARK proofs provide ≥ 256-bit classical security.

**Quantum QPT adversary** — equipped with Shor's algorithm. SPHINCS+-SHAKE-256f-simple, Kyber-1024/ML-KEM-1024, and Winterfell STARK proofs all maintain post-quantum security. No vulnerable elliptic-curve or pairing-based primitives remain.

**Byzantine validator adversary** — controls f < S/3 of staked supply and may direct those validators to behave arbitrarily, including equivocation and selective silence. The BFT safety guarantee holds unconditionally under this model.

### Independent Security Audit

An independent security audit of Protocol Version 3 reviewed 16,127 lines of Rust across six crates.

| Severity | Count | Resolved | Acknowledged |
|---|---|---|---|
| Critical | 2 | 2 | 0 |
| High | 3 | 3 | 0 |
| Medium | 4 | 3 | 1 (SA-M4: EIP-1559 design property; documented in `THREAT_MODEL.md`) |
| Low | 3 | 3 | 0 |
| Informational | 2 | 1 | 1 (SA-I2: NTP drift guard is a mainnet gate) |

Full audit report: `docs/SECURITY_AUDIT_SPRINT9.md`

### Adversarial Test Suite Results (72-hour continuous run, 7 validators)

| Scenario | Result | Invariant verified |
|---|---|---|
| `ValidatorCrash(1)` | Pass | f=1 < 2.33; consensus resumed |
| `ValidatorCrash(2)` | Pass | f=2 < 2.33; consensus resumed |
| `NetworkPartition(4/3)` | Pass | Majority partition continued; healed cleanly |
| `NetworkPartition(5/2)` | Pass | Majority partition continued; healed cleanly |
| `LongRangeReorg(10)` | Pass | Rejected at `FinalityManager` (invariant I-CON3) |
| `LongRangeReorg(50)` | Pass | Rejected at `FinalityManager` (invariant I-CON3) |
| `DoubleSign(validator-0)` | Pass | 33% slashed; evidence committed; tombstoned |
| `DoubleSign(validator-3)` | Pass | 33% slashed; evidence committed; tombstoned |
| `TxReplay` | Pass | Rejected by nonce check (invariant I-S5) |
| `EclipseAttack(validator-6)` | Pass | Mitigated by Kademlia k=20 and DNS seed nodes |
| `InvalidBlockFlood(1000)` | Pass | Rejected at SPHINCS+ gate; peer rate-limited |
| `LoadStress(1,000 TPS, 60s)` | Pass | 4,096 tx/block sustained; no dropped transactions |
| `LoadStress(5,000 TPS, 60s)` | Pass | 4,096 tx/block sustained; no dropped transactions |
| `LoadStress(10,000 TPS, 60s)` | Pass | Block capacity saturated; max throughput reached |

Note: `ValidatorCrash(3)` correctly halts consensus (f=3 ≥ 2.33, violating BFT bound) and is not listed above — it validates that the safety bound is correctly enforced, not that the system recovers.

### Key Audit Finding Resolutions

| Finding | Severity | Resolution |
|---|---|---|
| SA-C1: NullifierStore used in-memory HashSet, lost on restart; double-spend possible | Critical | `NullifierStore` now uses RocksDB `WriteBatch` with `sync=true`; persists across restarts |
| SA-C2: JWT rotation accepted low-entropy secrets | Critical | Shannon entropy gate (≥ 3.5 bits/byte) enforced on all JWT rotation |
| SA-H2: Balance check-and-debit had TOCTOU race | High | Replaced with RocksDB compare-and-swap loop (up to 3 retries) |
| SA-H3: No message size limit before deserialisation | High | 2 MiB gate enforced at receive boundary before any deserialisation |
| SA-M1: STARK ceremony | Previous | STARKs are transparent and require no ceremony. Eliminated. |
| SA-M2: Slash arithmetic could underflow | Medium | All slash arithmetic uses `saturating_sub` |
| SA-L3: Secret keys persisted in memory after drop | Low | All secret key types wrapped in `Zeroizing<Vec<u8>>`; zeroed before deallocation |
| SA-I2: NTP drift guard not enforced at startup | Informational | Implemented; activated as mainnet gate (warn >1 s, halt >30 s) |

---

## Known Limitations

### 1. The ZK Proof Subsystem Is Not Post-Quantum Secure

Winterfell STARK proofs are transparent (no ceremony required) and post-quantum secure (security reduces to hash collision resistance). All zero-knowledge proof paths are now unconditionally post-quantum secure.

### 2. Post-Quantum Primitives Introduce Measurable Overhead

SPHINCS+-SHAKE-256f-simple produces 7,856-byte signatures, compared to 64 bytes for ECDSA or 96 bytes for BLS. On a 4,096-transaction block, aggregate signature data is approximately 32 MB. At the 3,000 ms slot interval, this imposes a minimum bandwidth requirement of approximately **87 MB/s from signatures alone** — before transaction payloads or vote messages. Signature aggregation for SPHINCS+ remains an open research problem (see [Future Work](#future-work)).

Kyber-1024 public keys are 1,568 bytes compared to 32-byte Curve25519 keys, increasing per-session handshake overhead and validator registry storage costs.

These overheads are the direct, quantified cost of the post-quantum security guarantee. They are inherent properties of current post-quantum constructions, not implementation deficiencies.

### 3. Trusted Setup Requirement

STARKs require no trusted setup ceremony. Proofs are transparent and do not rely on any ceremony.

### 4. Shard Count and Validator Assignment

Increasing the shard count above 10 increases throughput but reduces per-shard validator assignment, weakening per-shard BFT safety. The minimum per-shard validator count to maintain a reasonable safety margin must be validated against the mainnet validator set size before the shard count is increased.

### 5. AI Components Are Pre-Production

`DeterministicInferenceEngine` (Phase 4) is under active development. No trained ONNX model is currently deployed in any governance-critical or consensus-critical path. The determinism invariants described for Phase 4 cannot be empirically confirmed until a trained model is deployed and its behaviour verified in production.

### 6. Tier 2 and Tier 1 Bridge Tiers Are Not Yet Live

Tier 2 full-node cross-chain verification and Tier 1 social-consensus bridge are implemented and tested against mock verifier sets. They have not been deployed in a live multi-party environment. Tier 3 (Winterfell STARK) and Tier 4 (executor auction) are live on Ethereum Sepolia.

### 7. NTP Clock Drift Guard

The NTP drift guard (warn >1 s, halt >30 s) is implemented but not enforced at startup in the testnet configuration. It is a documented mainnet gate (SA-I2).

---

## Future Work

### Post-Quantum Zero-Knowledge Proofs

All pathways to post-quantum-secure zero-knowledge proofs are now live. Winterfell STARK proofs (transparent, hash-based) replaced previous pairing-based constructions. The primary remaining research direction is optimizing proof generation and verification times.
- **Lattice-based SNARKs** — active research area with improving efficiency; not yet standardised.
- **Hash-based systems** (Ligero, Brakedown) — transparent, post-quantum secure; higher prover time.

Migration requires new circuit implementations, a new ceremony or transparent setup, and a governance-controlled protocol upgrade. This is a research-grade engineering effort spanning multiple development cycles.

### SPHINCS+ Signature Aggregation

SPHINCS+ does not support aggregation: n validators produce n independent 7,856-byte signatures. At large validator counts, aggregate vote message size becomes a bandwidth bottleneck. Hash-based signature aggregation combining Merkle-based multi-signatures with the SPHINCS+ construction is a medium-term research direction. No standardised solution exists at the time of writing.

### ONNX Inference Pipeline

Phase 4 completes the `DeterministicInferenceEngine` training pipeline, model governance approval flow, and `AIConstraintValidator v2` with a trained classification model. The research question is whether a trained model can reliably identify governance proposals that are economically harmful in ways not captured by the rule-based Phase 3 validator.

### Public Testnet Expansion

Phase 4 targets at least 50 validators across at least 6 continents, with open registration, a public block explorer, a 30-day sustained run, and a 100,000 BLEEP bug bounty pool. This milestone is required to validate the BFT safety bound and validator assignment algorithm at realistic network sizes.

---

## Roadmap

### Phase 1 — Foundation ✅ Complete

All 19 crates compile cleanly. SPHINCS+-SHAKE-256f-simple and Kyber-1024 active. RocksDB `StateManager` with `SparseMerkleTrie`. Full `BlockProducer` loop. Winterfell STARK ZK circuits (transparent, no ceremony). 4-node docker-compose devnet. BLEEP Connect Tier 4 live on Ethereum Sepolia. `BleepEconomicsRuntime` (EIP-1559 fee market, oracle bridge, validator incentives). `PATRegistry` live. `bleep-executor` standalone intent market maker.

### Phase 2 — Testnet Alpha ✅ Complete

7-validator `bleep-testnet-1` genesis. Public DNS seeds. Public faucet (1,000 BLEEP per 24 hours). Block explorer (6-second refresh). JWT rotation, NDJSON audit export. Grafana dashboard (12 panels) + Prometheus. Full CI pipeline: fmt, clippy, test, audit, build, fuzz-smoke, docker-smoke.

### Phase 3 — Protocol Hardening ✅ Complete

- ✅ Independent security audit — 14 findings, all Critical and High resolved
- ✅ 72-hour adversarial test suite — 14 scenarios, no unresolved failures
- ✅ Winterfell STARK proofs — transparent, post-quantum secure, no ceremony
- ✅ Cross-shard stress test — 10 shards, 1,000 concurrent cross-shard txs, 100 epochs
- ✅ BLEEP Connect Tier 3 — Winterfell STARK batch proof bridge live on testnet
- ✅ Live governance — `LiveGovernanceEngine` with typed proposals, ZK voting, on-chain execution
- ✅ Performance benchmark — avg **10,921 TPS**, peak **13,200 TPS**, 1-hour sustained, 10 shards
- ✅ Token distribution model — 6 allocation buckets, compile-time verified constants

### Phase 4 — AI Model Training ⏳ Active

Upgrade `bleep-ai` from rule-based advisory to a trained on-chain inference engine. `DeterministicInferenceEngine` ONNX pipeline. `AIConstraintValidator v2` with trained classification model. AI validator nodes with deterministic inference on governance-critical paths. **Definition of done:** AI advisory engine passes determinism test suite; governance pre-flight achieves ≥ 95% accuracy on labelled test set.

### Phase 5 — Public Testnet Expansion ⏳ Upcoming

Open validator onboarding. Target: ≥ 50 validators across ≥ 6 continents. 30-day sustained run. Cross-shard expansion: 10 → 20 shards. Community bug bounty: up to 100,000 BLEEP. **Definition of done:** 50+ active validators; 30 consecutive days without manual intervention.

### Phase 6 — Pre-Sale / ICO ⏳ Upcoming

Community token sale in two tranches. `LinearVestingSchedule` contracts deployed. KYC/AML compliant infrastructure. Multi-sig treasury custody. **Definition of done:** ICO completed; all vesting contracts deployed and verified on-chain.

### Phase 7 — Mainnet Launch 🔜 Planned

Mainnet requires: ≥ 21 validators with geographic diversity; governance active from block 1; BLEEP Connect Tier 1 through Tier 4 operational on Ethereum and Solana from genesis; client SDKs; NTP drift guard active (SA-I2); `GenesisAllocation` vesting contracts deployed.

**Definition of done:** Genesis block produced by ≥ 21 independent validators; governance proposal passes on-chain within first week; cross-chain Ethereum transfer confirms within 1 second.

---

## Contributing

1. Fork the repository and create a feature branch: `git checkout -b feat/your-feature`
2. Run the full test suite: `cargo test --workspace`
3. Run the linter: `cargo clippy --workspace -- -D warnings`
4. Verify formatting: `cargo fmt --all -- --check`
5. Open a pull request against `main` with a clear description of the change and the crates affected.

Changes to `bleep-consensus`, `bleep-crypto`, or `bleep-state` undergo extended review given their security surface area. Changes to constitutional constants (`MAX_SUPPLY`, `MAX_INFLATION_RATE_BPS`, `FEE_BURN_BPS`, and the finality threshold) will not be accepted without a formal governance proposal and protocol amendment process.

### Security Disclosures

Do not open public issues for security vulnerabilities. Email `security@bleep.network` with a description, reproduction steps, and your proposed fix. We target 72-hour acknowledgment and a 14-day patch timeline for Critical and High findings.

---

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

---

*BLEEP · Quantum Trust Network · Protocol Version 3 · Chain ID: `bleep-testnet-1`*

*This document corresponds to Protocol Version 3 and may change before mainnet deployment. It does not constitute financial advice, investment advice, or an offer to sell securities or digital assets.*│  │   bleep-core    │          │    bleep-vm          │                       │
│  │  Block · Tx     │          │  7-layer intent VM   │                       │
│  │  Blockchain     │          │  EVM (revm 3.5)      │                       │
│  │  BlockValidator │          │  WASM (wasmer 4.2)   │                       │
│  │  Mempool · Pool │          │  ZK · StateDiff      │                       │
│  └────────┬────────┘          └──────────┬───────────┘                       │
│           │                              │                                   │
│  ┌────────▼──────────────────────────────▼───────────────────────────────┐  │
│  │                          bleep-state                                  │  │
│  │  StateManager (RocksDB + LZ4)  ·  SparseMerkleTrie (256-bit paths)   │  │
│  │  Sharding · Cross-shard 2PC · Self-healing · Snapshot / Rollback      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌──────────────────┐  ┌────────────────┐  ┌────────────────────────────┐  │
│  │   bleep-p2p      │  │  bleep-crypto  │  │      bleep-interop         │  │
│  │  libp2p 0.53     │  │  SPHINCS+      │  │  BLEEP Connect (4 tiers)   │  │
│  │  Kademlia k=20   │  │  Kyber-768     │  │  10 sub-crates             │  │
│  │  Gossip (Plumtree│  │  BIP-39        │  │  ETH · SOL adapters        │  │
│  │  Onion routing   │  │  AES-256-GCM   │  │  Winterfell STARK bridge  │  │
│  └──────────────────┘  └────────────────┘  └────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Block production data flow

```
MempoolBridge (500 ms drain)
  │
TransactionPool  (FIFO, per-sender nonce ordering)
  │
BlockProducer.produce_one()
  ├── VM Executor per-tx          →  StateDiff { balances, nonces }
  ├── apply_transfer()            →  StateManager  (sender nonce++)
  ├── StateDiff.balances          →  contract side-effects
  ├── StateDiff.nonces            →  smart-contract nonce sync
  ├── advance_block()             →  flush cache · SparseMerkleTrie root
  ├── sign_block()                →  validator_signature (96 bytes)
  └── generate_zkp()             →  64-byte Fiat-Shamir commitment
        │
GossipBridge  →  P2PNode  →  peers
        │
FinalizedBlock  →  Scheduler tasks  →  RPC counters
```

### Inbound peer-block pipeline

```
P2PNode.recv()
  │  serde_json::from_slice  →  Block
  │
BlockValidator::validate_block()   — validator_signature + verify_zkp() (64-byte)
  │
per-tx SPHINCS+ sig check          — empty sigs skipped (legacy compat)
  │
height check  →  skip if already have
  │
Blockchain::add_block()
  │
StateManager::advance_block()
```

---

## Crate Map

| Crate | Version | Purpose |
|---|---|---|
| `bleep-core` | 0.1.0 | Block, Transaction, Blockchain, Mempool, TransactionPool, BlockValidator |
| `bleep-crypto` | 0.1.0 | SPHINCS+, Kyber-768, Falcon, BIP-39, AES-256-GCM, tx signer, ZKP verify |
| `bleep-consensus` | 0.1.0 | BlockProducer, PoS/PBFT/PoW orchestrator, Epoch, Slashing, Finality |
| `bleep-state` | 1.0.0 | StateManager (RocksDB), SparseMerkleTrie, Sharding, 2PC, Self-healing |
| `bleep-vm` | 0.5.0 | 7-layer intent VM: EVM (revm), WASM (wasmer/Cranelift), ZK, unified gas |
| `bleep-p2p` | 0.1.0 | libp2p, Kademlia DHT, Gossip (Plumtree), Onion routing, AI peer scoring |
| `bleep-rpc` | 1.0.0 | warp HTTP/JSON server (10 endpoints, live StateManager integration) |
| `bleep-wallet-core` | 0.1.0 | EncryptedWallet, AES-256-GCM key-at-rest, WalletManager |
| `bleep-cli` | 1.0.0 | clap async CLI: wallet, tx, block, governance, state, ZKP, AI, PAT |
| `bleep-governance` | 0.1.0 | On-chain proposals, voting windows, tally, GovernanceEngine |
| `bleep-economics` | 0.1.0 | Tokenomics engine, EIP-1559-style fee market, validator incentives, oracle |
| `bleep-pat` | 1.0.0 | Programmable Asset Token (PAT), 1 B token supply cap |
| `bleep-interop` | 0.1.0 | BLEEP Connect: 10 sub-crates, 4-tier cross-chain protocol |
| `bleep-zkp` | 0.1.0 | winterfell STARK circuits, transparent proofs, Prover/Verifier API |
| `bleep-scheduler` | 0.1.0 | 20-task Tokio scheduler: epoch, rewards, healing, mempool, indexer |
| `bleep-auth` | 0.1.0 | JWT sessions, RBAC, Merkle-chained audit log, Kyber validator binding |
| `bleep-indexer` | 0.1.0 | DashMap chain indexer, reorg rollback, checkpoint engine |
| `bleep-ai` | 0.1.0 | Deterministic AI advisory engine (BLEEPAIAssistant) |
| `bleep-telemetry` | 0.1.0 | tracing-subscriber, MetricCounter, MetricGauge |

---

## Core Subsystems

### Cryptography

**Crate:** `bleep-crypto`

BLEEP uses post-quantum algorithms as the primary security layer, not as a future upgrade path.

#### Transaction signing — SPHINCS+-SHAKE-256-simple

All transactions are signed with SPHINCS+-SHAKE-256-simple via `pqcrypto-sphincsplus`. The canonical signed payload is:

```
payload = SHA3-256( sender_bytes || receiver_bytes || amount_le8 || timestamp_le8 )
```

```rust
use bleep_crypto::{sign_tx_payload, verify_tx_signature, tx_payload, generate_tx_keypair};

let (pk, sk) = generate_tx_keypair();
let payload  = tx_payload(&sender, &receiver, amount, timestamp);
let sig      = sign_tx_payload(&payload, &sk)?;
assert!(verify_tx_signature(&payload, &sig, &sk));
```

#### Key encapsulation — Kyber-768

Wallets carry a Kyber-768 KEM public key used for encrypted P2P session establishment and validator identity binding.

```rust
use bleep_crypto::quantum_resistance::{generate_falcon_keypair, generate_kyber_keypair};

let (sphincs_pk, sphincs_sk) = generate_falcon_keypair()?;  // SPHINCS+ keypair
let (kyber_pk, kyber_sk)     = generate_kyber_keypair()?;   // Kyber-768 keypair
```

#### Signing-key encryption — AES-256-GCM

Signing keys are never stored in plaintext. Each `EncryptedWallet.signing_key` holds:

```
blob             = nonce(12 bytes) || AES-256-GCM-ciphertext || GCM-tag(16 bytes)
encryption_key   = SHA3-256( password_utf8 || address_utf8 )   →  32 bytes
```

```rust
use bleep_wallet_core::wallet::EncryptedWallet;

let w    = EncryptedWallet::with_signing_key_encrypted(pk, &sk, kyber_pk, "passphrase")?;
let sk   = w.unlock("passphrase")?;     // decrypt → plaintext SPHINCS+ sk
w.lock(&new_sk, "passphrase")?;         // re-encrypt and replace
assert!(w.can_sign());                  // true when signing_key is non-empty
```

The nonce is randomly generated for every `lock()` or `with_signing_key_encrypted()` call, ensuring two encryptions of the same key produce different blobs.

#### BIP-39 wallet import

Standard BIP-39: PBKDF2-HMAC-SHA512, 2,048 rounds, optional passphrase.

```rust
use bleep_crypto::{validate_mnemonic, mnemonic_to_seed, mnemonic_to_bleep_seed};

validate_mnemonic("abandon abandon ... about")?; // 12/15/18/21/24 words
let seed_64 = mnemonic_to_seed("...", "TREZOR")?;   // [u8; 64]
let seed_32 = mnemonic_to_bleep_seed("...", "")?;   // first 32 bytes used as SPHINCS+ seed
```

#### Address format

```
address = "BLEEP1" + lower_hex( SHA256( SHA256( public_key ) )[..20] )
```

Example: `BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8`

---

### Consensus Engine

**Crate:** `bleep-consensus`

Three interchangeable consensus modes are managed by `ConsensusOrchestrator`. Mode selection is a pure deterministic function of epoch metrics — identical on every honest node, evaluated at epoch boundaries only.

| Mode | Trigger | Finality guarantee |
|---|---|---|
| `PosNormal` | Default | >⅔ stake, one-epoch finality |
| `PbftFastFinality` | High-throughput epochs | BFT immediate, >⅔ validator signatures |
| `EmergencyPoW` | Validator liveness failure | Hash-based fallback until recovery |

#### Block production constants

| Constant | Value |
|---|---|
| `BLOCK_INTERVAL_MS` | 3,000 ms |
| `MAX_TXS_PER_BLOCK` | 4,096 |
| Epoch length | 1,000 blocks |

#### Block signing layout

`sign_block()` stores a 96-byte `validator_signature`:

```
[0..32]   validator public key  = SHA3-256(sk_seed)
[32..64]  message hash          = SHA3-256(block_hash_hex)
[64..96]  signature proof       = SHA3-256(msg_hash || sk_seed)
```

`verify_signature(public_key)` checks all three fields. A block with an empty or malformed signature is rejected without executing the ZKP check.

#### 64-byte Fiat-Shamir ZK commitment

Every signed block carries a `zk_proof` field produced by `generate_zkp()` and verified by `verify_zkp()`:

```
challenge = SHA3-256(
    "BLEEP-ZKP-v1"          ← domain separator
    || block_hash           ← commits to all header fields
    || validator_pk         ← binds to the signing key
    || epoch_id_le8
    || protocol_version_le4
    || consensus_mode_u8
    || merkle_root_bytes    ← binds to account state
    || shard_id_le8
    || shard_state_root     ← binds to shard state
    || tx_count_le8         ← binds to transaction set
)

response  = SHA3-256( challenge || validator_pk || block_index_le8 )

zk_proof  = challenge[32] || response[32]   →  64 bytes
```

Winterfell STARK scheme is production-grade — transparent, post-quantum secure, no trusted setup.

#### Finality

`FinalityManager` accumulates `ValidatorSignature` entries into `FinalizyCertificate` records. A block is finalized when `accumulated_voting_power > (2/3) * total_stake`. Once finalized, a block's state root cannot be rolled back.

#### Slashing

`SlashingEngine` applies automatic penalties:

- **Double-signing:** 33% of validator stake slashed immediately
- **Liveness failure:** tracked per epoch, escalates to de-registration after threshold misses

---

### State Layer

**Crate:** `bleep-state`

#### StateManager

RocksDB-backed with LZ4 compression, 512 max open files, and an in-memory write-back cache for hot-path performance. Account records are persisted under the key prefix `b"acct:"`:

```
Key:   b"acct:" + address_utf8
Value: bincode( AccountState { balance: u128, nonce: u64, code_hash: Option<[u8;32]> } )
```

Core API:

```rust
let mut state = StateManager::open("/var/lib/bleep/state")?;

state.get_balance("BLEEP1...");                      // → u128
state.set_balance("BLEEP1...", 1_000_000);
state.get_nonce("BLEEP1...");                        // → u64
state.increment_nonce("BLEEP1...");                  // increments and returns new nonce

// Atomic transfer: debit sender, credit receiver, increment sender nonce
state.apply_transfer(sender, receiver, 500_000_u128);

state.advance_block();   // flush dirty cache → RocksDB, sync trie, persist height
state.state_root();      // → [u8; 32]  SparseMerkleTrie root
```

`advance_block()` is the commit boundary. All writes before it are buffered; a crash before `advance_block()` leaves the previous block's state intact.

#### SparseMerkleTrie — O(k + 256) Merkle proofs

A full 256-bit-depth Sparse Merkle Trie where each account maps to exactly one leaf. The trie provides a cryptographic state commitment that is included in every block and verified by the ZKP.

```
Leaf key   =  blake3( address_utf8 )
Leaf value =  blake3( abi_encode(address, balance, nonce) )
Interior   =  blake3( left_child || right_child )
Empty node =  [0u8; 32]
```

Proof generation: `prove()` builds a **depth-bucketed sibling index** from `interior_cache` in O(k) then does exactly 256 O(1) lookups — one per trie level. Total: **O(k + 256)**.

```rust
// Node side
let proof = state.prove_account("BLEEP1...");

// Light-client side (no node required)
assert!(proof.verify(&known_state_root));
```

`MerkleProof` serialises to JSON for delivery via `/rpc/proof/{address}`.

#### Sharding

Horizontal sharding with deterministic assignment:

- `ShardRegistry` — canonical topology per epoch (all nodes compute identically)
- `ShardValidatorAssignment` — deterministic validator-to-shard mapping
- `CrossShard2PC` — Byzantine-safe Two-Phase Commit coordinator (Prepare → Commit | Abort)
- `SelfHealingOrchestrator` + `AdvancedFaultDetector` — automatic shard recovery on fault detection
- `SnapshotEngine` / `RollbackEngine` — crash recovery to any previous finalized height

---

### Universal VM

**Crate:** `bleep-vm` (v0.5.0)

Execution is intent-driven. Callers submit typed `Intent` values; the `VmRouter` dispatches to the appropriate engine and enforces gas limits.

#### 7-layer architecture

| Layer | Responsibility |
|---|---|
| 1 — Intent | `TransferIntent`, `ContractCallIntent`, `DeployIntent`, `CrossChainIntent`, `ZkVerifyIntent` |
| 2 — Router | Engine selection, gas budget validation, circuit breaker, per-engine metrics |
| 3 — Engines | EVM via `revm 3.5`, WASM via `wasmer 4.2` (Cranelift backend), ZK via `winterfell` (STARK, transparent) |
| 4 — Sandbox | Memory limits, call-stack depth enforcement, host API filtering |
| 5 — State transition | Returns `StateDiff`; never writes `StateManager` directly |
| 6 — Unified gas | EVM, WASM, ZK, SBF, Move gas normalised to a single BLEEP gas unit |
| 7 — Cross-chain | `bleep_call(chain, contract, data)` routes to BLEEP Connect Layer 4 |

#### StateDiff contract

The VM never acquires the `StateManager` lock. It returns:

```rust
pub struct StateDiff {
    pub balances: BTreeMap<[u8; 32], BalanceDelta>,   // address → signed delta
    pub nonces:   BTreeMap<[u8; 32], NonceUpdate>,    // address → new nonce
    // storage slots, deployed code, emitted events …
}
```

`BlockProducer` applies the diff under a single `StateManager` lock after all VM calls complete.

---

### P2P Networking

**Crate:** `bleep-p2p` — Default listen: `0.0.0.0:7700`

Built on `libp2p 0.53`. All transport-layer security is post-quantum.

#### Components

| Component | Description |
|---|---|
| `KademliaDHT` | 256 K-buckets, XOR metric, k=20 replication factor |
| `GossipProtocol` | Plumtree epidemic dissemination for blocks and transactions |
| `OnionRouter` | 3-hop encrypted routing; Kyber-768 KEM per hop, AES-256-GCM payload |
| `PeerManager` | AI-scored peer reputation, Sybil detection, exponential reputation decay |
| `MessageProtocol` | TCP framing, AES-256-GCM encryption, Ed25519 message auth, anti-replay nonce cache |
| `QuantumCrypto` | Kyber-768 session KEM, SPHINCS+-SHA2-128s message authentication |

Ed25519 in `MessageProtocol` is scheduled for replacement with SPHINCS+ in a future release.

---

### RPC Server

**Crate:** `bleep-rpc` — Port: 8545

`warp`-based HTTP/JSON server. `RpcState` holds shared live counters and an `Arc<Mutex<StateManager>>` for the state and proof endpoints.

#### Endpoint summary

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/health` | Status, chain height, peer count, uptime, version |
| `GET` | `/rpc/telemetry` | Blocks produced, transactions processed, uptime |
| `GET` | `/rpc/block/latest` | Latest block height and hash |
| `GET` | `/rpc/block/{id}` | Block by height or hash |
| `POST` | `/rpc/tx` | Submit a signed transaction |
| `GET` | `/rpc/tx/history` | Transaction history |
| `GET` | `/rpc/wallet` | Wallet RPC readiness |
| `GET` | `/rpc/ai` | AI advisory readiness |
| `GET` | `/rpc/state/{address}` | Live balance, nonce, state root, block height |
| `GET` | `/rpc/proof/{address}` | 256-level SMT inclusion/exclusion proof |
| `POST` | `/rpc/validator/stake` | Register validator / increase stake |
| `POST` | `/rpc/validator/unstake` | Initiate graceful validator exit |
| `GET` | `/rpc/validator/list` | All active validators with stake |
| `GET` | `/rpc/validator/status/{id}` | Validator status + slashing history |
| `POST` | `/rpc/validator/evidence` | Submit double-sign evidence (auto-execute) |
| `GET` | `/rpc/economics/supply` | Circulating supply, minted, burned, base fee  |
| `GET` | `/rpc/economics/fee` | Current EIP-1559 base fee + last epoch  |
| `GET` | `/rpc/economics/epoch/{n}` | Full epoch output (emissions, burns, price)  |
| `GET` | `/rpc/oracle/price/{asset}` | Aggregated oracle price (median, sources)  |
| `POST` | `/rpc/oracle/update` | Submit oracle price update  |
| `GET` | `/rpc/connect/intents/pending` | Pending Layer 4 instant intents  |
| `POST` | `/rpc/connect/intent` | Submit a new instant intent  |

The `/rpc/economics/*` and `/rpc/oracle/*` handlers return HTTP 503 when `BleepEconomicsRuntime` is not attached.

#### Wiring at startup

```rust
let rpc_state = RpcState::new()
    .with_state_manager(Arc::clone(&state_arc))
    .with_validator_registry(Arc::clone(&validator_registry))
    .with_slashing_engine(Arc::clone(&slashing_engine))
    .with_economics_runtime(Arc::clone(&economics_runtime));
let routes = rpc_routes_with_state(rpc_state);
warp::serve(routes).run(([0, 0, 0, 0], 8545)).await;
```

---

### Wallet and CLI

**Crates:** `bleep-wallet-core`, `bleep-cli`

#### Wallet file format

Wallets are stored as a JSON array at `~/.bleep/wallets.json`:

```json
[
  {
    "falcon_keys":  "<hex: SPHINCS+ public key>",
    "kyber_keys":   "<hex: Kyber-768 public key>",
    "signing_key":  "<hex: AES-256-GCM ciphertext of SPHINCS+ secret key>",
    "address":      "BLEEP1<40-hex-chars>",
    "label":        null
  }
]
```

The `signing_key` blob layout: `nonce(12) || ciphertext || GCM-tag(16)`. A wallet without `signing_key` is watch-only.

#### CLI commands

```
bleep-cli <COMMAND>

  start-node               Start a full BLEEP node (all subsystems)

  wallet create            Generate SPHINCS+ + Kyber-768 keypair; encrypt SK
  wallet balance           GET /rpc/state per wallet; fallback to local RocksDB
  wallet import <phrase>   BIP-39 mnemonic → PBKDF2 seed → SPHINCS+ keypair
  wallet export            Print wallet addresses

  tx send --to <addr> --amount <n>
                           Sign with SPHINCS+ (unlock SK); POST /rpc/tx
  tx history               GET /rpc/tx/history

  block latest             GET /rpc/block/latest
  block get <id>           GET /rpc/block/{id}
  block validate <hash>    Validate block hash via node

  governance <propose|vote|list|status>
                           Submit and vote on on-chain proposals

  state <task>             Inspect raw chain state
  zkp <proof>              Verify a ZK proof string
  ai <task>                Query AI advisory engine
  pat <task>               Programmable Asset Token operations
  telemetry                Print live node metrics
  info                     Print RPC and node connection info
```

#### Environment variables

| Variable | Default | Description |
|---|---|---|
| `BLEEP_RPC` | `http://127.0.0.1:8545` | RPC endpoint for all CLI commands |
| `BLEEP_STATE_DIR` | `/tmp/bleep-state` | Local RocksDB path (offline fallback) |
| `RUST_LOG` | `info` | tracing log filter |

#### Balance resolution order

1. `GET /rpc/state/{address}` on the configured node (returns balance + nonce + state root prefix)
2. If the node is unreachable: `StateManager::open(BLEEP_STATE_DIR)` — local RocksDB read
3. Prints `(offline — node at {rpc} unreachable)` when the fallback is used

---

### Governance

**Crate:** `bleep-governance`

On-chain governance with six typed proposal categories:

```rust
pub enum ProposalType {
    ParameterChange,
    ProtocolUpgrade,
    ValidatorSlashing,
    EmergencyPause,
    TreasurySpend,
    CrossChainPolicy,
}
```

`GovernanceEngine` manages the full lifecycle: proposal creation, voting window, quorum evaluation, tally, and automatic execution or archival. Proposals require a configurable quorum threshold and approval percentage. Rejected or expired proposals are archived, never deleted.

---

### Economics and Tokenomics

**Crate:** `bleep-economics`

#### Supply model

| Constant | Value |
|---|---|
| `MAX_SUPPLY` | 200,000,000 BLEEP (8 decimals) |
| `GENESIS_SUPPLY` | 0 (fair launch, no pre-mine) |
| `MAX_INFLATION_RATE_BPS` | 500 (5.00% per epoch, constitutional hard cap) |
| Base fee burn | 25% of every transaction fee |

`CanonicalTokenomicsEngine` enforces `total_minted ≤ MAX_SUPPLY` independently of `circulating_supply`, preventing a class of inflation bug where concurrent burns mask over-issuance.

#### Emission schedule

| Type | Rate per epoch |
|---|---|
| Validator participation reward | 1.5% |
| Cross-shard coordination reward | 0.5% |
| Ecosystem / governance grants | Governance-controlled |

#### Fee market

An EIP-1559-style base fee adjusted per block by `ShardCongestion` metrics. The base fee is burned; validators receive only the priority tip. This creates deflationary pressure under high network load.

#### Oracle bridge

`OracleBridgeEngine` aggregates price updates from multiple `OracleOperator` sources with median filtering and staleness rejection. No price data is committed to state without Byzantine-threshold confirmation.

---

### BLEEP Connect

**Crate:** `bleep-interop` (10 sub-crates)

BLEEP Connect is a four-tier cross-chain protocol. The executor automatically selects the tier based on transfer value and required security level.

| Tier | Latency | Security | Use case |
|---|---|---|---|
| Layer 4 — Instant | 200 ms – 1 s | Optimistic intent relay | Routine transfers |
| Layer 3 — ZK Proof | 10 s – 60 s | Winterfell STARK batch proof | Verified mid-value transfers (post-quantum secure) |
| Layer 2 — Full Node | 1 min – 5 min | Independent chain verification | Transfers > $100K |
| Layer 1 — Social | Hours | On-chain governance vote | Catastrophic recovery events |

#### Sub-crate map

| Sub-crate | Role |
|---|---|
| `bleep-connect-types` | Shared types: `ChainId`, `UniversalAddress`, `AssetId`, `InstantIntent` |
| `bleep-connect-crypto` | SPHINCS+, Kyber-1024, Ed25519, AES-GCM per-hop encryption |
| `bleep-connect-commitment-chain` | BFT micro-chain anchoring cross-chain state roots |
| `bleep-connect-adapters` | Per-chain encode/verify: Ethereum (EVM), Solana |
| `bleep-connect-executor` | Executor node: monitors intent pool, bids, executes |
| `bleep-connect-layer4-instant` | Optimistic 200 ms relay (99.9% of transfers) |
| `bleep-connect-layer3-zkproof` | Winterfell STARK proof generation and batch aggregation (transparent, post-quantum) |
| `bleep-connect-layer2-fullnode` | Full-node verification path for large transfers |
| `bleep-connect-layer1-social` | On-chain social governance for catastrophic recovery |
| `bleep-connect-core` | Top-level orchestrator over all layers |

---

### Supporting Services

#### Scheduler — `bleep-scheduler`

20 built-in Tokio maintenance tasks across 7 categories, each with an isolated per-task timeout and panic boundary:

Epoch management, validator reward distribution, self-healing sweeps, governance advancement, EIP-1559 fee parameter updates, supply invariant verification, shard rebalancing, session purges, mempool pruning, indexer checkpoints, cross-shard timeout sweeps, telemetry flush, peer health checks, oracle data refresh, and audit log rotation.

#### Auth — `bleep-auth`

Complete authentication surface for node operators and dApp developers:

- **Credentials** — SHA3-256 salted hashing with constant-time verification and `Zeroize` on drop
- **Sessions** — HS256 JWT issuance and validation with JTI deny-list revocation
- **RBAC** — Role hierarchy with O(1) `DashMap`-backed permission evaluation
- **Validator binding** — Kyber-1024 challenge/response proof-of-possession
- **Audit log** — Merkle-chained, append-only, tamper-detectable event log
- **Rate limiter** — Fixed-window token bucket per `(identity, action)` pair

#### Indexer — `bleep-indexer`

Async channel-driven indexer building `DashMap`-backed query indexes for blocks, transactions, accounts, governance events, validator events, shard events, cross-shard 2PC, and AI advisory events. Supports reorg rollback to any ancestor height and `CheckpointEngine` snapshots for crash recovery.

#### AI Advisory — `bleep-ai`

`BLEEPAIAssistant` produces deterministic advisory scores used by P2P peer selection, consensus anomaly detection, and governance risk scoring. Deterministic means identical inputs always produce identical outputs with no external model or API calls at runtime.

#### Telemetry — `bleep-telemetry`

`tracing-subscriber` integration with `MetricCounter` and `MetricGauge` primitives. All counters are aggregated into `RpcState` and exposed via `/rpc/telemetry`.

---

## Protocol Parameters

| Parameter | Value |
|---|---|
| Block time | 3,000 ms |
| Max transactions per block | 4,096 |
| Blocks per epoch | 1,000 |
| Finality threshold | > ⅔ total stake |
| Double-sign slash | 33% of validator stake |
| Max token supply | 200,000,000 BLEEP |
| Token decimals | 8 |
| Max inflation rate | 500 bps / epoch (5%) |
| Base fee burn | 25% of transaction fee |
| State trie depth | 256 bits |
| ZKP proof size | 64 bytes (Fiat-Shamir, current) |
| RPC port | 8545 |
| P2P port | 7700 |
| BIP-39 PBKDF2 rounds | 2,048 |
| AES-GCM nonce size | 12 bytes (random per encryption) |

---

## Getting Started

### Prerequisites

```bash
# Rust stable toolchain, edition 2021
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Ubuntu / Debian system dependencies
sudo apt-get install -y build-essential clang libclang-dev librocksdb-dev

# macOS (Homebrew)
brew install rocksdb llvm
export LIBRARY_PATH="$(brew --prefix rocksdb)/lib:$LIBRARY_PATH"
```

### Build

```bash
git clone https://github.com/bleep-project/bleep.git
cd bleep

# Full workspace (all 19 crates)
cargo build --release --workspace

# Node binary only
cargo build --release --bin bleep

# CLI only
cargo build --release --bin bleep-cli
```

### Run a local single-validator node

```bash
./target/release/bleep
# Node starts in 13 steps; RPC ready at :8545, P2P at :7700

# Verify the node is live
curl -s http://127.0.0.1:8545/rpc/health | jq .
```

### Create a wallet and send a transaction

```bash
# 1. Create wallet
./target/release/bleep-cli wallet create
# → Generates SPHINCS+ + Kyber-768 keypair
# → Prompts for encryption passphrase
# → Saves to ~/.bleep/wallets.json

# 2. Check balance (live from node)
./target/release/bleep-cli wallet balance

# 3. Import from BIP-39 mnemonic
./target/release/bleep-cli wallet import \
  "abandon abandon abandon abandon abandon abandon \
   abandon abandon abandon abandon abandon about"

# 4. Send a transaction (prompts for passphrase to unlock signing key)
./target/release/bleep-cli tx send \
  --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 \
  --amount 1000
```

### Run the test suite

```bash
# All workspace unit and integration tests
cargo test --workspace

# Single crate
cargo test -p bleep-state
cargo test -p bleep-crypto
cargo test -p bleep-wallet-core
cargo test -p bleep-consensus

# With detailed output
RUST_LOG=debug cargo test -p bleep-state -- --nocapture
```

---

## Configuration

The node reads from environment variables and an optional `bleep.toml` (via `config 0.14`).

```toml
# bleep.toml (all values shown are defaults)

[node]
p2p_port    = 7700
rpc_port    = 8545
state_dir   = "/var/lib/bleep/state"
log_level   = "info"

[consensus]
block_interval_ms  = 3000
max_txs_per_block  = 4096
blocks_per_epoch   = 1000
validator_id       = "validator-0"

[features]
quantum = true    # disable only for development/benchmarking
```

#### Cargo feature flags

| Flag | Default | Effect |
|---|---|---|
| `mainnet` | on | Mainnet protocol constants |
| `testnet` | off | Testnet constants (e.g. reduced epoch size) |
| `quantum` | on | Enables `pqcrypto` and `pqcrypto-kyber`; required for SPHINCS+/Kyber |

---

## RPC API Reference

All responses are JSON. Errors carry an `"error"` string field.

### `GET /rpc/health`

```json
{
  "status":      "ok",
  "height":      1024,
  "peers":       8,
  "uptime_secs": 3600,
  "version":     "1.0.0"
}
```

### `GET /rpc/telemetry`

```json
{
  "blocks_produced":        1024,
  "transactions_processed": 12800,
  "uptime_secs":            3600
}
```

### `GET /rpc/state/{address}`

Returns HTTP 503 in stub mode (no `StateManager` attached). `balance` is a decimal string to avoid JSON `u128` overflow.

```json
{
  "address":      "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "balance":      "10000000000",
  "nonce":        4,
  "state_root":   "8a3f2c1d9e7b4a0f5c6d7e8f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
  "block_height": 1024
}
```

### `GET /rpc/proof/{address}`

`exists: false` indicates an exclusion proof (no account at this address).

```json
{
  "address":  "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "exists":   true,
  "leaf":     "3f2a1b0c...",
  "root":     "8a3f2c1d...",
  "siblings": ["00000000...", "4a2b1c3d...", "..."],
  "is_right": [false, true, false, "..."]
}
```

Verify offline:

```rust
let proof: MerkleProof = serde_json::from_str(&json_body)?;
assert!(proof.verify(&known_state_root));
```

### `POST /rpc/tx`

```json
// Request
{
  "sender":    "BLEEP1...",
  "receiver":  "BLEEP1...",
  "amount":    1000,
  "timestamp": 1710000000
}

// Response
{
  "tx_id":  "BLEEP1...:BLEEP1...:1000:1710000000",
  "status": "queued"
}
```

### `GET /rpc/block/latest`

```json
{ "height": 1024, "hash": "00000...0400", "tx_count": 0, "epoch": 1 }
```

---

## CLI Reference

```
bleep-cli [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version

Environment variables:
  BLEEP_RPC        RPC endpoint     default: http://127.0.0.1:8545
  BLEEP_STATE_DIR  Local DB path    default: /tmp/bleep-state
  RUST_LOG         Log filter       default: info

Commands:
  start-node                        Start a full BLEEP node
  wallet create                     Generate SPHINCS+ keypair + encrypted wallet
  wallet balance                    Query balance from /rpc/state
  wallet import <phrase>            Import from BIP-39 mnemonic
  wallet export                     Export wallet addresses
  tx send --to <addr> --amount <n>  Sign and broadcast a transfer
  tx history                        Retrieve transaction history
  validator stake --amount <n>      Register as validator
  validator unstake                 Initiate graceful exit
  validator list                    List active validators
  validator status <id>             Validator status + slashing history
  validator submit-evidence <file>  Submit double-sign evidence
  governance propose <text>         Create a governance proposal
  governance vote <id> --yes/--no   Cast a vote
  governance list                   List all proposals
  state snapshot                    Create a RocksDB state snapshot
  state restore <path>              Restore from snapshot
  block latest                      Print the latest block
  block get <id>                    Get a block by hash or height
  block validate <hash>             Validate a block by hash
  zkp <proof>                       Verify a ZKP
  ai ask <prompt>                   Ask the AI advisory engine
  ai status                         AI engine status
  pat status                        PAT engine status
  pat list                          List asset tokens
  pat mint --to <addr> --amount <n> Mint PAT tokens (owner only)        
  pat burn --amount <n>             Burn PAT tokens                      
  pat transfer --to <addr> --amount Transfer PAT with auto burn-rate     
  pat balance <address>             Query PAT balance                    
  oracle price <asset>              Query aggregated oracle price        
  oracle submit --asset ... --price Submit oracle price update           
  economics supply                  Circulating supply, minted, burned   
  economics fee                     Current EIP-1559 base fee            
  economics epoch <n>               Epoch emissions, burns, price        
  telemetry                         Print telemetry metrics
  info                              Node version and RPC health
```

### Executor node

The `bleep-executor` binary is a separate process that participates in the Layer 4 instant intent market:

```bash
# Basic usage (ephemeral key, 0.1 BLEEP capital)
./bleep-executor

# Production usage
BLEEP_EXECUTOR_KEY=<32-byte-hex-seed>      \
BLEEP_EXECUTOR_CAPITAL_BLEEP=100000000000  \
BLEEP_EXECUTOR_CAPITAL_ETH=10000000000000000000 \
BLEEP_EXECUTOR_RISK=Medium                 \
BLEEP_RPC=http://your-node:8545            \
./bleep-executor
```



## Security Model

### Post-quantum threat model

BLEEP treats a cryptographically relevant quantum computer as a near-term engineering assumption, not a distant theoretical risk. Accordingly:

- **SPHINCS+-SHAKE-256** (NIST PQC, stateless hash-based) signs all transactions and blocks.
- **Kyber-768** (ML-KEM, NIST FIPS 203) is used for all key encapsulation.
- **AES-256-GCM** is used for symmetric encryption (128-bit post-quantum security at 256-bit key size).
- **SHA3-256 and BLAKE3** provide collision-resistant hashing.
- **Ed25519** is retained in P2P message authentication at the 128-bit classical security level and is scheduled for replacement with SPHINCS+.

### Consensus safety

- **Byzantine fault tolerance:** The system tolerates up to ⅓ of total stake being Byzantine (malicious or offline).
- **Deterministic mode selection:** No single validator can trigger a mode switch. Selection is a pure function of epoch metrics.
- **Automatic slashing:** Double-sign evidence triggers an on-chain 33% stake penalty without human intervention.
- **Irreversible finality:** Once `accumulated_voting_power > (2/3) * total_stake`, a `FinalizyCertificate` is produced. That block and its state root cannot be rolled back.

### State integrity

- **SparseMerkleTrie** ensures any modification to any account balance or nonce produces a different state root, which propagates through the block hash to the `validator_signature` and `zk_proof`. Tampered state is detectable by any node holding the state root.
- **Fiat-Shamir ZKP** binds the state root, shard state root, consensus mode, and tx count into `zk_proof`. A validator cannot produce a valid ZKP over a different set of transactions or state root.
- **Inflation invariant:** `CanonicalTokenomicsEngine` checks `total_minted ≤ MAX_SUPPLY` independently of `circulating_supply`. Concurrent burns cannot mask over-issuance.

### P2P security

- All sessions are established with Kyber-768 KEM; payload encryption is AES-256-GCM.
- An anti-replay nonce cache rejects duplicate messages within a session window.
- AI-scored peer reputation and stake-weighted selection provide Sybil resistance.
- Onion routing with 3 hops and per-hop Kyber-768 KEM prevents traffic analysis.

---

## Development Roadmap

BLEEP follows a structured, phase-based development roadmap. Phases 1–3 are complete. The four upcoming phases — AI model training, public testnet expansion, pre-sale ICO, and mainnet launch — form the path to production.

---

### Phase 1 — Foundation ✅ *Complete*

All 19 crates compile cleanly. Post-quantum cryptography active (SPHINCS+-SHAKE-256, Kyber-1024). RocksDB `StateManager` with `SparseMerkleTrie`. Full `BlockProducer` loop. Winterfell STARK ZK circuits (transparent, post-quantum secure). 4-node docker-compose devnet. BLEEP Connect Layer 4 live on Ethereum Sepolia. `BleepEconomicsRuntime` (EIP-1559 fee market, oracle bridge, validator incentives). `PATRegistry` live. `bleep-executor` standalone intent market maker.

---

### Phase 2 — Testnet Alpha ✅ *Complete*

7-validator `bleep-testnet-1` genesis across 4 continents. Public DNS seeds at `seeds.testnet.bleep.network`. Public faucet (`POST /faucet/{address}`, 1,000 BLEEP per 24 hours). Block explorer (`GET /explorer`, 6 s refresh). JWT rotation, NDJSON audit export. Grafana dashboard (12 panels) + Prometheus for all 7 validators. Full CI pipeline: fmt, clippy, test, audit, build, fuzz-smoke, docker-smoke.

---

### Phase 3 — Protocol Hardening ✅ *Complete*

- ✅ **Independent security audit** — 14 findings (2 Critical, 3 High, 4 Medium, 3 Low, 2 Info); all Critical/High resolved — see `docs/SECURITY_AUDIT.md`
- ✅ **Chaos testing** — 14 scenarios, 72-hour continuous harness — see `docs/CHAOS_TESTING.md`
- ✅ **ZKP MPC ceremony** — 5-participant Powers-of-Tau on BLS12-381; transcript at `https://ceremony.bleep.network/transcript-v1.json`
- ✅ **Cross-shard stress test** — 10 shards, 1,000 concurrent cross-shard txs, 100 epochs
- ✅ **BLEEP Connect Layer 3** — Winterfell STARK batch proof bridge live on testnet (post-quantum secure)
- ✅ **Live governance** — `LiveGovernanceEngine` with typed proposals, weighted voting, veto, on-chain execution
- ✅ **Performance benchmark** — avg **10,921 TPS**, peak **13,200 TPS** across 10 shards for 1 hour
- ✅ **Token distribution model** — 6 allocation buckets, vesting schedules, 25/50/25 fee split, compile-time verified constants

**Definition of done:** Security audit fully resolved ✅ · Chaos suite 72 h ✅ · ≥10,000 TPS ✅

---

### Phase 4 — AI Model Training ⏳ *Active*

Upgrade `bleep-ai` from rule-based advisory to a trained on-chain inference engine.

- `BLEEPAIAssistant v2` — training pipeline using on-chain governance history as training data
- AI validator nodes — optional validator upgrade for AI-scored transaction prioritisation
- `AIConstraintValidator v2` — trained classification models for governance pre-flight scoring
- Determinism guarantee — all AI inference on consensus-critical paths uses fixed-seed reproducible models

**Definition of done:** AI advisory engine passes determinism test suite; governance pre-flight achieves ≥95% accuracy on labelled test set.

---

### Phase 5 — Public Testnet Expansion ⏳ *Upcoming*

Open validator onboarding to the public. Target: ≥50 validators across ≥6 continents.

- Open validator registration with public `VALIDATOR_GUIDE.md`
- Validator incentive programme from Ecosystem Fund
- `testnet.bleep.network` — multi-validator explorer, leaderboard, public dashboard
- 30-day sustained test with live validator join/leave events
- Cross-shard expansion: 10 → 20 shards as validator count permits
- Community bug bounty: up to 100,000 BLEEP for documented protocol vulnerabilities

**Definition of done:** 50+ active validators; 30 consecutive days without manual intervention.

---

### Phase 6 — Pre-Sale / ICO ⏳ *Upcoming*

Community token sale in two tranches. Deploy on-chain vesting contracts.

| Tranche | Source | Lockup |
|---|---|---|
| Strategic Pre-Sale | Strategic Reserve (5M BLEEP) | 12-month cliff + 24-month linear |
| Public ICO | Community Incentives (up to 10M BLEEP) | 6-month linear vest |

KYC/AML compliant infrastructure · Multi-sig treasury custody · `LinearVestingSchedule` contracts deployed · `GenesisAllocation` engine activated for all 6 buckets.

**Definition of done:** ICO completed; all vesting contracts deployed and verified on-chain.

---

### Phase 7 — Mainnet Launch 🔜 *Planned*

Production mainnet with post-quantum security, live economics, and cross-chain connectivity from the genesis block.

- Mainnet genesis ceremony — public, multi-party verifiable
- ≥21 validators with geographic diversity enforced by genesis rules
- BLEEP Connect L4 + L3 live on Ethereum mainnet and Solana from genesis
- Governance active from block 1
- Full tokenomics live — emission, burn, staking rewards, oracle, EIP-1559 fee market
- Block explorer at `explorer.bleep.network`
- `bleep-sdk-js` and `bleep-sdk-python` SDK releases
- NTP drift guard at node startup (warn >1 s, halt >30 s)

**Definition of done:** Genesis block produced by ≥21 independent validators; governance proposal passes on-chain within first week; cross-chain Ethereum transfer confirms within 1 second.

---

## Contributing

1. Fork the repository and create a feature branch: `git checkout -b feat/your-feature`
2. Run the full test suite: `cargo test --workspace`
3. Run the linter: `cargo clippy --workspace -- -D warnings`
4. Verify formatting: `cargo fmt --all -- --check`
5. Open a pull request against `main` with a clear description of the change and the crates affected.

Changes to `bleep-consensus`, `bleep-crypto`, or `bleep-state` undergo an extended review given their security surface area.

### Security disclosures

Do not open public issues for security vulnerabilities. Email `security@bleep.network` with a description, reproduction steps, and your proposed fix. We target 72-hour acknowledgment and a 14-day patch timeline.

---

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

---

*BLEEP Blockchain — built in Rust, secured with post-quantum cryptography, designed for the next decade of decentralised computing.*
