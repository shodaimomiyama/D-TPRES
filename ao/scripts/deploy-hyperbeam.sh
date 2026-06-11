#!/usr/bin/env bash
set -euo pipefail

# HyperBEAM E2E test runner
# Prerequisites: running HyperBEAM node, WALLET_PATH, WASM target installed
#
# Usage: WALLET_PATH=/path/to/wallet.json ./ao/scripts/deploy-hyperbeam.sh

HB_PORT="${HB_PORT:-10000}"
HB_URL="http://localhost:${HB_PORT}"

echo "=== HyperBEAM E2E Test Runner ==="

# 1. Check wallet
if [ -z "${WALLET_PATH:-}" ]; then
    echo "ERROR: WALLET_PATH is not set"
    echo "  export WALLET_PATH=/path/to/arweave-wallet.json"
    exit 1
fi
if [ ! -f "$WALLET_PATH" ]; then
    echo "ERROR: wallet file not found: $WALLET_PATH"
    exit 1
fi
echo "[OK] Wallet: $WALLET_PATH"

# 2. Check HyperBEAM health
if ! curl -sf "${HB_URL}/~meta@1.0/info" > /dev/null 2>&1; then
    echo "ERROR: HyperBEAM not reachable at ${HB_URL}"
    echo "  Start it with: cd hyperbeam-sandbox && ./scripts/start.sh"
    exit 1
fi
echo "[OK] HyperBEAM: ${HB_URL}"

# Repo has no root cargo workspace — client/ and ao/contracts/ are standalone crates,
# so each cargo invocation needs an explicit manifest path.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# 3. Build WASM
if ! command -v wasm-tools >/dev/null 2>&1; then
    echo "ERROR: wasm-tools is required (cargo install wasm-tools)"
    exit 1
fi
echo "Building WASM contract..."
# -reference-types prevents reftype constructs in our own code; the prebuilt
# core/std still carry call_indirect overlong LEB encodings, which HyperBEAM's
# WAMR (built without REF_TYPES) rejects with "zero byte expected" — see #101.
RUSTFLAGS="-C target-feature=-reference-types" \
    cargo build --manifest-path "${REPO_ROOT}/ao/contracts/Cargo.toml" \
    --target wasm32-unknown-unknown --release
RAW_WASM="${REPO_ROOT}/ao/contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm"
if [ ! -f "$RAW_WASM" ]; then
    echo "ERROR: WASM binary not found at $RAW_WASM"
    exit 1
fi

# wasm-tools print|parse round-trip re-encodes the binary canonically,
# rewriting the overlong LEBs into the MVP form WAMR accepts (#101).
WASM_PATH="${RAW_WASM%.wasm}.canonical.wasm"
WAT_TMP="$(mktemp -t formix-contract).wat"
wasm-tools print "$RAW_WASM" -o "$WAT_TMP"
wasm-tools parse "$WAT_TMP" -o "$WASM_PATH"
rm -f "$WAT_TMP"
echo "[OK] WASM: $WASM_PATH ($(wc -c < "$WASM_PATH") bytes)"

# 4. Cache the image on the node (RPC — see cache-wasm.escript for why
#    the HTTP cache device cannot be used for images)
echo "Caching WASM image on the node..."
WASM_IMAGE_ID="$(escript "${REPO_ROOT}/ao/scripts/cache-wasm.escript" "$WASM_PATH")"
if [ -z "$WASM_IMAGE_ID" ]; then
    echo "ERROR: failed to obtain image ID from cache-wasm.escript"
    exit 1
fi
echo "[OK] Image ID: $WASM_IMAGE_ID"

# 5. Run E2E test
echo "Running E2E test..."
export WASM_IMAGE_ID
cargo test --manifest-path "${REPO_ROOT}/client/Cargo.toml" \
    --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture
echo "=== DONE ==="
