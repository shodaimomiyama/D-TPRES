#!/usr/bin/env node

/**
 * D-TPRES CWAO Integration Class
 * Phase 2A: Basic CWAO SDK integration for D-TPRES
 */

import { CWAO } from 'cwao';
import fs from 'fs/promises';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join, resolve } from 'path';
import chalk from 'chalk';
import Arweave from 'arweave';
import bech32Pkg from 'bech32';
import base64url from 'base64url';

const { bech32 } = bech32Pkg;

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Local AO Network Configuration
const LOCAL_NETWORK = {
  host: "localhost",
  port: 1984,
  protocol: "http",
};

// Load local AO configuration
const loadLocalConfig = () => {
  try {
    const configPath = resolve(__dirname, '.local-ao-config.json');
    const configData = readFileSync(configPath, 'utf8');
    return JSON.parse(configData);
  } catch (error) {
    Logger.warning('Local AO config not found, using defaults');
    return null;
  }
};

// Utility functions
const sleep = x => new Promise(res => setTimeout(() => res(), x));

function toBech32(arweaveAddress, prefix = "ao") {
  const decodedBytes = base64url.toBuffer(arweaveAddress);
  const words = bech32.toWords(decodedBytes);
  const bech32Address = bech32.encode(prefix, words);
  return bech32Address;
}

/**
 * Logger utility for CWAO operations
 */
class Logger {
  static info(msg) { console.log(chalk.blue('ℹ'), msg); }
  static success(msg) { console.log(chalk.green('✅'), msg); }
  static warning(msg) { console.log(chalk.yellow('⚠️'), msg); }
  static error(msg) { console.log(chalk.red('❌'), msg); }
  static crypto(msg) { console.log(chalk.magenta('🔐'), msg); }
  static network(msg) { console.log(chalk.cyan('🌐'), msg); }
}

/**
 * Main D-TPRES CWAO Integration Class
 */
export class D_TPRES_CWAO {
  constructor(config = {}) {
    // Load local AO configuration
    const localConfig = loadLocalConfig();

    this.config = {
      protocol: "ao",
      variant: "ao.TN.1",
      walletPath: config.walletPath || './wallet.json',
      network: 'local',
      ...config
    };

    this.cwao = null;
    this.wallet = null;
    this.arweave = null;
    this.moduleId = null;
    this.localConfig = localConfig;
    this.processes = {
      owner: null,
      holders: []
    };

    this.initialized = false;
  }

  /**
   * Initialize the CWAO instance with wallet
   */
  async initialize() {
    Logger.info('Initializing D-TPRES CWAO integration for local AO...');

    try {
      // Initialize Arweave for local network
      this.arweave = Arweave.init(LOCAL_NETWORK);
      Logger.info('Local Arweave instance initialized');

      // Load or generate wallet
      await this.loadWallet();

      // Initialize CWAO with local configuration
      this.cwao = new CWAO({
        protocol: this.config.protocol,
        variant: this.config.variant,
        wallet: this.wallet,
        arweave: this.arweave
      });

      // Set Scheduler Unit URL for local AO
      const suUrl = this.localConfig?.urls?.su || 'http://localhost:1986';
      try {
        await this.cwao.setSU({ url: suUrl });
        Logger.info(`Scheduler Unit configured: ${suUrl}`);
      } catch (suError) {
        Logger.warning(`Failed to set SU URL (${suUrl}): ${suError.message}`);
        Logger.info('CWAO will proceed without explicit SU configuration');
      }

      this.initialized = true;
      Logger.success('CWAO integration initialized successfully for local AO');

      return true;
    } catch (error) {
      Logger.error(`Failed to initialize CWAO: ${error.message}`);
      throw error;
    }
  }

