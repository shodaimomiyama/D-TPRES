#!/usr/bin/env node

/**
 * D-TPRES AO Network Deployment Script using cw method
 * Based on cosmwasm-ao SDK documentation
 */

// Set up fetch for Node.js
global.fetch = require('node-fetch');

const { CWAO } = require('cwao');
const Arweave = require('arweave');
const fs = require('fs');
const path = require('path');
const { program } = require('commander');
require('dotenv').config();

// Parse command line arguments
program
  .option('-r, --role <role>', 'Process role (owner|holder|requester)', 'owner')
  .option('-w, --wallet <path>', 'Path to Arweave wallet JSON file', './wallet.json')
  .option('-n, --network <network>', 'Network to deploy to (mainnet|testnet)', 'testnet')
  .option('-s, --scheduler <url>', 'Scheduler unit URL')
  .parse(process.argv);

const options = program.opts();

// Configuration
const CONFIG = {
  role: options.role,
  walletPath: options.wallet,
  network: options.network,
  schedulerUrl: options.scheduler || (options.network === 'mainnet' 
    ? '_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA'  // Use AO mainnet scheduler
    : '_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA'),
  wasmPath: path.join(__dirname, '..', 'd_tpres_optimized.wasm'),
};

// Initialize Arweave with proper gateway
const arweave = Arweave.init({
  host: 'arweave.net',  // Use mainnet host for both (it serves testnet too)
  port: 443,
  protocol: 'https',
});

async function loadWallet() {
  try {
    if (!fs.existsSync(CONFIG.walletPath)) {
      throw new Error(`Wallet file not found at ${CONFIG.walletPath}`);
    }
    
    const wallet = JSON.parse(fs.readFileSync(CONFIG.walletPath, 'utf-8'));
    console.log('✅ Wallet loaded successfully');
    return wallet;
  } catch (error) {
    console.error('❌ Failed to load wallet:', error.message);
    process.exit(1);
  }
}

async function loadWasm() {
  try {
    if (!fs.existsSync(CONFIG.wasmPath)) {
      throw new Error(`WASM file not found at ${CONFIG.wasmPath}. Run './build.sh' first.`);
    }
    
    const wasmBuffer = fs.readFileSync(CONFIG.wasmPath);
    console.log(`📦 Loaded WASM binary (${(wasmBuffer.length / 1024).toFixed(2)} KB)`);
    return wasmBuffer;
  } catch (error) {
    console.error('❌ Failed to load WASM:', error.message);
    process.exit(1);
  }
}

async function deployContract(wallet, wasmBuffer) {
  try {
    console.log('\n🚀 Starting deployment to AO Network...');
    
    // Initialize CWAO SDK - let it create its own arweave instance
    const cwao = new CWAO({ 
      wallet,
    });
    
    // Deploy WASM module to AO
    console.log('📤 Uploading WASM module to AO...');
    const moduleId = await cwao.deploy(wasmBuffer);
    
    if (!moduleId) {
      throw new Error('Failed to get module ID from deployment');
    }
    
    console.log(`✅ Module deployed: ${moduleId}`);
    
    // Use simplified cw method for instantiation
    console.log(`🎭 Instantiating ${CONFIG.role} process...`);
    console.log(`   Using scheduler: ${CONFIG.schedulerUrl}`);
    
    const cw = await cwao.cw({ 
      module: moduleId, 
      scheduler: CONFIG.schedulerUrl 
    });
    
    // Instantiate with role configuration
    const instantiateMsg = {
      role: CONFIG.role,
      process_id: `dtpres_${CONFIG.role}_${Date.now()}`,
    };
    
    console.log('📝 Instantiate message:', instantiateMsg);
    const processResponse = await cw.i(instantiateMsg);
    
    console.log('📋 Process response:', JSON.stringify(processResponse, null, 2));
    
    // Extract process ID from response
    let processId = processResponse;
    if (typeof processResponse === 'object') {
      processId = processResponse.processId || processResponse.id || processResponse.txId || moduleId;
    }
    
    if (!processId || processId === '[object Object]') {
      // If instantiate returns nothing useful, use module ID as process ID
      console.log('⚠️  No valid process ID returned, using module ID');
      processId = moduleId;
    }
    
    console.log(`✅ Process instantiated: ${processId}`);
    
    return {
      moduleId: moduleId,
      processId: processId,
      role: CONFIG.role,
    };
    
  } catch (error) {
    console.error('❌ Deployment failed:', error.message);
    throw error;
  }
}

async function saveDeploymentInfo(info) {
  const deploymentFile = path.join(__dirname, 'deployment.json');
  
  let deployments = {};
  if (fs.existsSync(deploymentFile)) {
    deployments = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
  }
  
  deployments[CONFIG.network] = deployments[CONFIG.network] || {};
  deployments[CONFIG.network][CONFIG.role] = {
    ...info,
    deployedAt: new Date().toISOString(),
    network: CONFIG.network,
    scheduler: CONFIG.schedulerUrl,
  };
  
  fs.writeFileSync(deploymentFile, JSON.stringify(deployments, null, 2));
  console.log(`💾 Deployment info saved to: ${deploymentFile}`);
}

async function main() {
  console.log('====================================');
  console.log('  D-TPRES AO Network Deployment (CW)');
  console.log('====================================');
  console.log(`Network: ${CONFIG.network}`);
  console.log(`Role: ${CONFIG.role}`);
  console.log(`Scheduler: ${CONFIG.schedulerUrl}\n`);
  
  console.log('🔑 Loading wallet...');
  const wallet = await loadWallet();
  
  const address = await arweave.wallets.jwkToAddress(wallet);
  console.log(`💰 Wallet address: ${address}`);
  
  console.log('\n📦 Loading WASM module...');
  const wasmBuffer = await loadWasm();
  
  try {
    const deployment = await deployContract(wallet, wasmBuffer);
    
    deployment.walletAddress = address;
    await saveDeploymentInfo(deployment);
    
    console.log('\n====================================');
    console.log('  🎉 Deployment Successful!');
    console.log('====================================');
    console.log(`Module ID: ${deployment.moduleId}`);
    console.log(`Process ID: ${deployment.processId}`);
    console.log(`Role: ${deployment.role}`);
    
    console.log('\nNext steps:');
    console.log(`1. Test the deployment: npm test`);
    console.log(`2. View process: https://ao.arweave.dev/#/process/${deployment.processId}`);
    console.log(`3. View module: https://ao.arweave.dev/#/module/${deployment.moduleId}`);
    
  } catch (error) {
    console.error('\n❌ Deployment failed:', error.message);
    process.exit(1);
  }
}

// Run deployment
main().catch(error => {
  console.error('❌ Unexpected error:', error);
  process.exit(1);
});