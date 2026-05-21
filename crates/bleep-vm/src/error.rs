//! Unified, strongly-typed error hierarchy for bleep-vm.

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum VmError {
    // ── Execution ────────────────────────────────────────────────────────────
    #[error("WASM compile failed: {0}")]
    WasmCompile(String),

    #[error("WASM instantiation failed: {0}")]
    WasmInstantiation(String),

    #[error("WASM execution trapped: {0}")]
    WasmTrap(String),

    #[error("Export not found: {name}")]
    ExportNotFound { name: String },

    #[error("Memory access violation at offset {offset:#x} (size {size} bytes)")]
    MemoryViolation { offset: u64, size: u64 },

    #[error("Memory limit exceeded: requested {requested} bytes, limit {limit} bytes")]
    MemoryLimitExceeded { requested: u64, limit: u64 },

    // ── Gas ──────────────────────────────────────────────────────────────────
    #[error("Gas exhausted: used {used}, limit {limit}")]
    GasExhausted { used: u64, limit: u64 },

    #[error("Gas overflow detected")]
    GasOverflow,

    // ── Transactions ─────────────────────────────────────────────────────────
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Signature verification failed")]
    SignatureVerification,

    #[error("Replay attack detected: nonce {nonce} already used by signer {signer}")]
    ReplayAttack { nonce: u64, signer: String },

    // ── State ────────────────────────────────────────────────────────────────
    #[error("State root mismatch — non-deterministic execution detected")]
    NonDeterministic,

    #[error("State commit failed: {0}")]
    StateCommit(String),

    #[error("Invariant violated: {0}")]
    InvariantViolation(String),

    // ── Optimiser ────────────────────────────────────────────────────────────
    #[error("WASM optimisation failed: {0}")]
    OptimisationFailed(String),

    #[error("Contract analysis failed: {0}")]
    AnalysisFailed(String),

    // ── ZK Proofs ────────────────────────────────────────────────────────────
    #[error("ZK proof generation failed: {0}")]
    ZkProofGeneration(String),

    #[error("ZK proof verification failed")]
    ZkProofVerification,

    #[error("Trusted setup invalid: {0}")]
    TrustedSetup(String),

    // ── Sandbox ──────────────────────────────────────────────────────────────
    #[error("Security policy violation: {0}")]
    SecurityViolation(String),

    #[error("Execution timeout after {millis}ms")]
    Timeout { millis: u64 },

    #[error("Forbidden host call: {syscall}")]
    ForbiddenSyscall { syscall: String },

    // ── Cross-chain ──────────────────────────────────────────────────────────
    #[error("Cross-chain ABI decode failed for chain {chain}: {reason}")]
    AbiDecodeFailed { chain: String, reason: String },

    #[error("Unsupported chain: {0}")]
    UnsupportedChain(String),

    // ── Internal ─────────────────────────────────────────────────────────────
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Gas limit exceeded: requested {requested}, limit {limit}")]
    GasLimitExceeded { requested: u64, limit: u64 },

    #[error("Unsupported VM type: {0}")]
    UnsupportedVmType(String),

    #[error("Internal VM error: {0}")]
    Internal(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Lock poisoned")]
    LockPoisoned,
}

pub type VmResult<T> = Result<T, VmError>;

// Convenience conversions
impl From<std::io::Error> for VmError {
    fn from(e: std::io::Error) -> Self {
        VmError::Internal(e.to_string())
    }
}
