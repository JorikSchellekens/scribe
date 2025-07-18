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
echo "  scribe                    # Generate site with default config"
echo "  scribe --help             # Show help"
echo "  scribe --config my.json   # Use custom config file"
echo ""
echo "The site generator will create a config.json file on first run." 