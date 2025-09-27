#!/usr/bin/env node

/**
 * D-TPRES Local AO Environment Setup
 * Sets up local ArWeave and AO Units (MU/SU/CU) for development
 */

import ArLocal from 'arlocal';
// ESM workaround for ArLocal
const ArLocalClass = ArLocal.default || ArLocal;
import Arweave from 'arweave';
import { MU, SU, CU } from 'cwao-units';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const CONFIG = {
  ports: {
    arweave: 1984,
    mu: 1985,
    su: 1986,
    cu: 1987
  },
  dirs: {
    root: resolve(__dirname, '../.cwao'),
    accounts: resolve(__dirname, '../.cwao/accounts'),
    db: resolve(__dirname, '../.cwao/db')
  },
  mint_amount: '10000000000000000' // 10 AR
};

class LocalAOEnvironment {
  constructor() {
    this.arLocal = null;
    this.arweave = null;
    this.wallets = {};
    this.units = {};
    this.running = false;
  }

  /**
   * Create necessary directories
   */
  async createDirectories() {
    console.log(chalk.blue('📁 Creating directories...'));

    for (const [name, dir] of Object.entries(CONFIG.dirs)) {
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
        console.log(chalk.green(`✅ Created directory: ${name} -> ${dir}`));
      } else {
        console.log(chalk.gray(`📂 Directory exists: ${name} -> ${dir}`));
      }
    }
  }

  /**
   * Generate or load wallet for a unit
   */
  async generateWallet(name) {
    const keyfile = resolve(CONFIG.dirs.accounts, `${name}.json`);

    let wallet = null;
    if (existsSync(keyfile)) {
      wallet = JSON.parse(readFileSync(keyfile, 'utf8'));
      const addr = await this.arweave.wallets.jwkToAddress(wallet);
      console.log(chalk.yellow(`🔄 [${name}] Wallet loaded: ${addr}`));
    } else {
      wallet = await this.arweave.wallets.generate();
      const addr = await this.arweave.wallets.jwkToAddress(wallet);

      // Mint tokens for development
      await this.arweave.api.get(`mint/${addr}/${CONFIG.mint_amount}`);

      writeFileSync(keyfile, JSON.stringify(wallet, null, 2));
      console.log(chalk.green(`✅ [${name}] Wallet generated: ${addr}`));
      console.log(chalk.green(`💰 Minted ${CONFIG.mint_amount} tokens`));
    }

    return wallet;
  }

  /**
   * Start ArLocal
   */
  async startArLocal() {
    console.log(chalk.blue('🚀 Starting ArLocal...'));

    this.arLocal = new ArLocalClass(
      CONFIG.ports.arweave,
      false,
      CONFIG.dirs.db,
      true
    );

    await this.arLocal.start();

    this.arweave = Arweave.init({
      host: 'localhost',
      port: CONFIG.ports.arweave,
      protocol: 'http'
    });

    console.log(chalk.green(`✅ ArLocal started on port ${CONFIG.ports.arweave}`));
  }

  /**
   * Generate wallets for all units
   */
  async generateWallets() {
    console.log(chalk.blue('🔑 Generating wallets for AO units...'));

    for (const unitName of ['mu', 'su', 'cu']) {
      this.wallets[unitName] = await this.generateWallet(unitName);
    }
  }

  /**
   * Start AO Units
   */
  async startAOUnits() {
    console.log(chalk.blue('🌐 Starting AO Units...'));

    // Start MU (Messenger Unit)
    console.log(chalk.cyan('📨 Starting MU (Messenger Unit)...'));
    this.units.mu = new MU({
      port: CONFIG.ports.mu,
      wallet: this.wallets.mu,
      arweave: this.arweave,
      protocol: 'ao',
      variant: 'ao.TN.1'
    });
    console.log(chalk.green(`✅ MU started on port ${CONFIG.ports.mu}`));

    // Start SU (Scheduler Unit)
    console.log(chalk.cyan('📅 Starting SU (Scheduler Unit)...'));
    this.units.su = new SU({
      port: CONFIG.ports.su,
      wallet: this.wallets.su,
      arweave: this.arweave,
      protocol: 'ao',
      variant: 'ao.TN.1'
    });
    console.log(chalk.green(`✅ SU started on port ${CONFIG.ports.su}`));

    // Start CU (Compute Unit)
    console.log(chalk.cyan('💻 Starting CU (Compute Unit)...'));
    this.units.cu = new CU({
      port: CONFIG.ports.cu,
      wallet: this.wallets.cu,
      arweave: this.arweave,
      protocol: 'ao',
      variant: 'ao.TN.1'
    });
    console.log(chalk.green(`✅ CU started on port ${CONFIG.ports.cu}`));
  }

  /**
   * Start complete local AO environment
   */
  async start() {
    try {
      console.log(chalk.blue.bold('🚀 D-TPRES Local AO Environment Setup\n'));

      await this.createDirectories();
      await this.startArLocal();
      await this.generateWallets();
      await this.startAOUnits();

      this.running = true;

      console.log(chalk.green.bold('\n🎉 Local AO Environment Ready!'));
      console.log(chalk.cyan('\n📋 Environment Summary:'));
      console.log(chalk.gray(`   ArWeave:  http://localhost:${CONFIG.ports.arweave}`));
      console.log(chalk.gray(`   GraphQL:  http://localhost:${CONFIG.ports.arweave}/graphql`));
      console.log(chalk.gray(`   MU:       http://localhost:${CONFIG.ports.mu}`));
      console.log(chalk.gray(`   SU:       http://localhost:${CONFIG.ports.su}`));
      console.log(chalk.gray(`   CU:       http://localhost:${CONFIG.ports.cu}`));

      console.log(chalk.cyan('\n🔑 Wallet Addresses:'));
      for (const [name, wallet] of Object.entries(this.wallets)) {
        const addr = await this.arweave.wallets.jwkToAddress(wallet);
        console.log(chalk.gray(`   ${name.toUpperCase()}: ${addr}`));
      }

      console.log(chalk.cyan('\n💡 Usage:'));
      console.log(chalk.gray('   - Use these URLs in your .env file'));
      console.log(chalk.gray('   - Generate your deployment wallet: npm run wallet:generate'));
      console.log(chalk.gray('   - Deploy locally: npm run deploy:local'));
      console.log(chalk.gray('   - Press Ctrl+C to stop all services'));

      // Save configuration for other scripts
      await this.saveConfiguration();

      return this.getEnvironmentConfig();

    } catch (error) {
      console.error(chalk.red('❌ Failed to start local AO environment:'), error);
      await this.stop();
      throw error;
    }
  }

  /**
   * Save configuration for other scripts
   */
  async saveConfiguration() {
    const config = this.getEnvironmentConfig();
    const configPath = resolve(__dirname, '.local-ao-config.json');
    writeFileSync(configPath, JSON.stringify(config, null, 2));
    console.log(chalk.gray(`\n💾 Configuration saved to: ${configPath}`));

    // Also create .env.local file
    const envContent = `# D-TPRES Local AO Environment - AUTO GENERATED
NETWORK=local
PROTOCOL=ao
VARIANT=ao.TN.1

# AO Network URLs
ARWEAVE_URL=http://localhost:${CONFIG.ports.arweave}
GRAPHQL_URL=http://localhost:${CONFIG.ports.arweave}/graphql
MU_URL=http://localhost:${CONFIG.ports.mu}
SU_URL=http://localhost:${CONFIG.ports.su}
CU_URL=http://localhost:${CONFIG.ports.cu}

# Wallet Configuration
WALLET_PATH=./wallet.json

# Deployment Configuration
HOLDER_COUNT=3
THRESHOLD=2
TOTAL_SHARES=3
MODULE_PATH=./output/d_tpres.wasm

# Debug Configuration
DEBUG_MODE=true
VERBOSE_LOGGING=true
`;

    const envPath = resolve(__dirname, '.env.local');
    writeFileSync(envPath, envContent);
    console.log(chalk.gray(`💾 Environment file saved to: ${envPath}`));
  }

  /**
   * Stop all services
   */
  async stop() {
    if (!this.running) return;

    console.log(chalk.blue('\n🛑 Stopping Local AO Environment...'));

    // Stop AO Units
    for (const [name, unit] of Object.entries(this.units)) {
      if (unit && typeof unit.stop === 'function') {
        try {
          unit.stop();
          console.log(chalk.yellow(`📴 ${name.toUpperCase()} stopped`));
        } catch (error) {
          console.log(chalk.red(`❌ Error stopping ${name.toUpperCase()}: ${error.message}`));
        }
      }
    }

    // Stop ArLocal
    if (this.arLocal) {
      try {
        await this.arLocal.stop();
        console.log(chalk.yellow('📴 ArLocal stopped'));
      } catch (error) {
        console.log(chalk.red(`❌ Error stopping ArLocal: ${error.message}`));
      }
    }

    this.running = false;
    console.log(chalk.green('✅ Local AO Environment stopped'));
  }

  /**
   * Get environment configuration for deployment
   */
  getEnvironmentConfig() {
    return {
      network: 'local',
      protocol: 'ao',
      variant: 'ao.TN.1',
      urls: {
        arweave: `http://localhost:${CONFIG.ports.arweave}`,
        graphql: `http://localhost:${CONFIG.ports.arweave}/graphql`,
        mu: `http://localhost:${CONFIG.ports.mu}`,
        su: `http://localhost:${CONFIG.ports.su}`,
        cu: `http://localhost:${CONFIG.ports.cu}`
      },
      ports: CONFIG.ports,
      walletAddresses: {},
      timestamp: Date.now()
    };
  }
}

