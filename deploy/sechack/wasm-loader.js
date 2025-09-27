#!/usr/bin/env node

/**
 * D-TPRES WASM Loader for Node.js
 * Phase 2B: Handle WASM loading in Node.js environment
 */

import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Load WASM module in Node.js environment
 */
export async function loadWasmModule(target = 'node') {
  const wasmDir = target === 'browser' ? 'pkg' : 'pkg-node';
  const wasmPath = join(__dirname, wasmDir, 'd_tpres_bg.wasm');

  try {
    // Read WASM binary
    const wasmBuffer = await readFile(wasmPath);

    // Create mock environment for WASM module
    const wasmImports = {
      env: {
        // Empty env module to satisfy WASM imports
      },
      __wbindgen_placeholder__: {
        // Placeholder for wasm-bindgen functions
      }
    };

    // Instantiate WASM module
    const wasmModule = await WebAssembly.instantiate(wasmBuffer, wasmImports);

    return wasmModule;
  } catch (error) {
    console.error(`Failed to load WASM module: ${error.message}`);
    throw error;
  }
}

/**
 * Create mock LocalCrypto class for testing
 */
export class MockLocalCrypto {
  constructor() {
    this.initialized = false;
  }

  async initialize() {
    this.initialized = true;
    return Promise.resolve();
  }

  generate_kfrags(secret, threshold, totalShares) {
    if (!this.initialized) {
      throw new Error('LocalCrypto not initialized');
    }

    // Mock kFrag generation for testing
    const mockKfrags = [];
    for (let i = 0; i < totalShares; i++) {
      mockKfrags.push({
        id: i,
        key_data: new Array(32).fill(0).map(() => Math.floor(Math.random() * 256)),
        verification_data: new Array(32).fill(0).map(() => Math.floor(Math.random() * 256)),
        precursor: new Array(32).fill(0).map(() => Math.floor(Math.random() * 256))
      });
    }

    const result = {
      kfrags: mockKfrags,
      shares_count: totalShares,
      threshold: threshold,
      capsule_id: `mock_capsule_${Date.now()}`,
      success: true
    };

    return JSON.stringify(result);
  }

  validate_secret(secret) {
    return secret && secret.length >= 16 && secret.length <= 1024;
  }

  generate_test_secret(length) {
    return new Uint8Array(length).map(() => Math.floor(Math.random() * 256));
  }

  version() {
    return 'D-TPRES LocalCrypto v0.1.0-mvp (Mock)';
  }
}

export default { loadWasmModule, MockLocalCrypto };