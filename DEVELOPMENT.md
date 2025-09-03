# Development Environment Setup

This document describes how to set up the development environment for the JIRA MCP Server project.

## Quick Setup

Run the automated setup script to configure everything:

```bash
./setup-dev-env.sh
```

This script will:
- Create a Python virtual environment in `.venv/`
- Install pre-commit hooks and development tools
- Configure security scanning with detect-secrets
- Install Rust development components (rustfmt, clippy, cargo-audit)

## Manual Setup

If you prefer to set up manually:

### 1. Python Virtual Environment

```bash
# Create virtual environment
python3 -m venv .venv

# Activate it
source .venv/bin/activate

# Install development dependencies
pip install -r requirements-dev.txt
```

### 2. Pre-commit Hooks

```bash
# Install pre-commit hooks
pre-commit install

# Run hooks manually
pre-commit run --all-files
```

### 3. Rust Tools

```bash
# Install Rust components
rustup component add rustfmt clippy

# Install security auditing tool
cargo install cargo-audit
```

## Pre-commit Hooks

The following hooks are configured and run automatically on each commit:

### General Code Quality
- **trailing-whitespace**: Removes trailing whitespace
- **end-of-file-fixer**: Ensures files end with a newline
- **check-merge-conflict**: Prevents committing merge conflict markers
- **check-added-large-files**: Prevents committing large files
- **check-yaml/toml/json**: Validates file formats
- **mixed-line-ending**: Enforces LF line endings

### Rust-specific Hooks
- **rustfmt**: Formats Rust code according to standard style
- **clippy**: Lints Rust code for common mistakes and improvements
- **cargo-check**: Ensures code compiles without errors
- **cargo-test**: Runs all tests to ensure they pass
- **cargo-audit**: Checks for known security vulnerabilities

### Security
- **detect-secrets**: Scans for accidentally committed secrets and tokens

### TOML Formatting
- **pretty-format-toml**: Formats TOML files consistently

## Running Checks Manually

You can run individual checks without committing:

```bash
# Activate virtual environment first
source .venv/bin/activate

# Format Rust code
cargo fmt

# Lint Rust code
cargo clippy --all-targets --all-features -- -D warnings

# Check compilation
cargo check --all-targets --all-features

# Run tests
cargo test --all-targets --all-features

# Security audit
cargo audit

# Run all pre-commit hooks
pre-commit run --all-files

# Run specific hook
pre-commit run rustfmt
```

## Code Quality Standards

The pre-commit hooks enforce the following standards:

### Rust Code
- **Formatting**: Uses `rustfmt` with default settings
- **Linting**: Uses `clippy` with warnings as errors (`-D warnings`)
- **Compilation**: Code must compile without errors
- **Tests**: All tests must pass
- **Security**: No known vulnerabilities in dependencies

### General
- **No trailing whitespace**
- **Files end with newline**
- **LF line endings only**
- **No merge conflict markers**
- **No large files (>500KB)**
- **Valid YAML/TOML/JSON syntax**

## Troubleshooting

### Pre-commit Hook Failures

If pre-commit hooks fail:

1. **Formatting issues**: Run `cargo fmt` to fix automatically
2. **Clippy warnings**: Address the specific warnings shown
3. **Test failures**: Fix the failing tests
4. **Compilation errors**: Fix syntax/type errors

### Virtual Environment Issues

If you encounter Python environment issues:

```bash
# Remove and recreate virtual environment
rm -rf .venv
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements-dev.txt
pre-commit install
```

### Secrets Detection

If detect-secrets finds a false positive:

1. Review the flagged content carefully
2. If it's not actually a secret, add it to the allowlist in `.secrets.baseline`
3. Run `detect-secrets scan --baseline .secrets.baseline --all-files` to update

## Files and Directories

### Added by Setup
- `.venv/` - Python virtual environment (gitignored)
- `.pre-commit-config.yaml` - Pre-commit hook configuration
- `requirements-dev.txt` - Python development dependencies
- `setup-dev-env.sh` - Automated setup script
- `.secrets.baseline` - Detect-secrets baseline file
- `DEVELOPMENT.md` - This documentation

### Updated Files
- `.gitignore` - Added virtual environment and pre-commit exclusions

The development environment ensures consistent code quality, security, and formatting across the project.
