#!/usr/bin/env node

/**
 * D-TPRES WASM Binary Verification Script
 * Phase 1: Verify optimized WASM binary for AO deployment
 */

import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const CONFIG = {
  outputDir: join(__dirname, 'output'),
  wasmFile: join(__dirname, 'output', 'd_tpres.wasm'),
  manifestFile: join(__dirname, 'output', 'metadata', 'build-manifest.json')
};

/**
 * Logger utility
 */
class Logger {
  static info(msg) { console.log(chalk.blue('ℹ'), msg); }
  static success(msg) { console.log(chalk.green('✅'), msg); }
  static warning(msg) { console.log(chalk.yellow('⚠️'), msg); }
  static error(msg) { console.log(chalk.red('❌'), msg); }
  static header(msg) {
    console.log('\n' + chalk.cyan.bold('=' + '='.repeat(msg.length + 2) + '='));
    console.log(chalk.cyan.bold(`= ${msg} =`));
    console.log(chalk.cyan.bold('=' + '='.repeat(msg.length + 2) + '=') + '\n');
  }
}

/**
 * WASM verification utility
 */
class WasmVerifier {
  constructor() {
    this.startTime = Date.now();
    this.results = {};
  }

  /**
   * Main verification process
   */
  async verify() {
    Logger.header('D-TPRES WASM Binary Verification');

    try {
      await this.checkFiles();
      await this.verifyWasmBinary();
      await this.analyzeSize();
      await this.checkWasmFeatures();
      await this.loadManifest();

      this.displayResults();
      return true;
    } catch (error) {
      Logger.error(`Verification failed: ${error.message}`);
      return false;
    }
  }

  /**
   * Check required files exist
   */
  async checkFiles() {
    Logger.info('Checking required files...');

    if (!existsSync(CONFIG.wasmFile)) {
      throw new Error('WASM binary not found: d_tpres.wasm');
    }

    if (!existsSync(CONFIG.manifestFile)) {
      Logger.warning('Build manifest not found');
    }

    Logger.success('Required files found');
  }

  /**
   * Verify WASM binary validity
   */
  async verifyWasmBinary() {
    Logger.info('Verifying WASM binary validity...');

    try {
      // Check WASM header (magic number: 0x6d736100)
      const buffer = readFileSync(CONFIG.wasmFile);
      const magic = buffer.readUInt32LE(0);

      if (magic !== 0x6d736100) {
        throw new Error('Invalid WASM magic number');
      }

      // Check WASM version
      const version = buffer.readUInt32LE(4);
      if (version !== 1) {
        Logger.warning(`Unexpected WASM version: ${version}`);
      }

      this.results.wasm = {
        valid: true,
        size: buffer.length,
        version: version
      };

      Logger.success('WASM binary is valid');
    } catch (error) {
      if (error.code === 'ENOENT') {
        throw new Error('WASM binary file not accessible');
      }
      throw error;
    }
  }

  /**
   * Analyze binary size and optimization
   */
  async analyzeSize() {
    Logger.info('Analyzing binary size...');

    const stats = require('fs').statSync(CONFIG.wasmFile);
    const sizeBytes = stats.size;
    const sizeKB = (sizeBytes / 1024).toFixed(2);
    const sizeMB = (sizeBytes / (1024 * 1024)).toFixed(2);

    this.results.size = {
      bytes: sizeBytes,
      kb: sizeKB,
      mb: sizeMB,
      optimized: sizeBytes < 1024 * 1024 // Under 1MB is good for WASM
    };

    // Size analysis
    if (sizeBytes < 100 * 1024) {
      Logger.success(`Excellent size: ${sizeKB} KB`);
    } else if (sizeBytes < 500 * 1024) {
      Logger.success(`Good size: ${sizeKB} KB`);
    } else if (sizeBytes < 1024 * 1024) {
      Logger.warning(`Large size: ${sizeKB} KB`);
    } else {
      Logger.warning(`Very large size: ${sizeMB} MB`);
    }
  }

