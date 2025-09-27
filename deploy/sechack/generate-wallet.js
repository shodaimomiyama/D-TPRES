#!/usr/bin/env node

/**
 * D-TPRES Wallet Generator
 * Generates Arweave wallets for D-TPRES deployment
 */

import Arweave from 'arweave';
import { writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import chalk from 'chalk';
import * as readline from 'readline/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

class WalletGenerator {
  constructor() {
    this.arweave = null;
  }

  /**
   * Initialize Arweave connection
   */
  async initialize(network = 'local') {
    console.log(chalk.blue(`🔧 Initializing Arweave connection (${network})...`));

    if (network === 'local') {
      this.arweave = Arweave.init({
        host: 'localhost',
        port: 1984,
        protocol: 'http'
      });
    } else if (network === 'testnet') {
      this.arweave = Arweave.init({
        host: 'arweave.net',
        port: 443,
        protocol: 'https'
      });
    } else {
      this.arweave = Arweave.init({
        host: 'arweave.net',
        port: 443,
        protocol: 'https'
      });
    }

    console.log(chalk.green(`✅ Arweave connection initialized for ${network}`));
  }

  /**
   * Generate a new wallet
   */
  async generateWallet(filename = 'wallet.json', network = 'local') {
    console.log(chalk.blue('🔑 Generating new Arweave wallet...'));

    try {
      // Generate new wallet
      const wallet = await this.arweave.wallets.generate();
      const address = await this.arweave.wallets.jwkToAddress(wallet);

      console.log(chalk.green(`✅ Wallet generated successfully!`));
      console.log(chalk.cyan(`📍 Address: ${address}`));

      // Mint tokens for local development
      if (network === 'local') {
        console.log(chalk.blue('💰 Minting tokens for local development...'));
        try {
          await this.arweave.api.get(`mint/${address}/10000000000000000`);
          console.log(chalk.green('✅ 10 AR tokens minted successfully'));
        } catch (error) {
          console.log(chalk.yellow(`⚠️  Failed to mint tokens: ${error.message}`));
          console.log(chalk.gray('   This is normal if ArLocal is not running'));
        }
      }

      return { wallet, address };

    } catch (error) {
      console.error(chalk.red(`❌ Failed to generate wallet: ${error.message}`));
      throw error;
    }
  }

  /**
   * Save wallet to file
   */
  saveWallet(wallet, filename, overwrite = false) {
    const walletPath = resolve(__dirname, filename);

    // Check if file exists
    if (existsSync(walletPath) && !overwrite) {
      throw new Error(`Wallet file already exists: ${walletPath}`);
    }

    // Save wallet
    writeFileSync(walletPath, JSON.stringify(wallet, null, 2));
    console.log(chalk.green(`💾 Wallet saved to: ${walletPath}`));

    return walletPath;
  }

  /**
   * Get wallet balance
   */
  async getBalance(address) {
    try {
      const balance = await this.arweave.wallets.getBalance(address);
      const ar = this.arweave.ar.winstonToAr(balance);
      return ar;
    } catch (error) {
      console.log(chalk.yellow(`⚠️  Failed to get balance: ${error.message}`));
      return '0';
    }
  }

  /**
   * Display wallet information
   */
  async displayWalletInfo(address, network) {
    console.log(chalk.cyan('\n📊 Wallet Information:'));
    console.log(chalk.gray(`   Address: ${address}`));
    console.log(chalk.gray(`   Network: ${network}`));

    const balance = await this.getBalance(address);
    console.log(chalk.gray(`   Balance: ${balance} AR`));

    if (network === 'local') {
      console.log(chalk.gray('   Note: Local development wallet with minted tokens'));
    } else {
      console.log(chalk.yellow('   ⚠️  Production wallet - ensure sufficient AR tokens for deployment'));
    }
  }
}

/**
 * Interactive wallet generation
 */
async function interactiveGeneration() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  console.log(chalk.blue.bold('🚀 D-TPRES Wallet Generator\n'));

  try {
    // Ask for network
    const network = await rl.question(chalk.cyan('Select network (local/testnet/mainnet) [local]: '));
    const selectedNetwork = network.trim() || 'local';

    // Ask for filename
    const filename = await rl.question(chalk.cyan('Wallet filename [wallet.json]: '));
    const selectedFilename = filename.trim() || 'wallet.json';

    // Check if file exists
    const walletPath = resolve(__dirname, selectedFilename);
    if (existsSync(walletPath)) {
      const overwrite = await rl.question(chalk.yellow(`Wallet file ${selectedFilename} already exists. Overwrite? (y/N): `));
      if (overwrite.toLowerCase() !== 'y') {
        console.log(chalk.gray('Operation cancelled.'));
        rl.close();
        return;
      }
    }

    rl.close();

    // Generate wallet
    const generator = new WalletGenerator();
    await generator.initialize(selectedNetwork);

    const { wallet, address } = await generator.generateWallet(selectedFilename, selectedNetwork);

    // Save wallet
    generator.saveWallet(wallet, selectedFilename, true);

    // Display information
    await generator.displayWalletInfo(address, selectedNetwork);

    // Network-specific instructions
    if (selectedNetwork === 'local') {
      console.log(chalk.cyan('\n💡 Next Steps (Local):'));
      console.log(chalk.gray('   1. Ensure ArLocal is running: npm run setup:local'));
      console.log(chalk.gray('   2. Deploy locally: npm run deploy:local'));
    } else {
      console.log(chalk.cyan('\n💡 Next Steps (Production):'));
      console.log(chalk.gray('   1. Fund your wallet with AR tokens'));
      console.log(chalk.gray('   2. Configure .env file for your network'));
      console.log(chalk.gray('   3. Deploy: npm run deploy:testnet or npm run deploy:mainnet'));
    }

    console.log(chalk.green('\n🎉 Wallet generation completed!'));

  } catch (error) {
    console.error(chalk.red('❌ Wallet generation failed:'), error);
    rl.close();
    process.exit(1);
  }
}

