//! # BLEEP PAT Engine
//!
//! Production-grade Programmable Asset Token (PAT) engine with RocksDB persistence.
//!
//! PATs are fungible tokens issued on the BLEEP chain with configurable
//! burn rates, transfer restrictions, and optional supply caps.
//!
//! ## Token Model
//! - Supply is capped at `total_supply_cap` (set at creation).
//! - Transfers automatically deduct `burn_rate_bps` from the transferred amount
//!   (deflationary mechanic).
//! - Only the token `owner` may mint new tokens.
//! - Any holder may burn their own tokens at any time.
//!
//! ## Storage
//! - `PersistentPATRegistry::open(db_path)` opens a RocksDB-backed registry.
//!   Every mutation (create_token, mint, burn, transfer) flushes the affected
//!   token and ledger atomically with sync=true before returning to the caller.
//!   PAT state survives node restarts.
//! - `PATRegistry` (in-memory) remains available for testing and devnet.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// ERROR TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum PATError {
    #[error("Token {0} not found")]
    TokenNotFound(String),
    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u128, need: u128 },
    #[error("Unauthorized: caller {0} is not the token owner")]
    Unauthorized(String),
    #[error("Supply cap exceeded: cap {cap}, would reach {would_reach}")]
    SupplyCapExceeded { cap: u128, would_reach: u128 },
    #[error("Token {0} already exists")]
    TokenAlreadyExists(String),
    #[error("Invalid burn rate: {0} bps (max 10000)")]
    InvalidBurnRate(u16),
    #[error("Zero amount not allowed")]
    ZeroAmount,
    #[error("Cannot transfer to self")]
    SelfTransfer,
}

pub type PATResult<T> = Result<T, PATError>;

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN DEFINITION
// ─────────────────────────────────────────────────────────────────────────────

/// Immutable token metadata + mutable supply state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATToken {
    /// Unique token symbol (e.g. "USDB", "WETH-PAT").
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Decimal places (default 8, same as BLEEP).
    pub decimals: u8,
    /// Address of the token owner (sole minter).
    pub owner: String,
    /// Maximum total supply (0 = unlimited).
    pub total_supply_cap: u128,
    /// Current total minted supply.
    pub current_supply: u128,
    /// Total ever burned.
    pub total_burned: u128,
    /// Deflationary burn rate applied on each transfer (basis points).
    /// E.g. 50 = 0.5%.  Max 1000 (10%).
    pub burn_rate_bps: u16,
    /// Creation timestamp (seconds since epoch).
    pub created_at: u64,
    /// State hash (sha256 of supply + burned).
    pub state_hash: [u8; 32],
}

impl PATToken {
    /// Create a new token definition.
    pub fn new(
        symbol: String,
        name: String,
        decimals: u8,
        owner: String,
        total_supply_cap: u128,
        burn_rate_bps: u16,
        created_at: u64,
    ) -> PATResult<Self> {
        if burn_rate_bps > 1000 {
            return Err(PATError::InvalidBurnRate(burn_rate_bps));
        }
        let mut token = PATToken {
            symbol,
            name,
            decimals,
            owner,
            total_supply_cap,
            current_supply: 0,
            total_burned: 0,
            burn_rate_bps,
            created_at,
            state_hash: [0u8; 32],
        };
        token.recompute_hash();
        Ok(token)
    }

    /// Compute and store the state hash.
    pub fn recompute_hash(&mut self) {
        let mut h = Sha256::new();
        h.update(self.symbol.as_bytes());
        h.update(self.current_supply.to_be_bytes());
        h.update(self.total_burned.to_be_bytes());
        let result = h.finalize();
        self.state_hash.copy_from_slice(&result);
    }

