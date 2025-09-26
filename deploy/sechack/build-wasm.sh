#!/bin/bash

# D-TPRES WASM Build Script for AO deployment
# Phase 1: Optimized WASM build environment

set -e

echo "🚀 D-TPRES WASM Build Script (Phase 1)"
echo "======================================"

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

# Build configuration
TARGET="wasm32-unknown-unknown"
RELEASE_DIR="target/$TARGET/release"
OUTPUT_DIR="deploy/sechack/output"

echo "📋 Build Configuration:"
echo "  Target: $TARGET"
echo "  Project: $PROJECT_ROOT"
echo "  Output: $OUTPUT_DIR"
echo ""

# Check Rust toolchain
echo "🔧 Checking Rust toolchain..."
if ! command -v rustup &> /dev/null; then
    echo "❌ Error: rustup not found. Please install Rust toolchain."
    exit 1
fi

# Add WASM target if not installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "📥 Installing WASM target: $TARGET"
    rustup target add "$TARGET"
else
    echo "✅ WASM target already installed: $TARGET"
fi

# Check wasm-opt (binaryen) for post-processing
echo "🔧 Checking wasm-opt..."
if command -v wasm-opt &> /dev/null; then
    WASM_OPT_VERSION=$(wasm-opt --version)
    echo "✅ wasm-opt found: $WASM_OPT_VERSION"
    HAS_WASM_OPT=true
else
    echo "⚠️  wasm-opt not found. Install binaryen for additional optimization."
    echo "   MacOS: brew install binaryen"
    echo "   Ubuntu: apt install binaryen"
    HAS_WASM_OPT=false
fi

echo ""

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean --target "$TARGET"

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "🔨 Building WASM binary with AO features..."
echo "Features: wasm-ao (no default features)"

# Build with AO features only
cargo build --target "$TARGET" --release --no-default-features --features wasm-ao

# Check if build succeeded
WASM_FILE="$RELEASE_DIR/d_tpres.wasm"
if [ ! -f "$WASM_FILE" ]; then
    echo "❌ Error: WASM build failed. Binary not found at $WASM_FILE"
    exit 1
fi

# Get file size
ORIGINAL_SIZE=$(wc -c < "$WASM_FILE")
echo "✅ WASM binary built successfully!"
echo "   Size: $ORIGINAL_SIZE bytes"

# Copy to output directory
cp "$WASM_FILE" "$OUTPUT_DIR/"

# Post-process with wasm-opt if available
if [ "$HAS_WASM_OPT" = true ]; then
    echo ""
    echo "🚀 Post-processing with wasm-opt..."

    OPTIMIZED_FILE="$OUTPUT_DIR/d_tpres_optimized.wasm"

    # Apply aggressive optimizations
    wasm-opt -Oz --enable-bulk-memory --enable-sign-ext "$WASM_FILE" -o "$OPTIMIZED_FILE"

    if [ -f "$OPTIMIZED_FILE" ]; then
        OPTIMIZED_SIZE=$(wc -c < "$OPTIMIZED_FILE")
        REDUCTION=$((ORIGINAL_SIZE - OPTIMIZED_SIZE))
        REDUCTION_PERCENT=$(( (REDUCTION * 100) / ORIGINAL_SIZE ))

        echo "✅ Optimized WASM binary created!"
        echo "   Original: $ORIGINAL_SIZE bytes"
        echo "   Optimized: $OPTIMIZED_SIZE bytes"
        echo "   Reduction: $REDUCTION bytes ($REDUCTION_PERCENT%)"

        # Use optimized version as primary
        cp "$OPTIMIZED_FILE" "$OUTPUT_DIR/d_tpres.wasm"
    else
        echo "⚠️  wasm-opt failed, using original binary"
    fi
fi

echo ""
echo "📊 Build Summary:"
echo "=================="
echo "✅ Target: $TARGET"
echo "✅ Features: wasm-ao"
echo "✅ Profile: release (opt-level=z, LTO=true)"
echo "✅ Output: $OUTPUT_DIR/d_tpres.wasm"

# Verify WASM binary
echo ""
echo "🔍 WASM Binary Verification:"
if command -v wasm-validate &> /dev/null; then
    if wasm-validate "$OUTPUT_DIR/d_tpres.wasm"; then
        echo "✅ WASM binary is valid"
    else
        echo "❌ WASM binary validation failed"
        exit 1
    fi
else
    echo "⚠️  wasm-validate not found (install wabt for validation)"
fi

# Show exports (if wasm-objdump available)
if command -v wasm-objdump &> /dev/null; then
    echo ""
    echo "📋 WASM Exports:"
    wasm-objdump -x "$OUTPUT_DIR/d_tpres.wasm" | grep -A 20 "Export\[" | head -20
fi

echo ""
echo "🎉 WASM build completed successfully!"
echo "   Binary ready for AO deployment: $OUTPUT_DIR/d_tpres.wasm"