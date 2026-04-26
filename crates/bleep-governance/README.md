# bleep-governance

**Self-Amending On-Chain Governance**

`bleep-governance` implements BLEEP's constitutional governance layer: proposal submission, ZKP-backed voting, deterministic execution, AI advisory hooks, forkless protocol upgrades, and reputation-weighted participation.

---

## License

Licensed under **MIT**.
Copyright © 2025 Muhammad Attahir.

---

## Architecture

```
bleep-governance
├── governance_core         — Proposal, Vote, VoteTally, GovernanceEngine, GovernanceError
├── deterministic_executor  — Reproducible, auditable proposal execution
├── constitution            — Immutable base rules that constrain all proposals
├── zk_voting               — ZKP-backed anonymous voting (Winterfell STARK, transparent)
├── proposal_lifecycle      — Submission → Voting → Quorum check → Execution
├── forkless_upgrades       — Live protocol parameter changes without hard forks
├── governance_binding      — Links governance decisions to bleep-state mutations
├── governance_engine       — Phase 5: AI-augmented protocol evolution engine
├── protocol_rules          — Declarative protocol rule registry
├── apip                    — Autonomous Protocol Improvement Proposals
├── safety_constraints      — Governance actions blocked by safety invariants
├── ai_reputation           — Reputation scoring for governance participants
├── protocol_evolution      — Long-range protocol trajectory management
├── ai_hooks                — Advisory hooks from bleep-ai into governance
├── invariant_monitoring    — Continuous safety invariant checks during voting
├── governance_voting       — Vote tallying with quadratic and weighted modes
└── deterministic_activation — Governance-approved changes activated at exact block heights
```

---

## Proposal Lifecycle

```
[Proposer] → submit_proposal()
       ↓
  AI categorises (bleep-ai advisory)
       ↓
  Constitution check (safety_constraints)
       ↓
  Voting window opens (VotingWindow)
       ↓
  Voters cast ZK-backed votes (zk_voting)
       ↓
  Quorum met? → deterministic_executor runs payload
       ↓
  Execution log → blockchain (quantum-encrypted)
       ↓
  bleep-state mutation via governance_binding
```

---

## Proposal Types

| Type | Description |
|------|-------------|
| `ProtocolUpgrade` | Changes to consensus rules, fee model, emission schedule |
| `ValidatorSanction` | Slashing or banning a validator by governance vote |
| `TreasurySpend` | Allocates Foundation Treasury funds |
| `ShardRebalance` | Adjusts shard count or validator assignment |
| `AssetRecovery` | Approves a ZKP-backed anti-asset-loss claim |
| `AIModelUpdate` | Rotates the AI inference model registry |
| `ConstitutionAmendment` | Modifies base constitutional rules (highest quorum required) |

---

## Voting Mechanisms

| Mode | Description |
|------|-------------|
| **Quadratic voting** | Cost of votes increases quadratically; reduces plutocracy |
| **1-token-1-vote** | Simple stake-weighted voting for operational proposals |
| **Category-based** | Different quorum thresholds per proposal category |
| **ZK anonymous** | Bulletproof-backed votes hide voter identity |

---

## Forkless Upgrades

Protocol parameter changes (base fee multiplier, epoch length, emission rate, etc.) are applied at the activation block height determined by `deterministic_activation`. No node restart or hard fork is required.

---

## AI Advisory Integration

`bleep-ai` submits `AIAssessmentProposal`s into governance for validator vote. AI recommendations are:

- Cryptographically attested (`ProofOfInference`)
- Blocked by `safety_constraints` if they violate invariants
- Advisory only — governance always has final authority

---

## Quick Start

```rust
use bleep_governance::{GovernanceEngine, ProposalType};

let engine = GovernanceEngine::new(config);
let proposal_id = engine.submit_proposal(ProposalType::ProtocolUpgrade(payload), submitter)?;
engine.vote(proposal_id, voter_id, vote_weight, zk_proof)?;
engine.try_execute(proposal_id)?;
```

---

## Testing

```bash
cargo test -p bleep-governance
```

Phase 4 and Phase 5 integration tests are in `tests/phase4_governance_tests.rs`, `phase5_integration_tests.rs`, and `phase5_comprehensive_tests.rs`.

---

*Part of the [BLEEP Ecosystem](https://github.com/BleepEcosystem/BLEEP-V1)*