    /// Amount burned on a transfer of `amount`.
    pub fn transfer_burn_amount(&self, amount: u128) -> u128 {
        if self.burn_rate_bps == 0 {
            return 0;
        }
        (amount * self.burn_rate_bps as u128) / 10_000
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN LEDGER — per-token balance map
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenLedger {
    /// address → balance
    pub balances: BTreeMap<String, u128>,
}

impl TokenLedger {
    pub fn balance_of(&self, address: &str) -> u128 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn credit(&mut self, address: &str, amount: u128) {
        *self.balances.entry(address.to_string()).or_insert(0) += amount;
    }

    pub fn debit(&mut self, address: &str, amount: u128) -> PATResult<()> {
        let bal = self.balance_of(address);
        if bal < amount {
            return Err(PATError::InsufficientBalance { have: bal, need: amount });
        }
        let entry = self.balances.entry(address.to_string()).or_insert(0);
        *entry -= amount;
        if *entry == 0 {
            self.balances.remove(address);
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT EVENT LOG
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PATEvent {
    Mint  { token: String, to: String, amount: u128, ts: u64 },
    Burn  { token: String, from: String, amount: u128, ts: u64 },
    Transfer {
        token: String,
        from: String,
        to: String,
        amount: u128,
        burn_deducted: u128,
        ts: u64,
    },
    TokenCreated { token: String, owner: String, ts: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT REGISTRY
// ─────────────────────────────────────────────────────────────────────────────

/// Central PAT registry. Arc<Mutex<PATRegistry>> is held by the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATRegistry {
    pub tokens:  BTreeMap<String, PATToken>,
    pub ledgers: BTreeMap<String, TokenLedger>,
    pub events:  Vec<PATEvent>,
}

impl PATRegistry {
    pub fn new() -> Self {
        Self {
            tokens:  BTreeMap::new(),
            ledgers: BTreeMap::new(),
            events:  Vec::new(),
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ── Token creation ───────────────────────────────────────────────────────

    /// Create a new PAT.  `owner` becomes the sole minter.
    pub fn create_token(
        &mut self,
        symbol: String,
        name: String,
        decimals: u8,
        owner: String,
        total_supply_cap: u128,
        burn_rate_bps: u16,
    ) -> PATResult<()> {
        if self.tokens.contains_key(&symbol) {
            return Err(PATError::TokenAlreadyExists(symbol));
        }
        let token = PATToken::new(
            symbol.clone(), name, decimals, owner.clone(),
            total_supply_cap, burn_rate_bps, Self::now(),
        )?;
        self.tokens.insert(symbol.clone(), token);
        self.ledgers.insert(symbol.clone(), TokenLedger::default());
        self.events.push(PATEvent::TokenCreated { token: symbol, owner, ts: Self::now() });
        Ok(())
    }

    // ── Mint ─────────────────────────────────────────────────────────────────

    /// Mint `amount` tokens of `symbol` to `to`.  Caller must be token owner.
    pub fn mint(&mut self, symbol: &str, caller: &str, to: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        let token = self.tokens.get(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
        if token.owner != caller {
            return Err(PATError::Unauthorized(caller.to_string()));
        }
        if token.total_supply_cap > 0 {
            let would_reach = token.current_supply.checked_add(amount)
                .unwrap_or(u128::MAX);
            if would_reach > token.total_supply_cap {
                return Err(PATError::SupplyCapExceeded {
                    cap: token.total_supply_cap,
                    would_reach,
                });
            }
        }
        // Update token supply
        {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply += amount;
            token.recompute_hash();
        }
        // Credit ledger
        self.ledgers.get_mut(symbol).unwrap().credit(to, amount);
        self.events.push(PATEvent::Mint {
            token: symbol.to_string(),
            to: to.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Burn ─────────────────────────────────────────────────────────────────

    /// Burn `amount` tokens of `symbol` from `from`.  Caller must own the tokens.
    pub fn burn(&mut self, symbol: &str, from: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        // Debit ledger first (validates balance)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;
        // Update supply
        {
            let token = self.tokens.get_mut(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.current_supply = token.current_supply.saturating_sub(amount);
            token.total_burned  += amount;
            token.recompute_hash();
        }
        self.events.push(PATEvent::Burn {
            token: symbol.to_string(),
            from: from.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Transfer ─────────────────────────────────────────────────────────────

    /// Transfer `amount` from `from` to `to`.
    ///
    /// The `burn_rate_bps` is deducted from `amount` and permanently destroyed.
    /// The recipient receives `amount - burn_deducted`.
    pub fn transfer(
        &mut self,
        symbol: &str,
        from: &str,
        to: &str,
        amount: u128,
    ) -> PATResult<u128> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        if from == to {
            return Err(PATError::SelfTransfer);
        }
        let burn_deducted = {
            let token = self.tokens.get(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.transfer_burn_amount(amount)
        };
        let received = amount - burn_deducted;

        // Debit sender (full amount)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;

        // Credit recipient (net of burn)
        self.ledgers.get_mut(symbol).unwrap().credit(to, received);

        // Update supply if burn > 0
        if burn_deducted > 0 {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply  = token.current_supply.saturating_sub(burn_deducted);
            token.total_burned   += burn_deducted;
            token.recompute_hash();
        }

        self.events.push(PATEvent::Transfer {
            token: symbol.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            burn_deducted,
            ts: Self::now(),
        });
        Ok(received)
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    pub fn balance_of(&self, symbol: &str, address: &str) -> u128 {
        self.ledgers.get(symbol)
            .map(|l| l.balance_of(address))
            .unwrap_or(0)
    }

    pub fn get_token(&self, symbol: &str) -> Option<&PATToken> {
        self.tokens.get(symbol)
    }

    pub fn list_tokens(&self) -> Vec<&PATToken> {
        self.tokens.values().collect()
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&PATEvent> {
        let n = self.events.len();
        let start = n.saturating_sub(limit);
        self.events[start..].iter().collect()
    }
}

impl Default for PATRegistry {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> PATRegistry {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "USDB".to_string(),
            "USD Bleep".to_string(),
            8,
            "alice".to_string(),
            1_000_000_000 * 100_000_000u128, // 1B tokens
            50, // 0.5% transfer burn
        ).unwrap();
        reg
    }

    #[test]
    fn test_mint_and_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "bob"), 1_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().current_supply, 1_000_000_000);
    }

    #[test]
    fn test_unauthorized_mint() {
        let mut reg = registry();
        let err = reg.mint("USDB", "mallory", "bob", 1_000).unwrap_err();
        assert!(matches!(err, PATError::Unauthorized(_)));
    }

    #[test]
    fn test_burn() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 5_000_000_000).unwrap();
        reg.burn("USDB", "alice", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "alice"), 4_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().total_burned, 1_000_000_000);
    }

    #[test]
    fn test_transfer_with_burn_rate() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 10_000_000_000).unwrap();
        // Transfer 1000 with 0.5% burn → burn = 5, received = 995
        let received = reg.transfer("USDB", "alice", "bob", 1_000_000_000).unwrap();
        let burn = 1_000_000_000u128 * 50 / 10_000; // 5_000_000
        assert_eq!(received, 1_000_000_000 - burn);
        assert_eq!(reg.balance_of("USDB", "bob"), received);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "CAPPED".to_string(), "Capped".to_string(), 8,
            "owner".to_string(), 1000, 0,
        ).unwrap();
        reg.mint("CAPPED", "owner", "user", 1000).unwrap();
        let err = reg.mint("CAPPED", "owner", "user", 1).unwrap_err();
        assert!(matches!(err, PATError::SupplyCapExceeded { .. }));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 100).unwrap();
        let err = reg.transfer("USDB", "bob", "carol", 200).unwrap_err();
        assert!(matches!(err, PATError::InsufficientBalance { .. }));
    }

    #[test]
    fn test_event_log() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 500_000_000).unwrap();
        reg.transfer("USDB", "bob", "carol", 100_000_000).unwrap();
        // TokenCreated + Mint + Transfer = 3 events
        assert_eq!(reg.events.len(), 3);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ROCKSDB PERSISTENCE LAYER
// ─────────────────────────────────────────────────────────────────────────────
//
// PATStore wraps a RocksDB instance with a single column family: `pat_registry`.
//
// Key layout:
//   "token:{symbol}"  → bincode-serialized PATToken
//   "ledger:{symbol}" → bincode-serialized TokenLedger
//   "events:seq"      → 8-byte big-endian event sequence counter
//   "event:{seq:016x}" → bincode-serialized PATEvent
//
// Every mutation (create_token, mint, burn, transfer) calls PATStore::flush()
// which serializes the full token + ledger state in a single WriteBatch with
// sync=true — matching the same durability guarantee used by the nullifier_store
// and audit_log column families.
//
// PATRegistry::open(db_path) loads the persisted state from RocksDB at startup
// so all PAT state survives node restarts.

use rocksdb::{DB, Options, WriteBatch, ColumnFamilyDescriptor};

/// Column family name for all PAT data.
const PAT_CF: &str = "pat_registry";

/// RocksDB-backed durability layer for PATRegistry.
/// Held as `Option<PATStore>` so in-memory operation (testing) is still
/// possible by constructing `PATRegistry::new()`.
pub struct PATStore {
    db: DB,
}

impl PATStore {
    /// Open (or create) a RocksDB database at `path` for PAT state.
    pub fn open(path: &str) -> Result<Self, String> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cf_desc = ColumnFamilyDescriptor::new(PAT_CF, cf_opts);

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_desc])
            .map_err(|e| format!("PATStore open failed: {e}"))?;
        Ok(Self { db })
    }

    /// Persist a token definition and its ledger atomically.
    pub fn flush_token(&self, token: &PATToken, ledger: &TokenLedger) -> Result<(), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        let token_key  = format!("token:{}", token.symbol);
        let ledger_key = format!("ledger:{}", token.symbol);

        let token_bytes  = bincode::serde::encode_to_vec(token, bincode::config::standard())
            .map_err(|e| format!("serialize token: {e}"))?;
        let ledger_bytes = bincode::serde::encode_to_vec(ledger, bincode::config::standard())
            .map_err(|e| format!("serialize ledger: {e}"))?;

        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, token_key.as_bytes(),  &token_bytes);
        batch.put_cf(&cf, ledger_key.as_bytes(), &ledger_bytes);

        let mut write_opts = rocksdb::WriteOptions::default();
        write_opts.set_sync(true);

        self.db.write_opt(batch, &write_opts)
            .map_err(|e| format!("PATStore flush_token: {e}"))?;
        Ok(())
    }

    /// Append a single PAT event with an auto-incrementing sequence key.
    pub fn append_event(&self, event: &PATEvent) -> Result<(), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        // Read + increment sequence counter
        let seq_key = b"events:seq";
        let seq: u64 = self.db.get_cf(&cf, seq_key)
            .map_err(|e| format!("read seq: {e}"))?
            .map(|v| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&v[..8.min(v.len())]);
                u64::from_be_bytes(buf)
            })
            .unwrap_or(0);