  /**
   * Load wallet from file or generate for local development
   */
  async loadWallet() {
    try {
      const walletPath = resolve(__dirname, this.config.walletPath);
      Logger.info(`Loading wallet from: ${walletPath}`);

      try {
        const walletData = await fs.readFile(walletPath, 'utf8');
        this.wallet = JSON.parse(walletData);

        const addr = await this.arweave.wallets.jwkToAddress(this.wallet);
        Logger.success(`Wallet loaded successfully: ${addr}`);

        return this.wallet;
      } catch (loadError) {
        if (loadError.code === 'ENOENT') {
          Logger.warning('Wallet file not found, generating new wallet for local development...');
          return await this.generateLocalWallet(walletPath);
        }
        throw loadError;
      }
    } catch (error) {
      Logger.error(`Failed to load wallet: ${error.message}`);
      throw error;
    }
  }

  /**
   * Generate wallet for local development with automatic token minting
   */
  async generateLocalWallet(walletPath) {
    try {
      Logger.info('Generating new wallet for local development...');

      this.wallet = await this.arweave.wallets.generate();
      const addr = await this.arweave.wallets.jwkToAddress(this.wallet);

      // Mint tokens for local development
      const mintAmount = '10000000000000000'; // 10 AR
      await this.arweave.api.get(`mint/${addr}/${mintAmount}`);
      Logger.success(`💰 Minted ${mintAmount} tokens for development`);

      // Save wallet to file for reuse
      await fs.writeFile(walletPath, JSON.stringify(this.wallet, null, 2));
      Logger.success(`Wallet generated and saved: ${addr}`);

      return this.wallet;
    } catch (error) {
      Logger.error(`Failed to generate local wallet: ${error.message}`);
      throw error;
    }
  }

  /**
   * Deploy WASM module to Arweave
   */
  async deployModule(wasmPath = null) {
    this.ensureInitialized();

    const defaultWasmPath = join(__dirname, 'output', 'd_tpres.wasm');
    const wasmFilePath = wasmPath || defaultWasmPath;

    Logger.info(`Deploying WASM module: ${wasmFilePath}`);

    try {
      // Read WASM binary
      const wasmBinary = await fs.readFile(wasmFilePath);
      Logger.info(`WASM binary size: ${wasmBinary.length} bytes`);

      // Deploy to Arweave via CWAO
      this.moduleId = await this.cwao.deploy(wasmBinary);

      Logger.success(`Module deployed successfully!`);
      Logger.success(`Module ID: ${this.moduleId}`);

      return this.moduleId;
    } catch (error) {
      Logger.error(`Failed to deploy module: ${error.message}`);
      throw error;
    }
  }

  /**
   * Instantiate Owner-Process
   */
  async spawnOwnerProcess(scheduler = null) {
    this.ensureInitialized();

    if (!this.moduleId) {
      throw new Error('Module not deployed. Call deployModule() first.');
    }

    Logger.network('Spawning Owner-Process...');

    try {
      // Use local scheduler if not provided
      const ownerAddr = await this.arweave.wallets.jwkToAddress(this.wallet);
      const schedulerAddr = scheduler || ownerAddr;
      const ownerAddr32 = toBech32(ownerAddr, "ao");

      const processId = await this.cwao.instantiate({
        module: this.moduleId,
        scheduler: schedulerAddr,
        input: {
          process_type: "owner",
          version: "0.1.0-mvp",
          timestamp: Date.now(),
          owner_address: ownerAddr32,
          initial_balances: [{ address: ownerAddr32, amount: "5000000" }],
          mint: {
            minter: ownerAddr32,
            cap: "1000000000",
          }
        }
      });

      this.processes.owner = processId;
      Logger.success(`Owner-Process spawned: ${processId}`);
      Logger.info(`Owner Address (bech32): ${ownerAddr32}`);

      return processId;
    } catch (error) {
      Logger.error(`Failed to spawn Owner-Process: ${error.message}`);
      throw error;
    }
  }

