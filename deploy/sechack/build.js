#!/usr/bin/env node

/**
 * D-TPRES SecHack Build Orchestrator
 * Phase 1: WASM optimization build environment
 */

import { execSync } from 'child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const CONFIG = {
  projectRoot: join(__dirname, '../..'),
  outputDir: join(__dirname, 'output'),
  wasmTarget: 'wasm32-unknown-unknown',
  features: 'wasm-ao',
  buildScript: join(__dirname, 'build-wasm.sh')
};

/**
 * Logger utility with colored output
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
 * Build orchestrator class
 */
class D_TPRES_Builder {
  constructor() {
    this.startTime = Date.now();
  }

  /**
   * Main build process
   */
  async build() {
    Logger.header('D-TPRES WASM Build Orchestrator (Phase 1)');

    try {
      await this.validateEnvironment();
      await this.setupOutputDirectory();
      await this.buildWasm();
      await this.verifyBuild();
      await this.generateManifest();

      this.displaySummary();
    } catch (error) {
      Logger.error(`Build failed: ${error.message}`);
      process.exit(1);
    }
  }

  /**
   * Validate build environment
   */
  async validateEnvironment() {
    Logger.info('Validating build environment...');

    // Check project structure
    const cargoToml = join(CONFIG.projectRoot, 'Cargo.toml');
    if (!existsSync(cargoToml)) {
      throw new Error('Cargo.toml not found in project root');
    }

    // Validate Cargo.toml features
    const cargoContent = readFileSync(cargoToml, 'utf8');
    if (!cargoContent.includes('wasm-ao = []')) {
      throw new Error('wasm-ao feature not found in Cargo.toml');
    }

    // Check build script
    if (!existsSync(CONFIG.buildScript)) {
      throw new Error('build-wasm.sh script not found');
    }

    // Verify Rust installation
    try {
      const rustVersion = execSync('rustc --version', { encoding: 'utf8', cwd: CONFIG.projectRoot });
      Logger.success(`Rust toolchain: ${rustVersion.trim()}`);
    } catch (error) {
      throw new Error('Rust toolchain not found. Please install Rust.');
    }

    Logger.success('Environment validation completed');
  }

  /**
   * Setup output directory structure
   */
  async setupOutputDirectory() {
    Logger.info('Setting up output directory...');

    mkdirSync(CONFIG.outputDir, { recursive: true });

    // Create subdirectories
    const subdirs = ['wasm', 'metadata', 'logs'];
    subdirs.forEach(dir => {
      mkdirSync(join(CONFIG.outputDir, dir), { recursive: true });
    });

    Logger.success('Output directory structure created');
  }

  /**
   * Execute WASM build
   */
  async buildWasm() {
    Logger.info('Building WASM binary...');

    try {
      // Execute build script
      const buildOutput = execSync(CONFIG.buildScript, {
        encoding: 'utf8',
        cwd: CONFIG.projectRoot,
        stdio: 'pipe'
      });

      // Save build log
      const logFile = join(CONFIG.outputDir, 'logs', `build-${Date.now()}.log`);
      writeFileSync(logFile, buildOutput);

      Logger.success('WASM build completed successfully');
    } catch (error) {
      const errorLog = join(CONFIG.outputDir, 'logs', `build-error-${Date.now()}.log`);
      writeFileSync(errorLog, error.stdout + '\n' + error.stderr);

      throw new Error(`WASM build failed. Check log: ${errorLog}`);
    }
  }

  /**
   * Verify build output
   */
  async verifyBuild() {
    Logger.info('Verifying build output...');

    const wasmFile = join(CONFIG.outputDir, 'd_tpres.wasm');
    if (!existsSync(wasmFile)) {
      throw new Error('WASM binary not found in output directory');
    }

    // Get file stats
    const stats = require('fs').statSync(wasmFile);
    const sizeKB = (stats.size / 1024).toFixed(2);

    Logger.success(`WASM binary verified: ${sizeKB} KB`);

    // Copy to wasm subdirectory
    const wasmOutput = join(CONFIG.outputDir, 'wasm', 'd_tpres.wasm');
    require('fs').copyFileSync(wasmFile, wasmOutput);

    return {
      size: stats.size,
      sizeKB,
      path: wasmFile
    };
  }

  /**
   * Generate build manifest
   */
  async generateManifest() {
    Logger.info('Generating build manifest...');

    const manifest = {
      buildInfo: {
        timestamp: new Date().toISOString(),
        duration: Date.now() - this.startTime,
        version: '0.1.0-mvp',
        phase: 'Phase 1 - WASM Optimization'
      },
      build: {
        target: CONFIG.wasmTarget,
        features: CONFIG.features,
        profile: 'release',
        optimizations: {
          'opt-level': 'z',
          'lto': true,
          'codegen-units': 1,
          'strip': true,
          'panic': 'abort',
          'overflow-checks': false
        }
      },
      output: {
        wasm: 'd_tpres.wasm',
        size: this.verifyResult?.size || 0,
        validated: true
      },
      environment: {
        node: process.version,
        platform: process.platform,
        arch: process.arch
      },
      nextSteps: [
        'Phase 2: CWAO SDK integration',
        'Phase 3: Enable actual encryption processing',
        'Phase 4: AO messaging implementation'
      ]
    };

    const manifestFile = join(CONFIG.outputDir, 'metadata', 'build-manifest.json');
    writeFileSync(manifestFile, JSON.stringify(manifest, null, 2));

    Logger.success('Build manifest generated');
  }

  /**
   * Display build summary
   */
  displaySummary() {
    const duration = Date.now() - this.startTime;

    Logger.header('Build Summary');
    console.log(chalk.green('🎉 Phase 1 build completed successfully!'));
    console.log(chalk.gray(`   Duration: ${duration}ms`));
    console.log(chalk.gray(`   Output: ${CONFIG.outputDir}/`));
    console.log(chalk.gray(`   WASM: d_tpres.wasm`));
    console.log('\n' + chalk.cyan('📋 Next Steps:'));
    console.log(chalk.gray('   1. Review build manifest: output/metadata/build-manifest.json'));
    console.log(chalk.gray('   2. Test WASM binary: npm run verify'));
    console.log(chalk.gray('   3. Request Phase 2 approval from user'));
    console.log();
  }
}

// Execute build if run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const builder = new D_TPRES_Builder();
  builder.build().catch(error => {
    Logger.error(`Build failed: ${error.message}`);
    process.exit(1);
  });
}

export { D_TPRES_Builder };