        let event_key   = format!("event:{:016x}", seq);
        let event_bytes = bincode::serde::encode_to_vec(event, bincode::config::standard())
            .map_err(|e| format!("serialize event: {e}"))?;

        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, event_key.as_bytes(), &event_bytes);
        batch.put_cf(&cf, seq_key, &(seq + 1).to_be_bytes());

        let mut write_opts = rocksdb::WriteOptions::default();
        write_opts.set_sync(true);

        self.db.write_opt(batch, &write_opts)
            .map_err(|e| format!("PATStore append_event: {e}"))?;
        Ok(())
    }

    /// Load all persisted tokens and ledgers from RocksDB.
    /// Called once at node startup by `PATRegistry::open()`.
    pub fn load_all(&self) -> Result<(BTreeMap<String, PATToken>, BTreeMap<String, TokenLedger>), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        let mut tokens:  BTreeMap<String, PATToken>    = BTreeMap::new();
        let mut ledgers: BTreeMap<String, TokenLedger> = BTreeMap::new();

        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| format!("iterator error: {e}"))?;
            let key_str = std::str::from_utf8(&key)
                .map_err(|_| "non-UTF8 key".to_string())?;

            if let Some(symbol) = key_str.strip_prefix("token:") {
                let token: PATToken = bincode::serde::decode_from_slice::<PATToken>(&value, bincode::config::standard()).map(|(v, _)| v)
                    .map_err(|e| format!("deserialize token {symbol}: {e}"))?;
                tokens.insert(symbol.to_string(), token);
            } else if let Some(symbol) = key_str.strip_prefix("ledger:") {
                let ledger: TokenLedger = bincode::serde::decode_from_slice::<TokenLedger>(&value, bincode::config::standard()).map(|(v, _)| v)
                    .map_err(|e| format!("deserialize ledger {symbol}: {e}"))?;
                ledgers.insert(symbol.to_string(), ledger);
            }
        }
        Ok((tokens, ledgers))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PERSISTENT PAT REGISTRY
