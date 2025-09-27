#!/usr/bin/env node

/**
 * D-TPRES Local Deployment Script
 * Deploy D-TPRES to local AO environment using CWAO SDK
 */

import { D_TPRES_CWAO } from './d-tpres-cwao.js';
import { existsSync } from 'fs';
import { resolve } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import chalk from 'chalk';
import dotenv from 'dotenv';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment variables
dotenv.config();

class LocalDeployment {
  constructor() {
    this.dtpres = null;
    this.config = {
      network: process.env.NETWORK || 'local',
      walletPath: process.env.WALLET_PATH || './wallet.json',
      modulePath: process.env.MODULE_PATH || './output/d_tpres.wasm',
      holderCount: parseInt(process.env.HOLDER_COUNT) || 3,
      threshold: parseInt(process.env.THRESHOLD) || 2,
      totalShares: parseInt(process.env.TOTAL_SHARES) || 3,
      debugMode: process.env.DEBUG_MODE === 'true'
    };
  }

  /**
   * Validate prerequisites
   */
  validatePrerequisites() {
    console.log(chalk.blue('🔍 Validating prerequisites...'));

    const issues = [];

    // Check wallet file
    const walletPath = resolve(__dirname, this.config.walletPath);
    if (!existsSync(walletPath)) {
      issues.push(`Wallet file not found: ${walletPath}`);
    }

    // Check WASM module
    const modulePath = resolve(__dirname, this.config.modulePath);
    if (!existsSync(modulePath)) {
      issues.push(`WASM module not found: ${modulePath}`);
    }

    // Check threshold configuration
    if (this.config.threshold > this.config.totalShares) {
      issues.push(`Threshold (${this.config.threshold}) cannot be greater than total shares (${this.config.totalShares})`);
    }

    if (issues.length > 0) {
      console.log(chalk.red('❌ Prerequisites validation failed:'));
      issues.forEach(issue => console.log(chalk.red(`   - ${issue}`)));
      console.log(chalk.cyan('\n💡 Solutions:'));
      if (issues.some(i => i.includes('Wallet file'))) {
        console.log(chalk.gray('   - Generate wallet: npm run wallet:generate'));
      }
      if (issues.some(i => i.includes('WASM module'))) {
        console.log(chalk.gray('   - Build WASM: npm run build-wasm'));
      }
      throw new Error('Prerequisites validation failed');
    }

    console.log(chalk.green('✅ Prerequisites validation passed'));
  }

  /**
   * Initialize D_TPRES_CWAO instance
   */
  async initialize() {
    console.log(chalk.blue('🚀 Initializing D-TPRES CWAO...'));

    try {
      // Create D_TPRES_CWAO instance with local configuration
      this.dtpres = new D_TPRES_CWAO({
        walletPath: this.config.walletPath,
        protocol: 'ao',
        variant: 'ao.TN.1'
      });

      // Initialize
      await this.dtpres.initialize();

      console.log(chalk.green('✅ D-TPRES CWAO initialized successfully'));

    } catch (error) {
      console.error(chalk.red('❌ Failed to initialize D-TPRES CWAO:'), error.message);
      throw error;
    }
  }

  /**
   * Deploy full workflow
   */
  async deployFullWorkflow() {
    console.log(chalk.blue('🚀 Starting full D-TPRES deployment workflow...'));

    try {
      const modulePath = resolve(__dirname, this.config.modulePath);

      // Deploy full workflow
      const deployment = await this.dtpres.deployFullWorkflow(
        modulePath,
        this.config.holderCount
      );

      console.log(chalk.green('✅ Deployment completed successfully!'));
      console.log(chalk.cyan('\n📋 Deployment Summary:'));
      console.log(chalk.gray(`   Module ID: ${deployment.moduleId}`));
      console.log(chalk.gray(`   Owner Process: ${deployment.processes.owner}`));
      console.log(chalk.gray(`   Holder Processes: ${deployment.processes.holders.length}`));

      deployment.processes.holders.forEach((holder, index) => {
        console.log(chalk.gray(`     Holder ${index + 1}: ${holder}`));
      });

      return deployment;

    } catch (error) {
      console.error(chalk.red('❌ Deployment failed:'), error.message);
      throw error;
    }
  }

  /**
   * Execute kFrag workflow
   */
  async executeKFragWorkflow(secret = 'demo-secret-for-d-tpres') {
    console.log(chalk.blue('🔐 Executing kFrag workflow...'));

    try {
      const result = await this.dtpres.executeKFragWorkflow(
        secret,
        this.config.threshold,
        this.config.totalShares
      );

      console.log(chalk.green('✅ kFrag workflow completed successfully!'));
      console.log(chalk.cyan('\n📋 kFrag Summary:'));
      console.log(chalk.gray(`   Generated kFrags: ${result.localResult.kfrags.length}`));
      console.log(chalk.gray(`   Threshold: ${result.localResult.threshold}`));
      console.log(chalk.gray(`   Total Shares: ${result.localResult.sharesCount}`));
      console.log(chalk.gray(`   Capsule ID: ${result.localResult.capsuleId}`));

      return result;

    } catch (error) {
      console.error(chalk.red('❌ kFrag workflow failed:'), error.message);
      throw error;
    }
  }

