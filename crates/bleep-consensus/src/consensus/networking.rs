//! bleep-consensus/src/consensus/networking.rs
//!
//! Consensus-layer networking wrapper around `bleep_core::networking`.
//!
//! ## Network weight
//! BLEEP uses Proof-of-Stake, not Proof-of-Work, so there is no literal
//! "hash rate".  The value returned by `get_network_hashrate` is the total
//! amount of BLEEP staked across all active validators (in whole BLEEP, not
//! microBLEEP).  Callers that relied on the previous hard-coded sentinel value
//! of `1_000_000` should update to call `set_total_stake_micro` at each epoch
//! boundary and treat the returned value as total stake weight.
//!
//! ## Thread safety
//! `total_stake_micro` is an `AtomicU64`, so `get_network_hashrate` and
//! `set_total_stake_micro` are safe to call from any thread without a lock.

use std::sync::atomic::{AtomicU64, Ordering};

use bleep_core::block::Block;
use bleep_core::networking::NetworkingModule as CoreNetworkingModule;

// ── NetworkingModule ──────────────────────────────────────────────────────────

pub struct NetworkingModule {
    pub inner: CoreNetworkingModule,
    /// Total staked BLEEP across all active validators, in microBLEEP.
    ///
    /// Updated at each epoch boundary by the epoch-advance scheduler task
    /// via `set_total_stake_micro`.  Initialised to `0`; callers should treat
    /// a zero return from `get_network_hashrate` as "not yet known" rather than
    /// "no stake".
    total_stake_micro: AtomicU64,
}

impl NetworkingModule {
    /// Create a new `NetworkingModule` with zero initial stake weight.
    pub fn new() -> Self {
        NetworkingModule {
            inner: CoreNetworkingModule::new(),
            total_stake_micro: AtomicU64::new(0),
        }
    }

    // ── Stake weight ──────────────────────────────────────────────────────────

    /// Persist the current total staked BLEEP (in microBLEEP units, 8 decimals).
    ///
    /// Called once per epoch by the `validator_reward_distribution` scheduler
    /// task after it has aggregated `ValidatorIdentity::stake` values across all
    /// active validators.
    ///
    /// ```text
    /// total_micro = sum_over_active_validators(validator.stake)
    /// ```
    pub fn set_total_stake_micro(&self, micro_bleep: u64) {
        self.total_stake_micro.store(micro_bleep, Ordering::Relaxed);
        log::debug!(
            "[NetworkingModule] total_stake updated: {} microBLEEP ({} BLEEP)",
            micro_bleep,
            micro_bleep / 100_000_000
        );
    }

    /// Return the network stake weight as whole BLEEP (microBLEEP ÷ 10⁸).
    ///
    /// This value is used by:
    /// - The PoW difficulty estimator (which interprets it as a proxy for
    ///   network security weight in the hybrid consensus path).
    /// - Telemetry / Prometheus metrics.
    /// - The chaos suite's `LoadStress` scenario calculations.
    ///
    /// Returns `0` until `set_total_stake_micro` has been called at least once.
    pub fn get_network_hashrate(&self) -> u64 {
        self.total_stake_micro.load(Ordering::Relaxed) / 100_000_000
    }

    // ── Block broadcast ───────────────────────────────────────────────────────

    /// Broadcast a block proposal from the current leader.
    ///
    /// Returns `true` if the underlying `CoreNetworkingModule` accepted the
    /// call without error, `false` otherwise.
    pub fn broadcast_proposal(&self, block: &Block, _leader_id: &str) -> bool {
        match self.inner.broadcast_block(block) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("[NetworkingModule] broadcast_proposal failed: {}", e);
                false
            }
        }
    }

    /// Broadcast a finalised block to all connected peers.
    pub fn broadcast_block(&self, block: &Block) -> bool {
        match self.inner.broadcast_block(block) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("[NetworkingModule] broadcast_block failed: {}", e);
                false
            }
        }
    }

    /// Accept a block received from a peer.
    pub fn receive_block(&self, block: Block) -> bool {
        match self.inner.receive_block(block) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("[NetworkingModule] receive_block failed: {}", e);
                false
            }
        }
    }
}

impl Default for NetworkingModule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_hashrate_is_zero() {
        let nm = NetworkingModule::new();
        assert_eq!(nm.get_network_hashrate(), 0,
            "hashrate must be 0 before set_total_stake_micro is called");
    }

    #[test]
    fn set_and_get_round_trips_correctly() {
        let nm = NetworkingModule::new();
        // 70,000,000 BLEEP = 7_000_000_000_000_000 microBLEEP
        let stake_micro: u64 = 70_000_000 * 100_000_000;
        nm.set_total_stake_micro(stake_micro);
        assert_eq!(nm.get_network_hashrate(), 70_000_000);
    }

    #[test]
    fn fractional_micro_bleep_truncates() {
        let nm = NetworkingModule::new();
        // 1.5 BLEEP = 150_000_000 microBLEEP → 1 whole BLEEP
        nm.set_total_stake_micro(150_000_000);
        assert_eq!(nm.get_network_hashrate(), 1);
    }

    #[test]
    fn update_overwrites_previous_value() {
        let nm = NetworkingModule::new();
        nm.set_total_stake_micro(100_000_000);   // 1 BLEEP
        nm.set_total_stake_micro(500_000_000_00); // 500 BLEEP
        assert_eq!(nm.get_network_hashrate(), 500);
    }

    #[test]
    fn pretestnet_participating_validators() {
        // bleep-pretestnet-1: participating validators with stake
        let nm = NetworkingModule::new();
        let per_validator_micro: u64 = 10_000_000 * 100_000_000;
        nm.set_total_stake_micro(per_validator_micro * 7);
        assert_eq!(nm.get_network_hashrate(), 70_000_000);
    }
}