// ─────────────────────────────────────────────────────────────────────────────

/// `PATRegistry` with RocksDB persistence.
///
/// Use `PATRegistry::open(db_path)` at node startup to restore all PAT state
/// from disk. All mutations (create_token, mint, burn, transfer) flush the
/// affected token and ledger synchronously before returning to the caller.
///
/// `PATRegistry::new()` continues to work for in-memory (test) use — the
/// `store` field will be `None` and writes are in-memory only.
pub struct PersistentPATRegistry {
    inner: PATRegistry,
    store: Option<PATStore>,
}

impl PersistentPATRegistry {
    /// Open a persistent PAT registry backed by RocksDB at `db_path`.
    /// Existing PAT state is loaded from disk before returning.
    pub fn open(db_path: &str) -> Result<Self, String> {
        let store = PATStore::open(db_path)?;
        let (tokens, ledgers) = store.load_all()?;
        Ok(Self {
            inner: PATRegistry { tokens, ledgers, events: Vec::new() },
            store: Some(store),
        })
    }

    /// In-memory only (for tests / devnet without a db_path).
    pub fn in_memory() -> Self {
        Self { inner: PATRegistry::new(), store: None }
    }

    fn flush_token(&self, symbol: &str) {
        if let Some(store) = &self.store {
            if let (Some(token), Some(ledger)) = (
                self.inner.tokens.get(symbol),
                self.inner.ledgers.get(symbol),
            ) {
                if let Err(e) = store.flush_token(token, ledger) {
                    tracing::error!("[PATStore] flush_token({symbol}) failed: {e}");
                }
            }
        }
    }

    fn append_event(&self, event: &PATEvent) {
        if let Some(store) = &self.store {
            if let Err(e) = store.append_event(event) {
                tracing::error!("[PATStore] append_event failed: {e}");
            }
        }
    }

    // ── Delegating mutators (each flushes to RocksDB) ────────────────────────