/**
 * Command line wallet generation
 */
async function commandLineGeneration() {
  const args = process.argv.slice(2);
  const network = args[0] || 'local';
  const filename = args[1] || 'wallet.json';

  console.log(chalk.blue.bold('🚀 D-TPRES Wallet Generator (CLI Mode)\n'));

  try {
    const generator = new WalletGenerator();
    await generator.initialize(network);

    const { wallet, address } = await generator.generateWallet(filename, network);

    // Save wallet
    generator.saveWallet(wallet, filename, true);

    // Display information
    await generator.displayWalletInfo(address, network);

    console.log(chalk.green('\n🎉 Wallet generation completed!'));

  } catch (error) {
    if (error.message.includes('already exists')) {
      console.error(chalk.red('❌ Wallet file already exists. Use --force to overwrite or choose different filename.'));
    } else {
      console.error(chalk.red('❌ Wallet generation failed:'), error);
    }
    process.exit(1);
  }
}

/**
 * Main function
 */
async function main() {
  const args = process.argv.slice(2);

  // Check for help flag
  if (args.includes('--help') || args.includes('-h')) {
    console.log(chalk.blue.bold('D-TPRES Wallet Generator\n'));
    console.log('Usage:');
    console.log('  node generate-wallet.js                    # Interactive mode');
    console.log('  node generate-wallet.js <network>          # CLI mode');
    console.log('  node generate-wallet.js <network> <file>   # CLI mode with custom filename');
    console.log('');
    console.log('Networks:');
    console.log('  local    - Local development (default)');
    console.log('  testnet  - Arweave testnet');
    console.log('  mainnet  - Arweave mainnet');
    console.log('');
    console.log('Examples:');
    console.log('  node generate-wallet.js');
    console.log('  node generate-wallet.js local');
    console.log('  node generate-wallet.js mainnet production-wallet.json');
    return;
  }

  // Run interactive mode if no arguments, otherwise CLI mode
  if (args.length === 0) {
    await interactiveGeneration();
  } else {
    await commandLineGeneration();
  }
}

// Run if this file is executed directly
if (process.argv[1] === __filename) {
  main().catch(error => {
    console.error(chalk.red('Wallet generator failed:'), error);
    process.exit(1);
  });
}

export { WalletGenerator };
export default WalletGenerator;