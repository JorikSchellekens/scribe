#!/bin/bash

# Scribe Installation Script

echo "Installing Scribe..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed. Please install Rust first:"
    echo "https://rustup.rs/"
    exit 1
fi

# Build the project
echo "Building project..."
cargo build --release

# Install globally
echo "Installing globally..."
cargo install --path .

echo "Installation complete!"
echo ""
echo "Usage:"
scribe --help 