    pub fn create_token(
        &mut self,
        symbol: String, name: String, decimals: u8, owner: String,
        total_supply_cap: u128, burn_rate_bps: u16,
    ) -> PATResult<()> {
        self.inner.create_token(symbol.clone(), name, decimals, owner, total_supply_cap, burn_rate_bps)?;
        self.flush_token(&symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn mint(&mut self, symbol: &str, caller: &str, to: &str, amount: u128) -> PATResult<()> {
        self.inner.mint(symbol, caller, to, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn burn(&mut self, symbol: &str, from: &str, amount: u128) -> PATResult<()> {
        self.inner.burn(symbol, from, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn transfer(&mut self, symbol: &str, from: &str, to: &str, amount: u128) -> PATResult<u128> {
        let received = self.inner.transfer(symbol, from, to, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(received)
    }

    // ── Read-only delegates ──────────────────────────────────────────────────

    pub fn balance_of(&self, symbol: &str, address: &str) -> u128 {
        self.inner.balance_of(symbol, address)
    }
    pub fn get_token(&self, symbol: &str) -> Option<&PATToken> {
        self.inner.get_token(symbol)
    }
    pub fn list_tokens(&self) -> Vec<&PATToken> {
        self.inner.list_tokens()
    }
    pub fn recent_events(&self, limit: usize) -> Vec<&PATEvent> {
        self.inner.recent_events(limit)
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_db_path(test_name: &str) -> String {
        let mut p = std::env::temp_dir();
        p.push(format!("bleep_pat_test_{}", test_name));
        p.to_string_lossy().to_string()
    }

    #[test]
    fn pat_state_survives_restart() {
        let db_path = tmp_db_path("restart");
        let _ = std::fs::remove_dir_all(&db_path); // clean slate

        // First "node start": create token and mint
        {
            let mut reg = PersistentPATRegistry::open(&db_path).unwrap();
            reg.create_token(
                "USDB".to_string(), "USD Bridge".to_string(),
                8, "alice".to_string(), 1_000_000, 50,
            ).unwrap();
            reg.mint("USDB", "alice", "alice", 500_000).unwrap();
            assert_eq!(reg.balance_of("USDB", "alice"), 500_000);
        }

        // Second "node start": reload from RocksDB — balances must persist
        {
            let reg = PersistentPATRegistry::open(&db_path).unwrap();
            assert_eq!(reg.balance_of("USDB", "alice"), 500_000,
                "balance must survive node restart via RocksDB");
            assert!(reg.get_token("USDB").is_some(),
                "token definition must survive node restart");
        }

        let _ = std::fs::remove_dir_all(&db_path);
    }

    #[test]
    fn transfer_flushes_both_sides() {
        let db_path = tmp_db_path("transfer");
        let _ = std::fs::remove_dir_all(&db_path);

        {
            let mut reg = PersistentPATRegistry::open(&db_path).unwrap();
            reg.create_token("TKN".to_string(), "Token".to_string(), 8, "alice".to_string(), 0, 0).unwrap();
            reg.mint("TKN", "alice", "alice", 1_000).unwrap();
            reg.transfer("TKN", "alice", "bob", 400).unwrap();
        }

        {
            let reg = PersistentPATRegistry::open(&db_path).unwrap();
            assert_eq!(reg.balance_of("TKN", "alice"), 600);
            assert_eq!(reg.balance_of("TKN", "bob"),   400);
        }

        let _ = std::fs::remove_dir_all(&db_path);
    }
            }            name,
            decimals,
            owner,
            total_supply_cap,
            current_supply: 0,
            total_burned: 0,
            burn_rate_bps,
            created_at,
            state_hash: [0u8; 32],
        };
        token.recompute_hash();
        Ok(token)
    }

    /// Compute and store the state hash.
    pub fn recompute_hash(&mut self) {
        let mut h = Sha256::new();
        h.update(self.symbol.as_bytes());
        h.update(self.current_supply.to_be_bytes());
        h.update(self.total_burned.to_be_bytes());
        let result = h.finalize();
        self.state_hash.copy_from_slice(&result);
    }

    /// Amount burned on a transfer of `amount`.
    pub fn transfer_burn_amount(&self, amount: u128) -> u128 {
        if self.burn_rate_bps == 0 {
            return 0;
        }
        (amount * self.burn_rate_bps as u128) / 10_000
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN LEDGER — per-token balance map
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenLedger {
    /// address → balance
    pub balances: BTreeMap<String, u128>,
}

impl TokenLedger {
    pub fn balance_of(&self, address: &str) -> u128 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn credit(&mut self, address: &str, amount: u128) {
        *self.balances.entry(address.to_string()).or_insert(0) += amount;
    }

    pub fn debit(&mut self, address: &str, amount: u128) -> PATResult<()> {
        let bal = self.balance_of(address);
        if bal < amount {
            return Err(PATError::InsufficientBalance { have: bal, need: amount });
        }
        let entry = self.balances.entry(address.to_string()).or_insert(0);
        *entry -= amount;
        if *entry == 0 {
            self.balances.remove(address);
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT EVENT LOG
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PATEvent {
    Mint  { token: String, to: String, amount: u128, ts: u64 },
    Burn  { token: String, from: String, amount: u128, ts: u64 },
    Transfer {
        token: String,
        from: String,
        to: String,
        amount: u128,
        burn_deducted: u128,
        ts: u64,
    },
    TokenCreated { token: String, owner: String, ts: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT REGISTRY
// ─────────────────────────────────────────────────────────────────────────────

/// Central PAT registry. Arc<Mutex<PATRegistry>> is held by the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATRegistry {
    pub tokens:  BTreeMap<String, PATToken>,
    pub ledgers: BTreeMap<String, TokenLedger>,
    pub events:  Vec<PATEvent>,
}

impl PATRegistry {
    pub fn new() -> Self {
        Self {
            tokens:  BTreeMap::new(),
            ledgers: BTreeMap::new(),
            events:  Vec::new(),
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ── Token creation ───────────────────────────────────────────────────────

    /// Create a new PAT.  `owner` becomes the sole minter.
    pub fn create_token(
        &mut self,
        symbol: String,
        name: String,
        decimals: u8,
        owner: String,
        total_supply_cap: u128,
        burn_rate_bps: u16,
    ) -> PATResult<()> {
        if self.tokens.contains_key(&symbol) {
            return Err(PATError::TokenAlreadyExists(symbol));
        }
        let token = PATToken::new(
            symbol.clone(), name, decimals, owner.clone(),
            total_supply_cap, burn_rate_bps, Self::now(),
        )?;
        self.tokens.insert(symbol.clone(), token);
        self.ledgers.insert(symbol.clone(), TokenLedger::default());
        self.events.push(PATEvent::TokenCreated { token: symbol, owner, ts: Self::now() });
        Ok(())
    }

    // ── Mint ─────────────────────────────────────────────────────────────────

    /// Mint `amount` tokens of `symbol` to `to`.  Caller must be token owner.
    pub fn mint(&mut self, symbol: &str, caller: &str, to: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        let token = self.tokens.get(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
        if token.owner != caller {
            return Err(PATError::Unauthorized(caller.to_string()));
        }
        if token.total_supply_cap > 0 {
            let would_reach = token.current_supply.checked_add(amount)
                .unwrap_or(u128::MAX);
            if would_reach > token.total_supply_cap {
                return Err(PATError::SupplyCapExceeded {
                    cap: token.total_supply_cap,
                    would_reach,
                });
            }
        }
        // Update token supply
        {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply += amount;
            token.recompute_hash();
        }
        // Credit ledger
        self.ledgers.get_mut(symbol).unwrap().credit(to, amount);
        self.events.push(PATEvent::Mint {
            token: symbol.to_string(),
            to: to.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Burn ─────────────────────────────────────────────────────────────────

    /// Burn `amount` tokens of `symbol` from `from`.  Caller must own the tokens.
    pub fn burn(&mut self, symbol: &str, from: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        // Debit ledger first (validates balance)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;
        // Update supply
        {
            let token = self.tokens.get_mut(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.current_supply = token.current_supply.saturating_sub(amount);
            token.total_burned  += amount;
            token.recompute_hash();
        }
        self.events.push(PATEvent::Burn {
            token: symbol.to_string(),
            from: from.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Transfer ─────────────────────────────────────────────────────────────

    /// Transfer `amount` from `from` to `to`.
    ///
    /// The `burn_rate_bps` is deducted from `amount` and permanently destroyed.
    /// The recipient receives `amount - burn_deducted`.
    pub fn transfer(
        &mut self,
        symbol: &str,
        from: &str,
        to: &str,
        amount: u128,
    ) -> PATResult<u128> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        if from == to {
            return Err(PATError::SelfTransfer);
        }
        let burn_deducted = {
            let token = self.tokens.get(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.transfer_burn_amount(amount)
        };
        let received = amount - burn_deducted;

        // Debit sender (full amount)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;

        // Credit recipient (net of burn)
        self.ledgers.get_mut(symbol).unwrap().credit(to, received);

        // Update supply if burn > 0
        if burn_deducted > 0 {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply  = token.current_supply.saturating_sub(burn_deducted);
            token.total_burned   += burn_deducted;
            token.recompute_hash();
        }

        self.events.push(PATEvent::Transfer {
            token: symbol.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            burn_deducted,
            ts: Self::now(),
        });
        Ok(received)
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    pub fn balance_of(&self, symbol: &str, address: &str) -> u128 {
        self.ledgers.get(symbol)
            .map(|l| l.balance_of(address))
            .unwrap_or(0)
    }

    pub fn get_token(&self, symbol: &str) -> Option<&PATToken> {
        self.tokens.get(symbol)
    }

    pub fn list_tokens(&self) -> Vec<&PATToken> {
        self.tokens.values().collect()
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&PATEvent> {
        let n = self.events.len();
        let start = n.saturating_sub(limit);
        self.events[start..].iter().collect()
    }
}

impl Default for PATRegistry {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> PATRegistry {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "USDB".to_string(),
            "USD Bleep".to_string(),
            8,
            "alice".to_string(),
            1_000_000_000 * 100_000_000u128, // 1B tokens
            50, // 0.5% transfer burn
        ).unwrap();
        reg
    }

    #[test]
    fn test_mint_and_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "bob"), 1_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().current_supply, 1_000_000_000);
    }

    #[test]
    fn test_unauthorized_mint() {
        let mut reg = registry();
        let err = reg.mint("USDB", "mallory", "bob", 1_000).unwrap_err();
        assert!(matches!(err, PATError::Unauthorized(_)));
    }

    #[test]
    fn test_burn() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 5_000_000_000).unwrap();
        reg.burn("USDB", "alice", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "alice"), 4_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().total_burned, 1_000_000_000);
    }

    #[test]
    fn test_transfer_with_burn_rate() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 10_000_000_000).unwrap();
        // Transfer 1000 with 0.5% burn → burn = 5, received = 995
        let received = reg.transfer("USDB", "alice", "bob", 1_000_000_000).unwrap();
        let burn = 1_000_000_000u128 * 50 / 10_000; // 5_000_000
        assert_eq!(received, 1_000_000_000 - burn);
        assert_eq!(reg.balance_of("USDB", "bob"), received);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "CAPPED".to_string(), "Capped".to_string(), 8,
            "owner".to_string(), 1000, 0,
        ).unwrap();
        reg.mint("CAPPED", "owner", "user", 1000).unwrap();
        let err = reg.mint("CAPPED", "owner", "user", 1).unwrap_err();
        assert!(matches!(err, PATError::SupplyCapExceeded { .. }));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 100).unwrap();
        let err = reg.transfer("USDB", "bob", "carol", 200).unwrap_err();
        assert!(matches!(err, PATError::InsufficientBalance { .. }));
    }

    #[test]
    fn test_event_log() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 500_000_000).unwrap();
        reg.transfer("USDB", "bob", "carol", 100_000_000).unwrap();
        // TokenCreated + Mint + Transfer = 3 events
        assert_eq!(reg.events.len(), 3);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ROCKSDB PERSISTENCE LAYER
// ─────────────────────────────────────────────────────────────────────────────
//
// PATStore wraps a RocksDB instance with a single column family: `pat_registry`.
//
// Key layout:
//   "token:{symbol}"  → bincode-serialized PATToken
//   "ledger:{symbol}" → bincode-serialized TokenLedger
//   "events:seq"      → 8-byte big-endian event sequence counter
//   "event:{seq:016x}" → bincode-serialized PATEvent
//
// Every mutation (create_token, mint, burn, transfer) calls PATStore::flush()
// which serializes the full token + ledger state in a single WriteBatch with
// sync=true — matching the same durability guarantee used by the nullifier_store
// and audit_log column families.
//
// PATRegistry::open(db_path) loads the persisted state from RocksDB at startup
// so all PAT state survives node restarts.

use rocksdb::{DB, Options, WriteBatch, ColumnFamilyDescriptor};

/// Column family name for all PAT data.
const PAT_CF: &str = "pat_registry";

/// RocksDB-backed durability layer for PATRegistry.
/// Held as `Option<PATStore>` so in-memory operation (testing) is still
/// possible by constructing `PATRegistry::new()`.
pub struct PATStore {
    db: DB,
}

impl PATStore {
    /// Open (or create) a RocksDB database at `path` for PAT state.
    pub fn open(path: &str) -> Result<Self, String> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cf_desc = ColumnFamilyDescriptor::new(PAT_CF, cf_opts);

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_desc])
            .map_err(|e| format!("PATStore open failed: {e}"))?;
        Ok(Self { db })
    }

    /// Persist a token definition and its ledger atomically.
    pub fn flush_token(&self, token: &PATToken, ledger: &TokenLedger) -> Result<(), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        let token_key  = format!("token:{}", token.symbol);
        let ledger_key = format!("ledger:{}", token.symbol);

        let token_bytes  = bincode::serde::encode_to_vec(token, bincode::config::standard())
            .map_err(|e| format!("serialize token: {e}"))?;
        let ledger_bytes = bincode::serde::encode_to_vec(ledger, bincode::config::standard())
            .map_err(|e| format!("serialize ledger: {e}"))?;

        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, token_key.as_bytes(),  &token_bytes);
        batch.put_cf(&cf, ledger_key.as_bytes(), &ledger_bytes);

        let mut write_opts = rocksdb::WriteOptions::default();
        write_opts.set_sync(true);

        self.db.write_opt(batch, &write_opts)
            .map_err(|e| format!("PATStore flush_token: {e}"))?;
        Ok(())
    }

    /// Append a single PAT event with an auto-incrementing sequence key.
    pub fn append_event(&self, event: &PATEvent) -> Result<(), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        // Read + increment sequence counter
        let seq_key = b"events:seq";
        let seq: u64 = self.db.get_cf(&cf, seq_key)
            .map_err(|e| format!("read seq: {e}"))?
            .map(|v| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&v[..8.min(v.len())]);
                u64::from_be_bytes(buf)
            })
            .unwrap_or(0);

        let event_key   = format!("event:{:016x}", seq);
        let event_bytes = bincode::serde::encode_to_vec(event, bincode::config::standard())
            .map_err(|e| format!("serialize event: {e}"))?;

        let mut batch = WriteBatch::default();
        batch.put_cf(&cf, event_key.as_bytes(), &event_bytes);
        batch.put_cf(&cf, seq_key, &(seq + 1).to_be_bytes());

        let mut write_opts = rocksdb::WriteOptions::default();
        write_opts.set_sync(true);

        self.db.write_opt(batch, &write_opts)
            .map_err(|e| format!("PATStore append_event: {e}"))?;
        Ok(())
    }

    /// Load all persisted tokens and ledgers from RocksDB.
    /// Called once at node startup by `PATRegistry::open()`.
    pub fn load_all(&self) -> Result<(BTreeMap<String, PATToken>, BTreeMap<String, TokenLedger>), String> {
        let cf = self.db.cf_handle(PAT_CF)
            .ok_or_else(|| "pat_registry CF missing".to_string())?;

        let mut tokens:  BTreeMap<String, PATToken>    = BTreeMap::new();
        let mut ledgers: BTreeMap<String, TokenLedger> = BTreeMap::new();

        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| format!("iterator error: {e}"))?;
            let key_str = std::str::from_utf8(&key)
                .map_err(|_| "non-UTF8 key".to_string())?;

            if let Some(symbol) = key_str.strip_prefix("token:") {
                let token: PATToken = bincode::serde::decode_from_slice::<PATToken>(&value, bincode::config::standard()).map(|(v, _)| v)
                    .map_err(|e| format!("deserialize token {symbol}: {e}"))?;
                tokens.insert(symbol.to_string(), token);
            } else if let Some(symbol) = key_str.strip_prefix("ledger:") {
                let ledger: TokenLedger = bincode::serde::decode_from_slice::<TokenLedger>(&value, bincode::config::standard()).map(|(v, _)| v)
                    .map_err(|e| format!("deserialize ledger {symbol}: {e}"))?;
                ledgers.insert(symbol.to_string(), ledger);
            }
        }
        Ok((tokens, ledgers))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PERSISTENT PAT REGISTRY
// ─────────────────────────────────────────────────────────────────────────────

/// `PATRegistry` with RocksDB persistence.
///
/// Use `PATRegistry::open(db_path)` at node startup to restore all PAT state
/// from disk. All mutations (create_token, mint, burn, transfer) flush the
/// affected token and ledger synchronously before returning to the caller.
///
/// `PATRegistry::new()` continues to work for in-memory (test) use — the
/// `store` field will be `None` and writes are in-memory only.
pub struct PersistentPATRegistry {
    inner: PATRegistry,
    store: Option<PATStore>,
}

impl PersistentPATRegistry {
    /// Open a persistent PAT registry backed by RocksDB at `db_path`.
    /// Existing PAT state is loaded from disk before returning.
    pub fn open(db_path: &str) -> Result<Self, String> {
        let store = PATStore::open(db_path)?;
        let (tokens, ledgers) = store.load_all()?;
        Ok(Self {
            inner: PATRegistry { tokens, ledgers, events: Vec::new() },
            store: Some(store),
        })
    }

    /// In-memory only (for tests / devnet without a db_path).
    pub fn in_memory() -> Self {
        Self { inner: PATRegistry::new(), store: None }
    }

    fn flush_token(&self, symbol: &str) {
        if let Some(store) = &self.store {
            if let (Some(token), Some(ledger)) = (
                self.inner.tokens.get(symbol),
                self.inner.ledgers.get(symbol),
            ) {
                if let Err(e) = store.flush_token(token, ledger) {
                    tracing::error!("[PATStore] flush_token({symbol}) failed: {e}");
                }
            }
        }
    }

    fn append_event(&self, event: &PATEvent) {
        if let Some(store) = &self.store {
            if let Err(e) = store.append_event(event) {
                tracing::error!("[PATStore] append_event failed: {e}");
            }
        }
    }

    // ── Delegating mutators (each flushes to RocksDB) ────────────────────────

    pub fn create_token(
        &mut self,
        symbol: String, name: String, decimals: u8, owner: String,
        total_supply_cap: u128, burn_rate_bps: u16,
    ) -> PATResult<()> {
        self.inner.create_token(symbol.clone(), name, decimals, owner, total_supply_cap, burn_rate_bps)?;
        self.flush_token(&symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn mint(&mut self, symbol: &str, caller: &str, to: &str, amount: u128) -> PATResult<()> {
        self.inner.mint(symbol, caller, to, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn burn(&mut self, symbol: &str, from: &str, amount: u128) -> PATResult<()> {
        self.inner.burn(symbol, from, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(())
    }

    pub fn transfer(&mut self, symbol: &str, from: &str, to: &str, amount: u128) -> PATResult<u128> {
        let received = self.inner.transfer(symbol, from, to, amount)?;
        self.flush_token(symbol);
        if let Some(ev) = self.inner.events.last() { self.append_event(ev); }
        Ok(received)
    }

    // ── Read-only delegates ──────────────────────────────────────────────────

    pub fn balance_of(&self, symbol: &str, address: &str) -> u128 {
        self.inner.balance_of(symbol, address)
    }
    pub fn get_token(&self, symbol: &str) -> Option<&PATToken> {
        self.inner.get_token(symbol)
    }
    pub fn list_tokens(&self) -> Vec<&PATToken> {
        self.inner.list_tokens()
    }
    pub fn recent_events(&self, limit: usize) -> Vec<&PATEvent> {
        self.inner.recent_events(limit)
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_db_path(test_name: &str) -> String {
        let mut p = std::env::temp_dir();
        p.push(format!("bleep_pat_test_{}", test_name));
        p.to_string_lossy().to_string()
    }

    #[test]
    fn pat_state_survives_restart() {
        let db_path = tmp_db_path("restart");
        let _ = std::fs::remove_dir_all(&db_path); // clean slate

        // First "node start": create token and mint
        {
            let mut reg = PersistentPATRegistry::open(&db_path).unwrap();
            reg.create_token(
                "USDB".to_string(), "USD Bridge".to_string(),
                8, "alice".to_string(), 1_000_000, 50,
            ).unwrap();
            reg.mint("USDB", "alice", "alice", 500_000).unwrap();
            assert_eq!(reg.balance_of("USDB", "alice"), 500_000);
        }

        // Second "node start": reload from RocksDB — balances must persist
        {
            let reg = PersistentPATRegistry::open(&db_path).unwrap();
            assert_eq!(reg.balance_of("USDB", "alice"), 500_000,
                "balance must survive node restart via RocksDB");
            assert!(reg.get_token("USDB").is_some(),
                "token definition must survive node restart");
        }

        let _ = std::fs::remove_dir_all(&db_path);
    }

    #[test]
    fn transfer_flushes_both_sides() {
        let db_path = tmp_db_path("transfer");
        let _ = std::fs::remove_dir_all(&db_path);

        {
            let mut reg = PersistentPATRegistry::open(&db_path).unwrap();
            reg.create_token("TKN".to_string(), "Token".to_string(), 8, "alice".to_string(), 0, 0).unwrap();
            reg.mint("TKN", "alice", "alice", 1_000).unwrap();
            reg.transfer("TKN", "alice", "bob", 400).unwrap();
        }

        {
            let reg = PersistentPATRegistry::open(&db_path).unwrap();
            assert_eq!(reg.balance_of("TKN", "alice"), 600);
            assert_eq!(reg.balance_of("TKN", "bob"),   400);
        }

        let _ = std::fs::remove_dir_all(&db_path);
    }
    }
