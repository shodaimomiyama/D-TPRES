#!/usr/bin/env node

/**
 * D-TPRES WASM-pack Integration Test
 * Phase 2B: Test WASM package functionality with actual kFrag generation
 */

import { LocalKFragGenerator } from './local-kfrag-generator.js';
import chalk from 'chalk';

class WasmPackTestRunner {
  constructor() {
    this.passed = 0;
    this.failed = 0;
  }

  async test(name, fn) {
    try {
      console.log(chalk.cyan(`🧪 Testing: ${name}`));
      const startTime = Date.now();
      await fn();
      const duration = Date.now() - startTime;
      console.log(chalk.green(`✅ PASS: ${name} (${duration}ms)`));
      this.passed++;
    } catch (error) {
      console.log(chalk.red(`❌ FAIL: ${name}`));
      console.log(chalk.red(`   Error: ${error.message}`));
      this.failed++;
    }
  }

  summary() {
    const total = this.passed + this.failed;
    console.log(chalk.cyan('\\n📊 WASM-pack Test Summary:'));
    console.log(chalk.green(`✅ Passed: ${this.passed}/${total}`));
    console.log(chalk.red(`❌ Failed: ${this.failed}/${total}`));

    if (this.failed === 0) {
      console.log(chalk.green('🎉 All WASM-pack tests passed!'));
    } else {
      console.log(chalk.red('💥 Some WASM-pack tests failed'));
    }
  }
}

async function main() {
  console.log(chalk.blue.bold('🦀 D-TPRES WASM-pack Integration Test\\n'));

  const runner = new WasmPackTestRunner();

  // Test 1: Initialize LocalKFragGenerator
  await runner.test('Initialize LocalKFragGenerator', async () => {
    const generator = new LocalKFragGenerator('node');

    if (typeof generator.initialize !== 'function') {
      throw new Error('initialize method not found');
    }

    await generator.initialize();

    const info = generator.getModuleInfo();
    if (!info.initialized) {
      throw new Error('Generator should be initialized');
    }

    console.log(chalk.green(`   Module version: ${info.version}`));
  });

  // Test 2: Generate Test Secret
  await runner.test('Generate Test Secret', async () => {
    const generator = new LocalKFragGenerator('node');
    await generator.initialize();

    const secret = await generator.generateTestSecret(32);

    if (!(secret instanceof Uint8Array)) {
      throw new Error('Secret should be Uint8Array');
    }

    if (secret.length !== 32) {
      throw new Error('Secret should be 32 bytes');
    }

    console.log(chalk.green(`   Secret generated: ${secret.length} bytes`));
  });

  // Test 3: Validate Secret
  await runner.test('Validate Secret', async () => {
    const generator = new LocalKFragGenerator('node');
    await generator.initialize();

    // Test valid secret
    const validSecret = new Uint8Array(32).fill(42);
    const isValid = generator.validateSecret(validSecret);

    if (!isValid) {
      throw new Error('Valid secret should pass validation');
    }

    // Test invalid secret (too short)
    const invalidSecret = new Uint8Array(8).fill(42);
    const isInvalid = generator.validateSecret(invalidSecret);

    if (isInvalid) {
      throw new Error('Invalid secret should fail validation');
    }

    console.log(chalk.green('   Secret validation working correctly'));
  });

  // Test 4: Generate kFrags (Mock or Real)
  await runner.test('Generate kFrags', async () => {
    const generator = new LocalKFragGenerator('node');
    await generator.initialize();

    const secret = 'test-secret-for-kfrag-generation';
    const threshold = 2;
    const totalShares = 3;

    const result = await generator.generateKFrags(secret, threshold, totalShares);

    if (!result.kfrags || !Array.isArray(result.kfrags)) {
      throw new Error('Result should contain kfrags array');
    }

    if (result.kfrags.length !== totalShares) {
      throw new Error(`Should generate ${totalShares} kFrags, got ${result.kfrags.length}`);
    }

    if (result.threshold !== threshold) {
      throw new Error(`Threshold should be ${threshold}, got ${result.threshold}`);
    }

    console.log(chalk.green(`   Generated ${result.kfrags.length} kFrags`));
    console.log(chalk.green(`   Capsule ID: ${result.capsuleId}`));
    console.log(chalk.green(`   Source: ${result.source}`));
  });

  // Test 5: Input Validation
  await runner.test('Input Validation', async () => {
    const generator = new LocalKFragGenerator('node');
    await generator.initialize();

    // Test empty secret
    try {
      await generator.generateKFrags('', 2, 3);
      throw new Error('Should fail with empty secret');
    } catch (error) {
      if (error.message.includes('Should fail')) {
        throw error;
      }
      // Expected error
    }

    // Test threshold > totalShares
    try {
      await generator.generateKFrags('test-secret', 5, 3);
      throw new Error('Should fail with threshold > totalShares');
    } catch (error) {
      if (error.message.includes('Should fail')) {
        throw error;
      }
      // Expected error
    }

    console.log(chalk.green('   Input validation working correctly'));
  });

  // Test 6: Different Secret Types
  await runner.test('Different Secret Types', async () => {
    const generator = new LocalKFragGenerator('node');
    await generator.initialize();

    const threshold = 2;
    const totalShares = 3;

    // Test string secret
    const stringResult = await generator.generateKFrags('string-secret', threshold, totalShares);
    if (stringResult.kfrags.length !== totalShares) {
      throw new Error('String secret failed');
    }

    // Test Uint8Array secret
    const arraySecret = new Uint8Array(32).fill(123);
    const arrayResult = await generator.generateKFrags(arraySecret, threshold, totalShares);
    if (arrayResult.kfrags.length !== totalShares) {
      throw new Error('Uint8Array secret failed');
    }

    console.log(chalk.green('   Different secret types handled correctly'));
  });

  runner.summary();

  // Additional info
  console.log(chalk.cyan('\\n📋 WASM-pack Status:'));
  const generator = new LocalKFragGenerator('node');
  await generator.initialize();
  const info = generator.getModuleInfo();

  if (info.version?.includes('Mock')) {
    console.log(chalk.yellow('   🔄 Running in mock mode (WASM package needs fixing)'));
    console.log(chalk.yellow('   🦀 Real WASM crypto operations pending environment fix'));
  } else {
    console.log(chalk.green('   ✅ Real WASM crypto operations working'));
  }

  console.log(chalk.gray(`   Target: ${info.target}`));
  console.log(chalk.gray(`   WASM Support: ${info.wasmSupported}`));
  console.log(chalk.gray(`   Version: ${info.version}`));

  process.exit(runner.failed > 0 ? 1 : 0);
}

main().catch(error => {
  console.error(chalk.red('WASM-pack test runner failed:'), error);
  process.exit(1);
});