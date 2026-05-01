#!/usr/bin/env bash
# BLEEP CI/CD Setup Script
# This script initializes the CI/CD environment and configures GitHub Actions

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;36m'
NC='\033[0m' # No Color

# Functions
print_header() {
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}${1}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ ${1}${NC}"
}

print_error() {
    echo -e "${RED}✗ ${1}${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ ${1}${NC}"
}

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    local missing=0
    
    # Check Git
    if ! command -v git &> /dev/null; then
        print_error "Git not found"
        missing=1
    else
        print_success "Git installed"
    fi
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        print_error "Rust not found"
        missing=1
    else
        print_success "Rust installed ($(rustc --version))"
    fi
    
    # Check Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found"
        missing=1
    else
        print_success "Cargo installed"
    fi
    
    # Check Docker (optional)
    if ! command -v docker &> /dev/null; then
        print_warning "Docker not found (optional, needed for Docker builds)"
    else
        print_success "Docker installed"
    fi
    
    # Check GitHub CLI (optional)
    if ! command -v gh &> /dev/null; then
        print_warning "GitHub CLI not found (optional, useful for managing actions)"
    else
        print_success "GitHub CLI installed"
    fi
    
    if [ $missing -eq 1 ]; then
        print_error "Missing required tools"
        exit 1
    fi
}

# Install development tools
install_dev_tools() {
    print_header "Installing Development Tools"
    
    echo "Installing cargo-watch..."
    cargo install cargo-watch --quiet || print_warning "cargo-watch install failed"
    
    echo "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin --quiet || print_warning "cargo-tarpaulin install failed"
    
    echo "Installing cargo-audit..."
    cargo install cargo-audit --quiet || print_warning "cargo-audit install failed"
    
    echo "Installing rustfmt..."
    rustup component add rustfmt 2>/dev/null || print_warning "rustfmt install failed"
    
    echo "Installing clippy..."
    rustup component add clippy 2>/dev/null || print_warning "clippy install failed"
    
    print_success "Development tools installed"
}

# Initialize GitHub Actions
init_github_actions() {
    print_header "GitHub Actions Configuration"
    
    if ! command -v gh &> /dev/null; then
        print_warning "GitHub CLI not installed, skipping GitHub setup"
        return
    fi
    
    echo "Checking GitHub repository connection..."
    if gh repo view &>/dev/null; then
        print_success "Connected to GitHub repository"
        
        # List repository secrets
        echo ""
        echo "Current repository secrets:"
        gh secret list --no-limit 2>/dev/null || echo "  (No secrets currently set)"
        
        echo ""
        echo "To configure secrets, run:"
        echo "  gh secret set CARGO_TOKEN --body '<token>'"
        echo "  gh secret set DOCKERHUB_USERNAME --body '<username>'"
        echo "  gh secret set DOCKERHUB_TOKEN --body '<token>'"
        echo "  gh secret set GITGUARDIAN_API_KEY --body '<key>'"
    else
        print_warning "Not connected to GitHub repository"
    fi
}

# Configure Cargo
configure_cargo() {
    print_header "Cargo Configuration"
    
    if [ ! -d "$HOME/.cargo" ]; then
        mkdir -p "$HOME/.cargo"
    fi
    
    # Create cargo config for faster builds
    mkdir -p .cargo
    
    cat > .cargo/config.toml << 'EOF'
[build]
# Use all available CPU cores for parallel compilation
jobs = { val = 0, relative = true }

# Optimize incremental builds
incremental = true

# Reduce binary size in debug mode
split-debuginfo = "packed"

[profile.release]
# Optimize for performance and size
lto = true
codegen-units = 1
strip = true

[profile.dev]
# Faster compilation in dev
opt-level = 1

[profile.test]
opt-level = 1
EOF
    
    print_success "Cargo config created"
}

# Create Makefile targets
verify_makefile() {
    print_header "Verifying Makefile"
    
    if [ -f "Makefile" ]; then
        print_success "Makefile found"
        echo ""
        echo "Available targets:"
        make help 2>/dev/null | head -30 || print_warning "Could not display Makefile targets"
    else
        print_error "Makefile not found"
        exit 1
    fi
}

# Verify Docker setup
verify_docker() {
    print_header "Docker Setup"
    
    if ! command -v docker &> /dev/null; then
        print_warning "Docker not installed (install Docker to use docker targets)"
        return
    fi
    
    if [ ! -f "Dockerfile" ]; then
        print_warning "Dockerfile not found"
        return
    fi
    
    print_success "Dockerfile found"
    
    if [ ! -f "docker-compose.yml" ]; then
        print_warning "docker-compose.yml not found"
        return
    fi
    
    print_success "docker-compose.yml found"
    
    echo "To start multi-node pre-testnet:"
    echo "  make docker-compose-up"
    echo "  make docker-compose-logs"
    echo "  make docker-compose-down"
}

# Run initial tests
run_initial_tests() {
    print_header "Running Initial Tests"
    
    echo "Building project..."
    if cargo build --quiet 2>/dev/null; then
        print_success "Build successful"
    else
        print_error "Build failed"
        return 1
    fi
    
    echo "Running basic tests..."
    if cargo test --lib --quiet 2>/dev/null; then
        print_success "Tests passed"
    else
        print_error "Tests failed"
        return 1
    fi
    
    echo "Checking formatting..."
    if cargo fmt --all -- --check &> /dev/null; then
        print_success "Code formatting OK"
    else
        print_warning "Code formatting issues found (run 'cargo fmt --all')"
    fi
    
    echo "Running clippy..."
    if cargo clippy --quiet --all-targets 2>/dev/null; then
        print_success "Clippy checks passed"
    else
        print_warning "Clippy found some issues"
    fi
}

# Summary
print_summary() {
    print_header "Setup Complete!"
    
    echo "✓ BLEEP CI/CD environment initialized"
    echo ""
    echo "Next steps:"
    echo ""
    echo "1. Configure GitHub Secrets (if using GitHub Actions):"
    echo "   - CARGO_TOKEN: Token for publishing to crates.io"
    echo "   - DOCKERHUB_USERNAME/TOKEN: For Docker Hub"
    echo "   - GITGUARDIAN_API_KEY: For secret scanning"
    echo ""
    echo "2. Run local checks:"
    echo "   make lint       # Format and lint checks"
    echo "   make test       # Run all tests"
    echo "   make coverage   # Generate coverage report"
    echo ""
    echo "3. Build and run:"
    echo "   make build-release     # Optimized build"
    echo "   make docker-compose-up # Start pre-testnet"
    echo ""
    echo "4. For more options:"
    echo "   make help"
    echo ""
    echo "Documentation: docs/CI_CD_PIPELINE.md"
}

# Main execution
main() {
    print_header "BLEEP CI/CD Setup"
    
    check_prerequisites
    install_dev_tools
    configure_cargo
    verify_makefile
    verify_docker
    init_github_actions
    run_initial_tests
    print_summary
}

# Run main function
main