  /**
   * Query deployment status
   */
  async queryDeploymentStatus() {
    console.log(chalk.blue('📊 Querying deployment status...'));

    try {
      // Get deployment summary
      const summary = this.dtpres.getDeploymentSummary();

      console.log(chalk.cyan('\n📋 Current Status:'));
      console.log(chalk.gray(`   Initialized: ${summary.initialized}`));
      console.log(chalk.gray(`   Module ID: ${summary.moduleId || 'Not deployed'}`));
      console.log(chalk.gray(`   Owner Process: ${summary.processes.owner || 'Not spawned'}`));
      console.log(chalk.gray(`   Holder Count: ${summary.processes.holderCount}`));

      // Query process state if available
      if (summary.processes.owner) {
        try {
          const state = await this.dtpres.queryProcessState();
          console.log(chalk.gray(`   Process State: Available`));
          if (this.config.debugMode) {
            console.log(chalk.gray(`   State Details: ${JSON.stringify(state, null, 2)}`));
          }
        } catch (error) {
          console.log(chalk.yellow(`   Process State: Query failed (${error.message})`));
        }
      }

      return summary;

    } catch (error) {
      console.error(chalk.red('❌ Status query failed:'), error.message);
      throw error;
    }
  }

  /**
   * Display configuration
   */
  displayConfiguration() {
    console.log(chalk.cyan('\n📋 Deployment Configuration:'));
    console.log(chalk.gray(`   Network: ${this.config.network}`));
    console.log(chalk.gray(`   Wallet: ${this.config.walletPath}`));
    console.log(chalk.gray(`   WASM Module: ${this.config.modulePath}`));
    console.log(chalk.gray(`   Holder Count: ${this.config.holderCount}`));
    console.log(chalk.gray(`   Threshold: ${this.config.threshold}`));
    console.log(chalk.gray(`   Total Shares: ${this.config.totalShares}`));
    console.log(chalk.gray(`   Debug Mode: ${this.config.debugMode}`));
  }
}

/**
 * Main deployment function
 */
async function main() {
  console.log(chalk.blue.bold('🚀 D-TPRES Local Deployment\n'));

  const deployment = new LocalDeployment();

  try {
    // Display configuration
    deployment.displayConfiguration();

    // Validate prerequisites
    deployment.validatePrerequisites();

    // Initialize
    await deployment.initialize();

    // Deploy full workflow
    const deployResult = await deployment.deployFullWorkflow();

    // Execute kFrag workflow
    const kfragResult = await deployment.executeKFragWorkflow();

    // Query final status
    await deployment.queryDeploymentStatus();

    console.log(chalk.green.bold('\n🎉 D-TPRES Local Deployment Completed Successfully!'));

    console.log(chalk.cyan('\n💡 Next Steps:'));
    console.log(chalk.gray('   - Test kFrag operations: npm run test-e2e'));
    console.log(chalk.gray('   - Verify deployment: npm run verify'));
    console.log(chalk.gray('   - View process states in AO explorer'));

    console.log(chalk.cyan('\n🌐 Local AO Environment:'));
    console.log(chalk.gray('   - ArWeave: http://localhost:1984'));
    console.log(chalk.gray('   - GraphQL: http://localhost:1984/graphql'));
    console.log(chalk.gray('   - MU: http://localhost:1985'));
    console.log(chalk.gray('   - SU: http://localhost:1986'));
    console.log(chalk.gray('   - CU: http://localhost:1987'));

    process.exit(0);

  } catch (error) {
    console.error(chalk.red('\n💥 Deployment failed:'), error.message);

    if (error.message.includes('wallet') || error.message.includes('Wallet')) {
      console.log(chalk.cyan('\n💡 Wallet Issue Solutions:'));
      console.log(chalk.gray('   - Generate wallet: npm run wallet:generate'));
      console.log(chalk.gray('   - Check wallet path in .env file'));
    }

    if (error.message.includes('WASM') || error.message.includes('module')) {
      console.log(chalk.cyan('\n💡 WASM Module Solutions:'));
      console.log(chalk.gray('   - Build WASM: npm run build-wasm'));
      console.log(chalk.gray('   - Check module path in .env file'));
    }

    if (error.message.includes('network') || error.message.includes('connection')) {
      console.log(chalk.cyan('\n💡 Network Issue Solutions:'));
      console.log(chalk.gray('   - Start local AO: npm run setup:local'));
      console.log(chalk.gray('   - Check if ArLocal is running on port 1984'));
    }

    process.exit(1);
  }
}

// Run if this file is executed directly
if (process.argv[1] === __filename) {
  main();
}

export { LocalDeployment };
export default LocalDeployment;