  /**
   * Instantiate Holder-Process(es)
   */
  async spawnHolderProcesses(count = 1, scheduler = null) {
    this.ensureInitialized();

    if (!this.moduleId) {
      throw new Error('Module not deployed. Call deployModule() first.');
    }

    Logger.network(`Spawning ${count} Holder-Process(es)...`);

    try {
      const holders = [];
      const ownerAddr = await this.arweave.wallets.jwkToAddress(this.wallet);
      const schedulerAddr = scheduler || ownerAddr;
      const ownerAddr32 = toBech32(ownerAddr, "ao");

      for (let i = 0; i < count; i++) {
        const processId = await this.cwao.instantiate({
          module: this.moduleId,
          scheduler: schedulerAddr,
          input: {
            process_type: "holder",
            index: i,
            version: "0.1.0-mvp",
            timestamp: Date.now(),
            owner_address: ownerAddr32,
            initial_balances: [{ address: ownerAddr32, amount: "1000000" }],
            mint: {
              minter: ownerAddr32,
              cap: "1000000000",
            }
          }
        });

        holders.push(processId);
        Logger.success(`Holder-Process ${i} spawned: ${processId}`);

        // Small delay between spawns for stability
        await sleep(100);
      }

      this.processes.holders = holders;
      return holders;
    } catch (error) {
      Logger.error(`Failed to spawn Holder-Processes: ${error.message}`);
      throw error;
    }
  }

  /**
   * Send kFrags to Owner-Process
   */
  async sendKFragsToOwner(kfrags) {
    this.ensureInitialized();

    if (!this.processes.owner) {
      throw new Error('Owner-Process not spawned. Call spawnOwnerProcess() first.');
    }

    Logger.crypto(`Sending ${kfrags.length} kFrags to Owner-Process...`);

    try {
      const result = await this.cwao.execute({
        process: this.processes.owner,
        action: "store-kfrags",
        input: {
          kfrags: kfrags,
          timestamp: Date.now(),
          source: "local-generation"
        }
      });

      Logger.success('kFrags sent to Owner-Process successfully');
      return result;
    } catch (error) {
      Logger.error(`Failed to send kFrags to Owner: ${error.message}`);
      throw error;
    }
  }

  /**
   * Instruct Owner-Process to transfer kFrags to Holders
   */
  async transferKFragsToHolders() {
    this.ensureInitialized();

    if (!this.processes.owner) {
      throw new Error('Owner-Process not spawned.');
    }
    if (this.processes.holders.length === 0) {
      throw new Error('No Holder-Processes spawned.');
    }

    Logger.network(`Transferring kFrags to ${this.processes.holders.length} Holder(s)...`);

    try {
      const result = await this.cwao.execute({
        process: this.processes.owner,
        action: "transfer-kfrags",
        input: {
          holder_processes: this.processes.holders,
          timestamp: Date.now()
        }
      });

      Logger.success('kFrags transferred to Holder-Processes successfully');
      return result;
    } catch (error) {
      Logger.error(`Failed to transfer kFrags: ${error.message}`);
      throw error;
    }
  }