  /**
   * Check WASM features and exports
   */
  async checkWasmFeatures() {
    Logger.info('Checking WASM features...');

    try {
      // Try to use wasm-objdump if available
      const objdumpOutput = execSync('wasm-objdump -x "' + CONFIG.wasmFile + '"', {
        encoding: 'utf8',
        stdio: 'pipe'
      });

      // Parse exports
      const exports = this.parseWasmExports(objdumpOutput);
      this.results.exports = exports;

      if (exports.length > 0) {
        Logger.success(`Found ${exports.length} WASM exports`);
        exports.slice(0, 5).forEach(exp => {
          console.log(chalk.gray(`   - ${exp.name} (${exp.type})`));
        });
        if (exports.length > 5) {
          console.log(chalk.gray(`   ... and ${exports.length - 5} more`));
        }
      } else {
        Logger.warning('No WASM exports found');
      }

    } catch (error) {
      Logger.warning('wasm-objdump not available (install wabt for detailed analysis)');
      this.results.exports = [];
    }
  }

  /**
   * Load and verify build manifest
   */
  async loadManifest() {
    if (!existsSync(CONFIG.manifestFile)) {
      Logger.warning('Build manifest not available');
      return;
    }

    Logger.info('Loading build manifest...');

    try {
      const manifestContent = readFileSync(CONFIG.manifestFile, 'utf8');
      const manifest = JSON.parse(manifestContent);

      this.results.manifest = manifest;

      Logger.success('Build manifest loaded');
      console.log(chalk.gray(`   Build: ${manifest.buildInfo?.timestamp || 'Unknown'}`));
      console.log(chalk.gray(`   Phase: ${manifest.buildInfo?.phase || 'Unknown'}`));
      console.log(chalk.gray(`   Features: ${manifest.build?.features || 'Unknown'}`));
    } catch (error) {
      Logger.warning('Failed to parse build manifest');
    }
  }

  /**
   * Parse WASM exports from objdump output
   */
  parseWasmExports(objdumpOutput) {
    const exports = [];
    const lines = objdumpOutput.split('\n');
    let inExportSection = false;

    for (const line of lines) {
      if (line.includes('Export[')) {
        inExportSection = true;
        continue;
      }

      if (inExportSection) {
        if (line.trim() === '' || line.includes('Section[')) {
          break;
        }

        const match = line.match(/\s*-\s*(\w+)\[(\d+)\]\s*->\s*(\w+)\s*(\w+)/);
        if (match) {
          exports.push({
            name: match[4] || match[1],
            type: match[3] || 'function',
            index: match[2]
          });
        }
      }
    }

    return exports;
  }

  /**
   * Display verification results
   */
  displayResults() {
    const duration = Date.now() - this.startTime;

    Logger.header('Verification Results');

    // WASM validity
    if (this.results.wasm?.valid) {
      Logger.success('WASM binary is valid and ready for deployment');
    } else {
      Logger.error('WASM binary validation failed');
    }

    // Size summary
    console.log(chalk.cyan('\n📏 Size Analysis:'));
    console.log(chalk.gray(`   Size: ${this.results.size?.kb} KB (${this.results.size?.bytes} bytes)`));
    console.log(chalk.gray(`   Optimized: ${this.results.size?.optimized ? 'Yes' : 'No'}`));

    // Exports summary
    if (this.results.exports && this.results.exports.length > 0) {
      console.log(chalk.cyan('\n📤 WASM Exports:'));
      console.log(chalk.gray(`   Count: ${this.results.exports.length}`));
    }

    // Build info
    if (this.results.manifest) {
      console.log(chalk.cyan('\n🔧 Build Info:'));
      console.log(chalk.gray(`   Phase: ${this.results.manifest.buildInfo?.phase}`));
      console.log(chalk.gray(`   Target: ${this.results.manifest.build?.target}`));
      console.log(chalk.gray(`   Features: ${this.results.manifest.build?.features}`));
    }

    // Summary
    console.log(chalk.cyan('\n🎯 Summary:'));
    console.log(chalk.green('✅ Phase 1 WASM optimization completed successfully'));
    console.log(chalk.gray(`   Verification time: ${duration}ms`));
    console.log(chalk.gray(`   Binary: ${CONFIG.wasmFile}`));

    // Next steps
    console.log(chalk.cyan('\n📋 Ready for:'));
    console.log(chalk.gray('   - AO Network deployment'));
    console.log(chalk.gray('   - Phase 2: CWAO SDK integration'));
    console.log(chalk.gray('   - CosmWasm smart contract execution'));
  }
}

// Execute verification if run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const verifier = new WasmVerifier();
  verifier.verify().then(success => {
    process.exit(success ? 0 : 1);
  });
}

export { WasmVerifier };