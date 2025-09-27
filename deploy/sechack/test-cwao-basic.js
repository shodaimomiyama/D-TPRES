#!/usr/bin/env node

/**
 * D-TPRES CWAO Basic Integration Test
 * Phase 2A: Test basic CWAO functionality
 */

import { D_TPRES_CWAO } from './d-tpres-cwao.js';
import chalk from 'chalk';

class TestRunner {
  constructor() {
    this.passed = 0;
    this.failed = 0;
  }

  async test(name, fn) {
    try {
      console.log(chalk.cyan(`🧪 Testing: ${name}`));
      await fn();
      console.log(chalk.green(`✅ PASS: ${name}`));
      this.passed++;
    } catch (error) {
      console.log(chalk.red(`❌ FAIL: ${name}`));
      console.log(chalk.red(`   Error: ${error.message}`));
      this.failed++;
    }
  }

  summary() {
    const total = this.passed + this.failed;
    console.log(chalk.cyan('\n📊 Test Summary:'));
    console.log(chalk.green(`✅ Passed: ${this.passed}/${total}`));
    console.log(chalk.red(`❌ Failed: ${this.failed}/${total}`));

    if (this.failed === 0) {
      console.log(chalk.green('🎉 All tests passed!'));
    } else {
      console.log(chalk.red('💥 Some tests failed'));
    }
  }
}

async function main() {
  console.log(chalk.blue.bold('🚀 D-TPRES CWAO Basic Integration Test\n'));

  const runner = new TestRunner();

  // Test 1: Constructor
  await runner.test('D_TPRES_CWAO Constructor', async () => {
    const dtpres = new D_TPRES_CWAO({
      walletPath: './wallet.json'
    });

    if (!dtpres.config.protocol) {
      throw new Error('Protocol not set');
    }

    if (dtpres.config.protocol !== 'ao') {
      throw new Error('Wrong protocol');
    }
  });

  // Test 2: Deployment Summary (before init)
  await runner.test('Deployment Summary (Uninitialized)', async () => {
    const dtpres = new D_TPRES_CWAO();
    const summary = dtpres.getDeploymentSummary();

    if (summary.initialized !== false) {
      throw new Error('Should not be initialized');
    }

    if (summary.moduleId !== null) {
      throw new Error('Module ID should be null');
    }
  });

  // Test 3: Error handling (uninitialized)
  await runner.test('Error Handling (Uninitialized)', async () => {
    const dtpres = new D_TPRES_CWAO();

    try {
      await dtpres.deployModule();
      throw new Error('Should have thrown error');
    } catch (error) {
      if (!error.message.includes('not initialized')) {
        throw new Error('Wrong error message');
      }
    }
  });

  // Test 4: Initialize with missing wallet (expected to fail)
  await runner.test('Initialize with Missing Wallet', async () => {
    const dtpres = new D_TPRES_CWAO({
      walletPath: './nonexistent-wallet.json'
    });

    try {
      await dtpres.initialize();
      throw new Error('Should have thrown error for missing wallet');
    } catch (error) {
      if (!error.message.includes('not found') && error.code !== 'ENOENT') {
        throw new Error('Wrong error for missing wallet');
      }
    }
  });

  // Test 5: Configuration validation
  await runner.test('Configuration Validation', async () => {
    const customConfig = {
      protocol: "custom",
      variant: "custom_variant",
      walletPath: './test-wallet.json'
    };

    const dtpres = new D_TPRES_CWAO(customConfig);

    if (dtpres.config.protocol !== 'custom') {
      throw new Error('Custom protocol not set');
    }

    if (dtpres.config.variant !== 'custom_variant') {
      throw new Error('Custom variant not set');
    }
  });

  // Test 6: Method availability
  await runner.test('Method Availability', async () => {
    const dtpres = new D_TPRES_CWAO();

    const requiredMethods = [
      'initialize',
      'loadWallet',
      'deployModule',
      'spawnOwnerProcess',
      'spawnHolderProcesses',
      'sendKFragsToOwner',
      'transferKFragsToHolders',
      'queryProcessState',
      'getDeploymentSummary'
    ];

    for (const method of requiredMethods) {
      if (typeof dtpres[method] !== 'function') {
        throw new Error(`Method ${method} not available`);
      }
    }
  });

  runner.summary();
  process.exit(runner.failed > 0 ? 1 : 0);
}

main().catch(error => {
  console.error(chalk.red('Test runner failed:'), error);
  process.exit(1);
});