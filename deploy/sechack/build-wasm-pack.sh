#!/bin/bash

# D-TPRES wasm-pack Build Script
# Phase 2B: Build WASM package for JavaScript/browser integration

set -e

echo "🏗️  D-TPRES wasm-pack Build Script (Phase 2B)"
echo "=============================================="

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

# Build configuration
OUTPUT_DIR="deploy/sechack/pkg"
NODE_OUTPUT_DIR="deploy/sechack/pkg-node"

echo "📋 Build Configuration:"
echo "  Project: $PROJECT_ROOT"
echo "  Browser Output: $OUTPUT_DIR"
echo "  Node.js Output: $NODE_OUTPUT_DIR"
echo ""

# Check wasm-pack installation
echo "🔧 Checking wasm-pack..."
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ Error: wasm-pack not found. Please install wasm-pack."
    echo "   Install: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
else
    WASM_PACK_VERSION=$(wasm-pack --version)
    echo "✅ wasm-pack found: $WASM_PACK_VERSION"
fi

echo ""

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf "$OUTPUT_DIR"
rm -rf "$NODE_OUTPUT_DIR"

# Create output directories
mkdir -p "$(dirname "$OUTPUT_DIR")"
mkdir -p "$(dirname "$NODE_OUTPUT_DIR")"

echo "🔨 Building WASM package for browser..."
echo "Features: browser (wasm-bindgen + js-sys + web-sys)"

# Build for browser/web
wasm-pack build \
    --target web \
    --out-dir "$OUTPUT_DIR" \
    --features browser \
    --no-default-features

# Check if browser build succeeded
if [ ! -f "$OUTPUT_DIR/d_tpres.js" ]; then
    echo "❌ Error: Browser WASM build failed"
    exit 1
fi

# Get browser package size
BROWSER_WASM_SIZE=$(wc -c < "$OUTPUT_DIR/d_tpres_bg.wasm")
echo "✅ Browser WASM package built successfully!"
echo "   Size: $BROWSER_WASM_SIZE bytes"

echo ""
echo "🔨 Building WASM package for Node.js..."

# Build for Node.js
wasm-pack build \
    --target bundler \
    --out-dir "$NODE_OUTPUT_DIR" \
    --features browser \
    --no-default-features

# Check if Node.js build succeeded
if [ ! -f "$NODE_OUTPUT_DIR/d_tpres.js" ]; then
    echo "❌ Error: Node.js WASM build failed"
    exit 1
fi

# Get Node.js package size
NODE_WASM_SIZE=$(wc -c < "$NODE_OUTPUT_DIR/d_tpres_bg.wasm")
echo "✅ Node.js WASM package built successfully!"
echo "   Size: $NODE_WASM_SIZE bytes"

echo ""
echo "📦 Updating package.json files..."

# Update browser package.json
if [ -f "$OUTPUT_DIR/package.json" ]; then
    # Add module type and update name
    node -e "
    const fs = require('fs');
    const pkg = JSON.parse(fs.readFileSync('$OUTPUT_DIR/package.json', 'utf8'));
    pkg.name = 'd-tpres-crypto-browser';
    pkg.description = 'D-TPRES Local Crypto Operations for Browser';
    pkg.type = 'module';
    pkg.keywords = ['d-tpres', 'crypto', 'wasm', 'browser', 'threshold-crypto'];
    fs.writeFileSync('$OUTPUT_DIR/package.json', JSON.stringify(pkg, null, 2));
    "
    echo "✅ Browser package.json updated"
fi

# Update Node.js package.json
if [ -f "$NODE_OUTPUT_DIR/package.json" ]; then
    node -e "
    const fs = require('fs');
    const pkg = JSON.parse(fs.readFileSync('$NODE_OUTPUT_DIR/package.json', 'utf8'));
    pkg.name = 'd-tpres-crypto-node';
    pkg.description = 'D-TPRES Local Crypto Operations for Node.js';
    pkg.keywords = ['d-tpres', 'crypto', 'wasm', 'nodejs', 'threshold-crypto'];
    fs.writeFileSync('$NODE_OUTPUT_DIR/package.json', JSON.stringify(pkg, null, 2));
    "
    echo "✅ Node.js package.json updated"
fi

echo ""
echo "📋 WASM Package Summary:"
echo "========================="
echo "✅ Browser target: web"
echo "✅ Node.js target: nodejs"
echo "✅ Features: browser (wasm-bindgen + crypto)"
echo "✅ Browser output: $OUTPUT_DIR/"
echo "✅ Node.js output: $NODE_OUTPUT_DIR/"

# Show package contents
echo ""
echo "📁 Browser Package Contents:"
ls -la "$OUTPUT_DIR/" | head -10

echo ""
echo "📁 Node.js Package Contents:"
ls -la "$NODE_OUTPUT_DIR/" | head -10

# Verify WASM files
echo ""
echo "🔍 WASM Package Verification:"
if command -v wasm-validate &> /dev/null; then
    echo "Browser WASM:"
    if wasm-validate "$OUTPUT_DIR/d_tpres_bg.wasm"; then
        echo "✅ Browser WASM is valid"
    else
        echo "❌ Browser WASM validation failed"
        exit 1
    fi

    echo "Node.js WASM:"
    if wasm-validate "$NODE_OUTPUT_DIR/d_tpres_bg.wasm"; then
        echo "✅ Node.js WASM is valid"
    else
        echo "❌ Node.js WASM validation failed"
        exit 1
    fi
else
    echo "⚠️  wasm-validate not found (install wabt for validation)"
fi

echo ""
echo "🎉 wasm-pack build completed successfully!"
echo "   Browser package ready: $OUTPUT_DIR/"
echo "   Node.js package ready: $NODE_OUTPUT_DIR/"
echo ""
echo "💡 Next steps:"
echo "   - Test with: npm run test-wasm-pack"
echo "   - Import in JS: import init, { LocalCrypto } from './pkg/d_tpres.js'"