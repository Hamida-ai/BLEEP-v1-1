//! bleep-consensus/src/chaos_engine.rs
//! Chaos scenario model
//!
//! This module is an *analytical model*, not a live adversarial test.
//!
//! * `Analytical` scenarios (validator crash, network partition, load) evaluate
//!   quorum arithmetic and a block-capacity formula over plain integers. They
//!   do not start, crash, partition, or load any node, and no transaction is
//!   sent.
//! * `NotExercised` scenarios (long-range reorg, double-sign, transaction
//!   replay, eclipse, invalid-block flood) have no code path that attacks a
//!   node. Their text is the *documented expectation* and is reported as not
//!   verified; they never count as passed.
//!
//! Nothing here measures throughput or proves resilience. Evidence for either
//! has to come from running real nodes (see docs/MULTINODE.md).

use std::time::Instant;

// ── Scenario definitions ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChaosScenario {
    /// Kill N validators simultaneously (crash-stop fault model).
    ValidatorCrash { count: usize },
    /// Partition the network into two groups of `group_a` and `group_b` sizes.
    NetworkPartition { group_a: usize, group_b: usize },
    /// Inject a long-range reorg attempt from `fork_depth` blocks behind tip.
    LongRangeReorg { fork_depth: u64 },
    /// One validator signs two different blocks at the same height (equivocation).
    DoubleSign { validator_id: String },
    /// Replay a previously processed transaction.
    TxReplay { tx_id: String },
    /// Eclipse a single validator (all peers are adversarial).
    EclipseAttack { target_validator: String },
    /// Flood the network with invalid blocks.
    InvalidBlockFlood { count: u32 },
    /// Sustained high-load: `tps` transactions per second for `duration_secs`.
    LoadStress { tps: u32, duration_secs: u64 },
}

/// How much of a scenario's result is backed by executed code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosFidelity {
    /// Result comes from arithmetic over the scenario parameters. No node,
    /// network, or consensus engine is exercised.
    Analytical,
    /// No code path in this engine runs the scenario. The outcome text is a
    /// documented expectation only.
    NotExercised,
}

impl ChaosScenario {
    /// What backs this scenario's outcome. No scenario in this engine drives a
    /// running node, so none is reported as live.
    pub fn fidelity(&self) -> ChaosFidelity {
        match self {
            ChaosScenario::ValidatorCrash { .. }
            | ChaosScenario::NetworkPartition { .. }
            | ChaosScenario::LoadStress { .. } => ChaosFidelity::Analytical,
            ChaosScenario::LongRangeReorg { .. }
            | ChaosScenario::DoubleSign { .. }
            | ChaosScenario::TxReplay { .. }
            | ChaosScenario::EclipseAttack { .. }
            | ChaosScenario::InvalidBlockFlood { .. } => ChaosFidelity::NotExercised,
        }
    }
}

// ── Outcome ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChaosOutcome {
    pub scenario: ChaosScenario,
    pub passed: bool,
    pub duration_ms: u64,
    pub chain_height_before: u64,
    pub chain_height_after: u64,
    pub final_consensus: bool,
    pub invariants_held: Vec<String>,
    pub violations: Vec<String>,
    pub notes: String,
}

impl ChaosOutcome {
    /// What backs this outcome (see [`ChaosFidelity`]).
    pub fn fidelity(&self) -> ChaosFidelity {
        self.scenario.fidelity()
    }

    fn pass(
        scenario: ChaosScenario,
        start: Instant,
        h_before: u64,
        h_after: u64,
        notes: &str,
    ) -> Self {
        Self {
            passed: true,
            duration_ms: start.elapsed().as_millis() as u64,
            chain_height_before: h_before,
            chain_height_after: h_after,
            final_consensus: true,
            invariants_held: vec![
                "I-CON1: finality threshold maintained".into(),
                "I-S1: supply conservation".into(),
                "I-S2: no negative balances".into(),
                "I-CON3: no fork past finalised checkpoint".into(),
            ],
            violations: vec![],
            notes: notes.into(),
            scenario,
        }
    }
    fn fail(scenario: ChaosScenario, start: Instant, h_before: u64, violation: &str) -> Self {
        Self {
            passed: false,
            duration_ms: start.elapsed().as_millis() as u64,
            chain_height_before: h_before,
            chain_height_after: h_before,
            final_consensus: false,
            invariants_held: vec![],
            violations: vec![violation.into()],
            notes: "Scenario failed invariant checks".into(),
            scenario,
        }
    }
}

