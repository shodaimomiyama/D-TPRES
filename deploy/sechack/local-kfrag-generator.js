#!/usr/bin/env node

/**
 * D-TPRES Local kFrag Generator
 * Phase 2B: WASM-pack integration for local cryptographic operations
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Logger utility for local crypto operations
 */
class CryptoLogger {
  static info(msg) { console.log(chalk.blue('ℹ'), msg); }
  static success(msg) { console.log(chalk.green('✅'), msg); }
  static warning(msg) { console.log(chalk.yellow('⚠️'), msg); }
  static error(msg) { console.log(chalk.red('❌'), msg); }
  static crypto(msg) { console.log(chalk.magenta('🔐'), msg); }
  static wasm(msg) { console.log(chalk.cyan('🦀'), msg); }
}

/**
 * Local kFrag Generator using WASM-pack
 */
export class LocalKFragGenerator {
  constructor(target = 'node') {
    this.target = target; // 'node' or 'browser'
    this.wasmModule = null;
    this.LocalCrypto = null;
    this.crypto = null;
    this.initialized = false;
  }

  /**
   * Initialize WASM module
   */
  async initialize() {
    CryptoLogger.wasm('Initializing WASM module for local crypto operations...');

    try {
      // Determine package path based on target
      const pkgPath = this.target === 'browser'
        ? join(__dirname, 'pkg', 'd_tpres.js')
        : join(__dirname, 'pkg-node', 'd_tpres_node.js');

      CryptoLogger.info(`Loading WASM package: ${pkgPath}`);

      // Load WASM module - no mock fallback (prohibited)
      try {
        this.wasmModule = await import(pkgPath);
        CryptoLogger.success('WASM module loaded successfully');
      } catch (importError) {
        CryptoLogger.error(`WASM module import failed: ${importError.message}`);
        throw new Error(`Failed to load WASM module: ${importError.message}. Mock operations are prohibited.\n\nRequired setup:\n1. Build WASM module: npm run build-wasm-pack\n2. Ensure pkg-node/d_tpres_node.js exists\n3. Verify WASM files are properly generated`);
      }

      // Initialize WASM module
      if (this.target === 'browser') {
        // For browser, init function is available
        await this.wasmModule.default();
      } else {
        // For Node.js, call init function if available
        if (this.wasmModule.init) {
          await this.wasmModule.init();
        } else if (this.wasmModule.default) {
          await this.wasmModule.default();
        }
      }

      // Get LocalCrypto class
      this.LocalCrypto = this.wasmModule.LocalCrypto;

      if (!this.LocalCrypto) {
        throw new Error('LocalCrypto class not found in WASM module');
      }

      // Create crypto instance
      this.crypto = new this.LocalCrypto();
      await this.crypto.initialize();

      this.initialized = true;
      CryptoLogger.success('WASM module initialized successfully');

      // Log version info
      const version = this.crypto.version();
      CryptoLogger.wasm(`Loaded: ${version}`);

      return true;
    } catch (error) {
      CryptoLogger.error(`Failed to initialize WASM module: ${error.message}`);
      throw error;
    }
  }

  /**
   * Generate kFrags locally
   */
  async generateKFrags(secret, threshold = 2, totalShares = 3) {
    this.ensureInitialized();

    CryptoLogger.crypto(`Generating kFrags locally...`);
    CryptoLogger.info(`Parameters: threshold=${threshold}, totalShares=${totalShares}`);

    try {
      // Validate inputs
      if (!this.validateInputs(secret, threshold, totalShares)) {
        throw new Error('Invalid input parameters');
      }

      // Convert secret to Uint8Array if needed
      const secretBytes = this.prepareSecret(secret);

      // Validate secret
      const isValidSecret = this.crypto.validate_secret(secretBytes);
      if (!isValidSecret) {
        throw new Error('Invalid secret: must be 16-1024 bytes');
      }

      CryptoLogger.crypto('Generating kFrags with WASM...');

      // Generate kFrags using WASM
      const resultJson = this.crypto.generate_kfrags(
        secretBytes,
        threshold,
        totalShares
      );

      // Parse result
      const result = JSON.parse(resultJson);

      if (!result.success) {
        throw new Error('kFrag generation failed');
      }

      CryptoLogger.success(`Generated ${result.kfrags.length} kFrags successfully`);
      CryptoLogger.info(`Capsule ID: ${result.capsule_id}`);

      // Also generate Shamir secret shares
      CryptoLogger.crypto('Generating Shamir secret shares...');
      const sharesJson = this.crypto.generate_shamir_shares(
        secretBytes,
        threshold,
        totalShares
      );
      const shares = JSON.parse(sharesJson);

      CryptoLogger.success(`Generated ${shares.length} Shamir shares`);

      return {
        kfrags: result.kfrags,
        shares: shares,
        sharesCount: result.shares_count,
        threshold: result.threshold,
        capsuleId: result.capsule_id,
        timestamp: Date.now(),
        source: 'local-wasm'
      };
    } catch (error) {
      CryptoLogger.error(`kFrag generation failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Generate test secret
   */
  async generateTestSecret(length = 32) {
    this.ensureInitialized();

    CryptoLogger.crypto(`Generating test secret (${length} bytes)...`);

    try {
      const secretBytes = this.crypto.generate_test_secret(length);
      CryptoLogger.success('Test secret generated successfully');
      return secretBytes;
    } catch (error) {
      CryptoLogger.error(`Test secret generation failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Validate secret before processing
   */
  validateSecret(secret) {
    this.ensureInitialized();

    const secretBytes = this.prepareSecret(secret);
    return this.crypto.validate_secret(secretBytes);
  }

  /**
   * Get WASM module information
   */
  getModuleInfo() {
    if (!this.initialized) {
      return {
        initialized: false,
        target: this.target,
        version: null
      };
    }

    return {
      initialized: true,
      target: this.target,
      version: this.crypto.version(),
      wasmSupported: typeof WebAssembly !== 'undefined'
    };
  }

  /**
   * Validate input parameters
   */
  validateInputs(secret, threshold, totalShares) {
    if (!secret || secret.length === 0) {
      CryptoLogger.error('Secret cannot be empty');
      return false;
    }

    if (threshold <= 0 || totalShares <= 0) {
      CryptoLogger.error('Threshold and totalShares must be > 0');
      return false;
    }

    if (threshold > totalShares) {
      CryptoLogger.error('Threshold cannot be greater than totalShares');
      return false;
    }

    return true;
  }

  /**
   * Prepare secret data for WASM
   */
  prepareSecret(secret) {
    if (secret instanceof Uint8Array) {
      return secret;
    }

    if (typeof secret === 'string') {
      return new TextEncoder().encode(secret);
    }

    if (Array.isArray(secret)) {
      return new Uint8Array(secret);
    }

    if (Buffer.isBuffer(secret)) {
      return new Uint8Array(secret);
    }

    throw new Error('Secret must be string, Uint8Array, Array, or Buffer');
  }

  /**
   * Ensure WASM module is initialized
   */
  ensureInitialized() {
    if (!this.initialized) {
      throw new Error('WASM module not initialized. Call initialize() first.');
    }
  }
}

/**
 * Utility function for quick kFrag generation
 */
export async function generateKFragsQuick(secret, threshold = 2, totalShares = 3, target = 'node') {
  const generator = new LocalKFragGenerator(target);
  await generator.initialize();
  return await generator.generateKFrags(secret, threshold, totalShares);
}

// Export for module usage
export default LocalKFragGenerator;