// Graceful shutdown handling
const environment = new LocalAOEnvironment();

process.on('SIGINT', async () => {
  console.log(chalk.blue('\n\n🛑 Received SIGINT, shutting down gracefully...'));
  await environment.stop();
  process.exit(0);
});

process.on('SIGTERM', async () => {
  console.log(chalk.blue('\n\n🛑 Received SIGTERM, shutting down gracefully...'));
  await environment.stop();
  process.exit(0);
});

// Error handling
process.on('uncaughtException', async (error) => {
  console.error(chalk.red('\n💥 Uncaught Exception:'), error);
  await environment.stop();
  process.exit(1);
});

process.on('unhandledRejection', async (reason, promise) => {
  console.error(chalk.red('\n💥 Unhandled Rejection at:'), promise, 'reason:', reason);
  await environment.stop();
  process.exit(1);
});

// Start environment if run directly
async function main() {
  try {
    const config = await environment.start();

    console.log(chalk.cyan('\n⏳ Environment is running. Press Ctrl+C to stop.\n'));

    // Keep process alive
    await new Promise((resolve) => {
      // Process will be terminated by signal handlers
    });

  } catch (error) {
    console.error(chalk.red('Failed to start environment:'), error);
    process.exit(1);
  }
}

// Run if this file is executed directly
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}

export { LocalAOEnvironment, CONFIG };
export default LocalAOEnvironment;