// ── ChaosConfig ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Maximum proportion of validators that can be crashed (BFT bound).
    pub max_crash_fraction: f64,
    /// Minimum blocks that must be produced during a partition scenario.
    pub min_blocks_under_partition: u64,
    /// Recovery window in ms after each scenario before checking consensus.
    pub recovery_window_ms: u64,
    /// Number of repetitions per scenario in the stress suite.
    pub stress_repetitions: u32,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            max_crash_fraction: 0.33,
            min_blocks_under_partition: 5,
            recovery_window_ms: 3_000,
            stress_repetitions: 3,
        }
    }
}

// ── ChaosEngine ───────────────────────────────────────────────────────────────

pub struct ChaosEngine {
    config: ChaosConfig,
    validator_count: usize,
    outcomes: Vec<ChaosOutcome>,
}

impl ChaosEngine {
    pub fn new(validator_count: usize) -> Self {
        Self {
            config: ChaosConfig::default(),
            validator_count,
            outcomes: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: ChaosConfig) -> Self {
        self.config = config;
        self
    }

    /// Run all chaos scenarios in order.
    /// Returns true iff every scenario passed. Scenarios that are
    /// `NotExercised` never count as passed, so this is `false` for the default
    /// suite until those scenarios are implemented against a running node.
    pub fn run_full_suite(&mut self, initial_height: u64) -> bool {
        let scenarios = self.build_scenario_list();
        let mut all_pass = true;
        let mut height = initial_height;
        for scenario in scenarios {
            let outcome = self.run_scenario(scenario, height);
            height = outcome.chain_height_after;
            if !outcome.passed {
                all_pass = false;
            }
            self.outcomes.push(outcome);
        }
        all_pass
    }

    fn build_scenario_list(&self) -> Vec<ChaosScenario> {
        let crash_max = (self.validator_count as f64 * self.config.max_crash_fraction) as usize;
        vec![
            ChaosScenario::ValidatorCrash { count: 1 },
            ChaosScenario::ValidatorCrash { count: crash_max },
            ChaosScenario::NetworkPartition {
                group_a: 4,
                group_b: 3,
            },
            ChaosScenario::NetworkPartition {
                group_a: 5,
                group_b: 2,
            },
            ChaosScenario::LongRangeReorg { fork_depth: 10 },
            ChaosScenario::LongRangeReorg { fork_depth: 50 },
            ChaosScenario::DoubleSign {
                validator_id: "validator-0".into(),
            },
            ChaosScenario::DoubleSign {
                validator_id: "validator-3".into(),
            },
            ChaosScenario::TxReplay {
                tx_id: "replay-test-tx-0001".into(),
            },
            ChaosScenario::EclipseAttack {
                target_validator: "validator-6".into(),
            },
            ChaosScenario::InvalidBlockFlood { count: 1_000 },
            ChaosScenario::LoadStress {
                tps: 500,
                duration_secs: 60,
            },
            ChaosScenario::LoadStress {
                tps: 1_000,
                duration_secs: 60,
            },
            ChaosScenario::LoadStress {
                tps: 1_300,
                duration_secs: 60,
            },
        ]
    }

    fn run_scenario(&mut self, scenario: ChaosScenario, height: u64) -> ChaosOutcome {
        let fidelity = scenario.fidelity();
        let mut outcome = self.run_scenario_model(scenario, height);
        // No scenario here checks the invariants the constructors list, so they
        // must not be reported as held.
        let would_check = outcome.invariants_held.join("; ");
        outcome.invariants_held.clear();
        match fidelity {
            ChaosFidelity::Analytical => {
                outcome.notes = format!("MODEL ONLY, no node exercised: {}", outcome.notes);
            }
            ChaosFidelity::NotExercised => {
                outcome.passed = false;
                outcome.final_consensus = false;
                outcome.violations.push(
                    "not exercised: no code path in this engine runs this scenario against a node"
                        .into(),
                );
                outcome.notes = format!(
                    "DOCUMENTED EXPECTATION, NOT VERIFIED: {} (invariants a real test would need to check: {})",
                    outcome.notes, would_check
                );
            }
        }
        outcome
    }

    fn run_scenario_model(&mut self, scenario: ChaosScenario, height: u64) -> ChaosOutcome {
        let start = Instant::now();
        match scenario {
            ChaosScenario::ValidatorCrash { count } => {
                let max_safe =
                    (self.validator_count as f64 * self.config.max_crash_fraction) as usize;
                if count > max_safe {
                    ChaosOutcome::fail(
                        ChaosScenario::ValidatorCrash { count },
                        start,
                        height,
                        "crash count exceeds BFT safety bound (f < n/3)",
                    )
                } else {
                    // Simulate: crash `count` validators, wait recovery_window, verify consensus resumes.
                    let remaining = self.validator_count - count;
                    let quorum = (self.validator_count * 2 / 3) + 1;
                    if remaining >= quorum {
                        ChaosOutcome::pass(ChaosScenario::ValidatorCrash { count }, start, height, height + 5,
                            &format!("{} validators crashed; {} remaining ≥ quorum {}; consensus resumed in recovery window", count, remaining, quorum))
                    } else {
                        ChaosOutcome::fail(
                            ChaosScenario::ValidatorCrash { count },
                            start,
                            height,
                            "remaining validators below 2/3 quorum",
                        )
                    }
                }
            }
            ChaosScenario::NetworkPartition { group_a, group_b } => {
                let minority = group_a.min(group_b);
                let majority = group_a.max(group_b);
                let quorum = (self.validator_count * 2 / 3) + 1;
                if majority >= quorum {
                    // Majority partition continues producing blocks; minority stalls.
                    // On heal, minority reorgs to canonical chain.
                    ChaosOutcome::pass(ChaosScenario::NetworkPartition { group_a, group_b }, start, height, height + self.config.min_blocks_under_partition + 2,
                        &format!("partition {}/{} — majority partition ({}) met quorum ({}); {} stalled; healed cleanly", group_a, group_b, majority, quorum, minority))
                } else {
                    ChaosOutcome::fail(ChaosScenario::NetworkPartition { group_a, group_b }, start, height, "neither partition reaches 2/3 quorum — liveness halted (expected under symmetric partition)")
                }
            }
            ChaosScenario::LongRangeReorg { fork_depth } => {
                // A finalised block at height (current - fork_depth) cannot be reorganised.
                // The attacker would need >2/3 stake retrospectively — impossible without breaking safety.
                if fork_depth > 6 {
                    ChaosOutcome::pass(ChaosScenario::LongRangeReorg { fork_depth }, start, height, height,
                        &format!("long-range reorg from -{} rejected: finalised blocks are irreversible (FinalityManager invariant I-CON3)", fork_depth))
                } else {
                    // Within the unfinalized window — reorg possible if attacker has >2/3 stake.
                    ChaosOutcome::pass(ChaosScenario::LongRangeReorg { fork_depth }, start, height, height,
                        &format!("short reorg from -{} blocks: within unfinalized window, canonical chain selection by cumulative stake", fork_depth))
                }
            }
            ChaosScenario::DoubleSign { validator_id } => {
                // SlashingEngine detects equivocation; 33% stake burned; evidence committed on-chain.
                let msg = format!("double-sign by {} detected: SlashingEngine applied 33% penalty, evidence committed on-chain, validator tombstoned (idempotent, invariant I-C3)", validator_id);
                ChaosOutcome::pass(
                    ChaosScenario::DoubleSign { validator_id },
                    start,
                    height,
                    height + 1,
                    &msg,
                )
            }
            ChaosScenario::TxReplay { tx_id } => {
                // Nonce monotonicity invariant (I-S5) rejects replayed transactions.
                let msg = format!("tx {} replay rejected by nonce check (I-S5: nonce must be current_nonce + 1); mempool dedup also blocks broadcast", tx_id);
                ChaosOutcome::pass(
                    ChaosScenario::TxReplay { tx_id },
                    start,
                    height,
                    height,
                    &msg,
                )
            }
            ChaosScenario::EclipseAttack { target_validator } => {
                // Kademlia k=20 prevents full eclipse; validator maintains ≥1 honest peer from seed list.
                let msg = format!("eclipse of {} mitigated: Kademlia k=20 routing diversity; DNS seeds provide re-entry; onion routing prevents IP mapping", target_validator);
                ChaosOutcome::pass(
                    ChaosScenario::EclipseAttack { target_validator },
                    start,
                    height,
                    height,
                    &msg,
                )
            }
            ChaosScenario::InvalidBlockFlood { count } => {
                // SPHINCS+ signature validation rejects all unsigned/malformed blocks in O(1) per block.
                ChaosOutcome::pass(ChaosScenario::InvalidBlockFlood { count }, start, height, height,
                    &format!("{} invalid blocks: all rejected at SPHINCS+ signature gate before state application; no chain disruption; rate-limit triggered after 100 rejected blocks from same peer", count))
            }
            ChaosScenario::LoadStress { tps, duration_secs } => {
                // Capacity arithmetic only. No transaction is sent and no
                // throughput is measured.
                const MAX_TXS_PER_BLOCK: u64 = 4096;
                const BLOCK_TIME_SECS: u64 = 3;
                let ceiling_tps = MAX_TXS_PER_BLOCK / BLOCK_TIME_SECS;
                let target_tps = tps as u64;
                let expected_blocks = duration_secs / BLOCK_TIME_SECS;
                let txs_per_block = (target_tps * BLOCK_TIME_SECS).min(MAX_TXS_PER_BLOCK);
                let effective_tps = txs_per_block / BLOCK_TIME_SECS;
                if effective_tps >= target_tps {
                    ChaosOutcome::pass(
                        ChaosScenario::LoadStress { tps, duration_secs },
                        start,
                        height,
                        height + expected_blocks,
                        &format!(
                            "{} TPS target is within the block-capacity ceiling of {} TPS \
                             ({} tx/block over {}s blocks); capacity arithmetic only, no \
                             transactions were sent",
                            tps, ceiling_tps, txs_per_block, BLOCK_TIME_SECS
                        ),
                    )
                } else {
                    ChaosOutcome::fail(
                        ChaosScenario::LoadStress { tps, duration_secs },
                        start,
                        height,
                        &format!(
                            "TPS target {} exceeds the block-capacity ceiling of {} TPS \
                             ({} tx/block over {}s blocks)",
                            tps, ceiling_tps, MAX_TXS_PER_BLOCK, BLOCK_TIME_SECS
                        ),
                    )
                }
            }
        }
    }

    pub fn outcomes(&self) -> &[ChaosOutcome] {
        &self.outcomes
    }

    pub fn summary(&self) -> ChaosSummary {
        let total = self.outcomes.len();
        let not_exercised = self
            .outcomes
            .iter()
            .filter(|o| o.fidelity() == ChaosFidelity::NotExercised)
            .count();
        let modelled = total - not_exercised;
        let passed = self
            .outcomes
            .iter()
            .filter(|o| o.fidelity() == ChaosFidelity::Analytical && o.passed)
            .count();
        ChaosSummary {
            total,
            modelled,
            not_exercised,
            passed,
            failed: modelled - passed,
            all_passed: not_exercised == 0 && passed == modelled,
            pass_rate_pct: if modelled == 0 {
                0.0
            } else {
                (passed as f64 / modelled as f64) * 100.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChaosSummary {
    /// Every scenario in the run.
    pub total: usize,
    /// Scenarios with an analytical model (see [`ChaosFidelity::Analytical`]).
    pub modelled: usize,
    /// Scenarios with no executed code path; never counted as passed.
    pub not_exercised: usize,
    /// Modelled scenarios whose model assertion held.
    pub passed: usize,
    /// Modelled scenarios whose model assertion failed.
    pub failed: usize,
    /// True only if nothing was left unexercised and every model held.
    pub all_passed: bool,
    /// `passed / modelled`; excludes `not_exercised` scenarios.
    pub pass_rate_pct: f64,
}

// ── Continuous harness (bookkeeping only) ────────────────────────────────────
//
// `tick` re-evaluates the in-memory model instantly. It does not run for
// `target_hours`, does not start nodes, and is not a soak test.

pub struct ContinuousChaosHarness {
    engine: ChaosEngine,
    target_hours: u64,
    iteration: u64,
    total_passed: u64,
    total_failed: u64,
}

impl ContinuousChaosHarness {
    pub fn new(validator_count: usize, target_hours: u64) -> Self {
        Self {
            engine: ChaosEngine::new(validator_count),
            target_hours,
            iteration: 0,
            total_passed: 0,
            total_failed: 0,
        }
    }

    /// Run one iteration of the full suite; returns false if any scenario failed.
    pub fn tick(&mut self, current_height: u64) -> bool {
        self.iteration += 1;
        let passed = self.engine.run_full_suite(current_height);
        let summary = self.engine.summary();
        self.total_passed += summary.passed as u64;
        self.total_failed += summary.failed as u64;
        passed
    }

    pub fn iterations(&self) -> u64 {
        self.iteration
    }
    pub fn total_passed(&self) -> u64 {
        self.total_passed
    }
    pub fn total_failed(&self) -> u64 {
        self.total_failed
    }
    pub fn target_hours(&self) -> u64 {
        self.target_hours
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_crash_within_bft_bound_passes() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(ChaosScenario::ValidatorCrash { count: 2 }, 100);
        assert!(outcome.passed, "crash of 2/7 should be within BFT bound");
    }

    #[test]
    fn validator_crash_exceeds_bft_bound_fails() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(ChaosScenario::ValidatorCrash { count: 3 }, 100);
        assert!(!outcome.passed, "crash of 3/7 violates f < n/3");
    }

    #[test]
    fn double_sign_is_reported_as_not_exercised() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(
            ChaosScenario::DoubleSign {
                validator_id: "validator-0".into(),
            },
            200,
        );
        // No code path attacks a node, so this must not be reported as passed.
        assert!(!outcome.passed);
        assert_eq!(outcome.fidelity(), ChaosFidelity::NotExercised);
        assert!(outcome.notes.contains("NOT VERIFIED"));
        assert!(outcome.notes.contains("33%"));
        assert!(outcome.notes.contains("I-CON1"));
        assert!(outcome.invariants_held.is_empty());
        assert!(!outcome.violations.is_empty());
    }

    #[test]
    fn tx_replay_is_reported_as_not_exercised() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(
            ChaosScenario::TxReplay {
                tx_id: "test-tx".into(),
            },
            300,
        );
        assert!(!outcome.passed);
        assert_eq!(outcome.fidelity(), ChaosFidelity::NotExercised);
        assert!(outcome.notes.contains("I-S5"));
    }

    #[test]
    fn long_range_reorg_is_reported_as_not_exercised() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(ChaosScenario::LongRangeReorg { fork_depth: 100 }, 500);
        assert!(!outcome.passed);
        assert_eq!(outcome.fidelity(), ChaosFidelity::NotExercised);
        assert!(outcome.notes.contains("I-CON3"));
    }

    #[test]
    fn load_stress_above_block_capacity_ceiling_fails() {
        // 4096 tx per 3s block is a ceiling of 1365 TPS; 10,000 TPS cannot be met.
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(
            ChaosScenario::LoadStress {
                tps: 10_000,
                duration_secs: 60,
            },
            1000,
        );
        assert!(
            !outcome.passed,
            "10,000 TPS exceeds the block-capacity ceiling"
        );
        assert!(outcome.violations.iter().any(|v| v.contains("1365")));
    }

    #[test]
    fn load_stress_within_ceiling_is_labelled_arithmetic_only() {
        let mut engine = ChaosEngine::new(7);
        let outcome = engine.run_scenario(
            ChaosScenario::LoadStress {
                tps: 1_000,
                duration_secs: 60,
            },
            1000,
        );
        assert!(outcome.passed);
        assert_eq!(outcome.fidelity(), ChaosFidelity::Analytical);
        assert!(outcome.notes.contains("MODEL ONLY"));
        assert!(outcome.notes.contains("no transactions were sent"));
        assert!(outcome.invariants_held.is_empty());
    }

    #[test]
    fn full_chaos_suite_7_validators() {
        let mut engine = ChaosEngine::new(7);
        let all_passed = engine.run_full_suite(5000);
        let summary = engine.summary();
        assert_eq!(summary.total, 14);
        assert_eq!(summary.modelled, 7);
        assert_eq!(summary.not_exercised, 7);
        // A 4/3 partition leaves no 5-of-7 quorum, so one model assertion fails.
        assert_eq!(summary.passed, 6);
        assert_eq!(summary.failed, 1);
        // Unexercised scenarios must prevent a clean bill of health.
        assert!(!summary.all_passed);
        assert_eq!(all_passed, summary.all_passed);
        assert!(summary.pass_rate_pct >= 80.0);
    }

    #[test]
    fn default_suite_contains_no_unreachable_load_target() {
        let engine = ChaosEngine::new(7);
        for scenario in engine.build_scenario_list() {
            if let ChaosScenario::LoadStress { tps, .. } = scenario {
                assert!(tps <= 1365, "{tps} TPS is above the capacity ceiling");
            }
        }
    }

    #[test]
    fn continuous_harness_tick_increments_iteration() {
        let mut harness = ContinuousChaosHarness::new(7, 72);
        harness.tick(1000);
        assert_eq!(harness.iterations(), 1);
    }
}
