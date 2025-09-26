#!/usr/bin/env node

/**
 * D-TPRES Proper AO Network Deployment Script
 * Based on CWAO SDK documentation
 * 
 * Step 1: Deploy module (upload WASM to Arweave)
 * Step 2: Instantiate process (create AO process from module)
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
  .option('-n, --network <network>', 'Network to deploy to (mainnet|testnet)', process.env.NETWORK || 'mainnet')
  .option('-s, --scheduler <id>', 'Scheduler ID')
  .parse(process.argv);

const options = program.opts();

// Configuration
const CONFIG = {
  role: options.role,
  walletPath: options.wallet,
  network: options.network,
  // AO mainnet scheduler address
  schedulerAddress: options.scheduler || '_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA',
  wasmPath: path.join(__dirname, '..', 'd_tpres_optimized.wasm'),
};

// Initialize Arweave
const arweave = Arweave.init({
  host: 'arweave.net',
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

async function checkBalance(wallet) {
  try {
    const address = await arweave.wallets.jwkToAddress(wallet);
    const balance = await arweave.wallets.getBalance(address);
    const ar = arweave.ar.winstonToAr(balance);
    
    console.log(`💰 Wallet address: ${address}`);
    console.log(`   Balance: ${ar} AR`);
    
    if (parseFloat(ar) < 0.01) {
      console.warn('⚠️  Low balance warning. You may not have enough AR for deployment.');
      if (CONFIG.network === 'mainnet') {
        console.error('❌ Insufficient balance for mainnet deployment.');
        process.exit(1);
      }
    }
    
    return address;
  } catch (error) {
    console.error('❌ Failed to check balance:', error.message);
    return null;
  }
}

async function deployModule(cwao, wasmBuffer) {
  try {
    console.log('\n📤 Step 1: Deploying WASM module to Arweave...');
    console.log('   This uploads the contract code to permanent storage');
    
    const moduleId = await cwao.deploy(wasmBuffer);
    
    if (!moduleId) {
      throw new Error('Failed to get module ID from deployment');
    }
    
    console.log(`✅ Module deployed successfully!`);
    console.log(`   Module ID: ${moduleId}`);
    console.log(`   View at: https://arweave.net/tx/${moduleId}`);
    
    return moduleId;
  } catch (error) {
    console.error('❌ Module deployment failed:', error.message);
    throw error;
  }
}

async function instantiateProcess(cwao, moduleId) {
  try {
    console.log('\n🎭 Step 2: Instantiating process from module...');
    console.log(`   Module: ${moduleId}`);
    console.log(`   Scheduler: ${CONFIG.schedulerAddress}`);
    
    // Instantiate message for CosmWasm contract
    const instantiateMsg = {
      role: CONFIG.role,
      process_id: `dtpres_${CONFIG.role}_${Date.now()}`,
    };
    
    console.log('📝 Instantiate message:', JSON.stringify(instantiateMsg, null, 2));
    
    // Instantiate the process
    const processResponse = await cwao.instantiate({
      module: moduleId,
      scheduler: CONFIG.schedulerAddress,
      input: instantiateMsg,
    });
    
    console.log('Process response:', JSON.stringify(processResponse, null, 2));
    
    // Extract process ID from response
    let processId = processResponse;
    if (typeof processResponse === 'object') {
      if (processResponse.error) {
        console.warn('⚠️  Process instantiation returned error:', processResponse.error);
        console.log('   Using module ID as process ID');
        processId = moduleId;
      } else {
        processId = processResponse.processId || processResponse.id || processResponse.txId || moduleId;
      }
    }
    
    console.log(`✅ Process ID: ${processId}`);
    console.log(`   View at: https://www.ao.link/#/process/${processId}`);
    
    return processId;
  } catch (error) {
    console.error('⚠️  Process instantiation failed:', error.message);
    console.log('   Note: The module is still deployed and can be used later');
    return null;
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
    scheduler: CONFIG.schedulerAddress,
  };
  
  fs.writeFileSync(deploymentFile, JSON.stringify(deployments, null, 2));
  console.log(`\n💾 Deployment info saved to: ${deploymentFile}`);
}

async function main() {
  console.log('====================================');
  console.log('  D-TPRES AO Network Deployment');
  console.log('====================================');
  console.log(`Network: ${CONFIG.network}`);
  console.log(`Role: ${CONFIG.role}`);
  console.log(`Scheduler: ${CONFIG.schedulerAddress}`);
  console.log('');
  
  // Load wallet
  console.log('🔑 Loading wallet...');
  const wallet = await loadWallet();
  const walletAddress = await checkBalance(wallet);
  
  // Load WASM module
  console.log('\n📦 Loading WASM module...');
  const wasmBuffer = await loadWasm();
  
  try {
    // Initialize CWAO SDK - let it create its own arweave instance
    console.log('\n🚀 Initializing CWAO SDK...');
    const cwao = new CWAO({ 
      wallet,
    });
    
    // Step 1: Deploy module to Arweave
    const moduleId = await deployModule(cwao, wasmBuffer);
    
    // Wait a bit for propagation
    console.log('\n⏳ Waiting for module to propagate...');
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    // Step 2: Instantiate process
    const processId = await instantiateProcess(cwao, moduleId);
    
    // Save deployment info
    const deploymentInfo = {
      moduleId,
      processId: processId || moduleId, // Use module ID as fallback
      role: CONFIG.role,
      walletAddress,
    };
    
    await saveDeploymentInfo(deploymentInfo);
    
    // Success summary
    console.log('\n====================================');
    console.log('  🎉 Deployment Complete!');
    console.log('====================================');
    console.log(`Module ID: ${moduleId}`);
    console.log(`Process ID: ${processId || 'Use module ID'}`);
    console.log(`Role: ${CONFIG.role}`);
    console.log('');
    console.log('📍 Important URLs:');
    console.log(`   Module TX: https://arweave.net/tx/${moduleId}`);
    console.log(`   AO Process: https://www.ao.link/#/process/${processId || moduleId}`);
    console.log(`   AO Message: https://www.ao.link/#/message/${moduleId}`);
    console.log('');
    console.log('📋 Next Steps:');
    console.log('   1. Wait 1-2 minutes for full propagation');
    console.log('   2. Check the module TX URL to verify upload');
    console.log('   3. Use aos CLI to interact with the process');
    console.log('   4. Run "npm test" to verify deployment');
    
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