  /**
   * Generate kFrags locally using WASM-pack
   */
  async generateKFragsLocal(secret, threshold = 2, totalShares = 3) {
    Logger.crypto('Generating kFrags locally with WASM...');

    try {
      // Dynamic import of LocalKFragGenerator
      const { LocalKFragGenerator } = await import('./local-kfrag-generator.js');

      // Create and initialize generator
      const generator = new LocalKFragGenerator('node');
      await generator.initialize();

      // Generate kFrags
      const result = await generator.generateKFrags(secret, threshold, totalShares);

      Logger.success(`Generated ${result.kfrags.length} kFrags locally`);
      return result;
    } catch (error) {
      Logger.error(`Local kFrag generation failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Full deployment workflow
   */
  async deployFullWorkflow(wasmPath = null, holderCount = 1, scheduler = null) {
    this.ensureInitialized();

    Logger.info('Starting full D-TPRES deployment workflow...');

    try {
      // Step 1: Deploy module
      const moduleId = await this.deployModule(wasmPath);

      // Step 2: Spawn Owner-Process
      const ownerProcess = await this.spawnOwnerProcess(scheduler);

      // Step 3: Spawn Holder-Process(es)
      const holderProcesses = await this.spawnHolderProcesses(holderCount, scheduler);

      Logger.success('Full deployment workflow completed successfully');

      return {
        moduleId,
        processes: {
          owner: ownerProcess,
          holders: holderProcesses
        }
      };
    } catch (error) {
      Logger.error(`Deployment workflow failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Complete E2E kFrag workflow
   */
  async executeKFragWorkflow(secret, threshold = 2, totalShares = 3) {
    this.ensureInitialized();

    if (!this.processes.owner) {
      throw new Error('Owner-Process not deployed. Run deployFullWorkflow() first.');
    }

    Logger.info('Executing complete kFrag workflow...');

    try {
      // Step 1: Generate kFrags locally
      const kfragResult = await this.generateKFragsLocal(secret, threshold, totalShares);

      // Step 2: Send to Owner-Process
      await this.sendKFragsToOwner(kfragResult.kfrags);

      // Step 3: Transfer to Holders
      await this.transferKFragsToHolders();

      Logger.success('Complete kFrag workflow executed successfully');

      return {
        localResult: kfragResult,
        processesInvolved: {
          owner: this.processes.owner,
          holders: this.processes.holders
        }
      };
    } catch (error) {
      Logger.error(`kFrag workflow failed: ${error.message}`);
      throw error;
    }
  }

  /**
   * Query process state with CW20 compatibility
   */
  async queryProcessState(processId = null) {
    this.ensureInitialized();

    const targetProcess = processId || this.processes.owner;
    if (!targetProcess) {
      throw new Error('No process ID specified and no default process available.');
    }

    Logger.info(`Querying process state: ${targetProcess}`);

    try {
      // Query using CW20 compatible method
      const result = await this.cwao.query({
        process: targetProcess,
        func: "get_info",
        input: {}
      });

      Logger.success('Process state retrieved successfully');
      return result;
    } catch (error) {
      Logger.warning(`CW20 query failed, trying D-TPRES specific query: ${error.message}`);

      try {
        // Fallback to D-TPRES specific query
        const fallbackResult = await this.cwao.query({
          process: targetProcess,
          action: "get-state",
          input: {}
        });

        Logger.success('Process state retrieved with fallback method');
        return fallbackResult;
      } catch (fallbackError) {
        Logger.error(`Failed to query process state: ${fallbackError.message}`);
        throw fallbackError;
      }
    }
  }

  /**
   * Test balance query for CW20 compatibility
   */
  async queryBalance(processId = null, address = null) {
    this.ensureInitialized();

    const targetProcess = processId || this.processes.owner;
    if (!targetProcess) {
      throw new Error('No process ID specified and no default process available.');
    }

    const queryAddr = address || toBech32(await this.arweave.wallets.jwkToAddress(this.wallet), "ao");

    Logger.info(`Querying balance for ${queryAddr} on process: ${targetProcess}`);

    try {
      const result = await this.cwao.query({
        process: targetProcess,
        func: "balance",
        input: { address: queryAddr }
      });

      Logger.success(`Balance query successful: ${result}`);
      return result;
    } catch (error) {
      Logger.error(`Failed to query balance: ${error.message}`);
      throw error;
    }
  }

  /**
   * Get deployment summary
   */
  getDeploymentSummary() {
    return {
      initialized: this.initialized,
      moduleId: this.moduleId,
      network: this.config.network,
      protocol: this.config.protocol,
      variant: this.config.variant,
      processes: {
        owner: this.processes.owner,
        holders: this.processes.holders,
        holderCount: this.processes.holders.length
      },
      wallet: this.wallet ? '***loaded***' : null,
      localConfig: this.localConfig ? '***loaded***' : null,
      suUrl: this.localConfig?.urls?.su || 'http://localhost:1986'
    };
  }

  /**
   * Ensure CWAO is initialized
   */
  ensureInitialized() {
    if (!this.initialized || !this.cwao) {
      throw new Error('CWAO not initialized. Call initialize() first.');
    }
  }
}

// Export for module usage
export default D_TPRES_CWAO;