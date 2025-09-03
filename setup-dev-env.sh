#!/bin/bash
# Setup development environment for JIRA MCP Server

set -e

echo "🔧 Setting up development environment..."

# Create virtual environment if it doesn't exist
if [ ! -d ".venv" ]; then
    echo "📦 Creating Python virtual environment..."
    python3 -m venv .venv
else
    echo "📦 Virtual environment already exists"
fi

# Activate virtual environment
echo "🔀 Activating virtual environment..."
source .venv/bin/activate

# Install Python development dependencies
echo "📦 Installing Python development dependencies..."
pip install --upgrade pip
pip install -r requirements-dev.txt

# Install pre-commit hooks
echo "🪝 Installing pre-commit hooks..."
pre-commit install

# Initialize secrets baseline
echo "🔒 Initializing secrets baseline..."
detect-secrets scan --baseline .secrets.baseline || echo "Secrets baseline created/updated"

# Install Rust components if not present
echo "🦀 Checking Rust components..."
rustup component add rustfmt clippy || echo "Rust components already installed"

# Install cargo-audit for security auditing
echo "🔍 Installing cargo-audit..."
cargo install cargo-audit

echo "✅ Development environment setup complete!"
echo ""
echo "To activate the virtual environment in the future, run:"
echo "  source .venv/bin/activate"
echo ""
echo "To run pre-commit hooks manually:"
echo "  pre-commit run --all-files"
echo ""
echo "To run individual checks:"
echo "  cargo fmt        # Format code"
echo "  cargo clippy     # Lint code"
echo "  cargo check      # Check compilation"
echo "  cargo test       # Run tests"
echo "  cargo audit      # Security audit"
