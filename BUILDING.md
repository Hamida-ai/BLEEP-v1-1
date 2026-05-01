# Building BLEEP from Source

## Prerequisites

### 1. Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup show          # should report stable channel per rust-toolchain.toml
```

### 2. System Libraries (Ubuntu / Debian)
```bash
sudo apt-get update && sudo apt-get install -y \
    build-essential cmake clang libclang-dev \
    libssl-dev pkg-config \
    librocksdb-dev \
    perl nasm
```

### 3. System Libraries (macOS)
```bash
brew install cmake openssl rocksdb llvm
export OPENSSL_DIR=$(brew --prefix openssl)
export LIBCLANG_PATH=$(brew --prefix llvm)/lib
```

### 4. System Libraries (Windows)
Use WSL2 with the Ubuntu instructions above. Native Windows builds are not supported.

---

## Build

### Check all crates compile
```bash
cargo check --workspace
```

### Full debug build
```bash
cargo build --workspace
```

### Release build (optimised)
```bash
cargo build --workspace --release
```

### Run all tests
```bash
cargo test --workspace
```

### Run linter
```bash
cargo clippy --workspace -- -D warnings
```

### Run formatter check
```bash
cargo fmt --all -- --check
```

---

## Run the Node

### Local devnet (4 validators, Docker required)
```bash
cd devnet
docker compose up -d
```

### Single-node development mode
```bash
cargo run --bin bleep
```

### Executor node (Layer 4 intent market maker)
```bash
cargo run --bin bleep-executor
```

### Admin CLI
```bash
cargo run --bin bleep_admin -- --help
```

### RPC server
```bash
cargo run --bin bleep-rpc
```

---

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `quantum` | ✅ on | SPHINCS+ / Kyber post-quantum crypto |
| `mainnet` | ✅ on | Mainnet genesis parameters |
| `pretestnet` | off | Pre-testnet genesis parameters |
| `ml` | off | Phase 4 AI inference (requires trained models) |

```bash
# Pre-testnet build
cargo build --workspace --no-default-features --features pretestnet,quantum

# Without PQ crypto (faster dev builds)
cargo build --workspace --no-default-features --features mainnet
```

---

## Environment Setup for rocksdb

If `cargo build` fails with `rocksdb` linker errors, try building it from source:

```bash
ROCKSDB_COMPILE=1 cargo build --workspace
```

This takes longer but avoids needing the system `librocksdb-dev`.

---

## Common Build Errors

| Error | Fix |
|-------|-----|
| `cannot find -lrocksdb` | `sudo apt install librocksdb-dev` or set `ROCKSDB_COMPILE=1` |
| `libclang.so not found` | `sudo apt install libclang-dev` |
| `cmake not found` | `sudo apt install cmake` |
| `pkg-config not found` | `sudo apt install pkg-config` |
| `openssl/ssl.h not found` | `sudo apt install libssl-dev` |
| `nasm: command not found` | `sudo apt install nasm` |
