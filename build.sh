#!/bin/bash

# D-TPRES WebAssembly Build Script for AO Network
# This script builds the Rust contract to WebAssembly for deployment on AO

set -e

echo "🚀 Building D-TPRES for AO Network..."

# Check if Rust and wasm32 target are installed
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first."
    exit 1
fi

if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "📦 Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Backup rust-toolchain.toml if it exists
if [ -f "rust-toolchain.toml" ]; then
    echo "📋 Temporarily moving rust-toolchain.toml..."
    mv rust-toolchain.toml rust-toolchain.toml.bak
fi

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Build the contract with proper flags for WASM
echo "🔨 Building WebAssembly contract..."
RUSTFLAGS='-C link-arg=-s' cargo build \
    --target wasm32-unknown-unknown \
    --release \
    --lib \
    --no-default-features

# Restore rust-toolchain.toml if it was backed up
if [ -f "rust-toolchain.toml.bak" ]; then
    echo "📋 Restoring rust-toolchain.toml..."
    mv rust-toolchain.toml.bak rust-toolchain.toml
fi

# Check if wasm-opt is installed for optimization
if command -v wasm-opt &> /dev/null; then
    echo "⚡ Optimizing WASM binary with wasm-opt..."
    wasm-opt -Os \
        target/wasm32-unknown-unknown/release/d_tpres.wasm \
        -o d_tpres_optimized.wasm
    
    # Show file sizes
    echo "📊 Build complete!"
    echo "Original size: $(du -h target/wasm32-unknown-unknown/release/d_tpres.wasm | cut -f1)"
    echo "Optimized size: $(du -h d_tpres_optimized.wasm | cut -f1)"
else
    echo "⚠️  wasm-opt not found. Skipping optimization."
    echo "   Install with: npm install -g wasm-opt"
    cp target/wasm32-unknown-unknown/release/d_tpres.wasm d_tpres_optimized.wasm
    echo "📊 Build complete!"
    echo "Size: $(du -h d_tpres_optimized.wasm | cut -f1)"
fi

echo "✅ WebAssembly build successful!"
echo "📦 Output: d_tpres_optimized.wasm"
echo ""
echo "Next steps:"
echo "1. Test locally: cargo test"
echo "2. Deploy to AO: npm run deploy"