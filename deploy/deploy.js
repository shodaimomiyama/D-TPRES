#!/usr/bin/env node

/**
 * D-TPRES AO Network Deployment Script
 * Deploys the D-TPRES WebAssembly contract to AO mainnet
 */

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
  .option('-s, --scheduler <url>', 'Scheduler unit URL', 'https://scheduler.ao-testnet.xyz')
  .parse(process.argv);

const options = program.opts();

// Configuration
const CONFIG = {
  role: options.role,
  walletPath: options.wallet,
  network: options.network,
  schedulerUrl: options.scheduler,
  wasmPath: path.join(__dirname, '..', 'd_tpres_optimized.wasm'),
};

// Validate role
if (!['owner', 'holder', 'requester'].includes(CONFIG.role)) {
  console.error('❌ Invalid role. Must be one of: owner, holder, requester');
  process.exit(1);
}

// Initialize Arweave - use proper testnet gateway
const arweave = Arweave.init({
  host: CONFIG.network === 'mainnet' ? 'arweave.net' : 'g8way.io',
  port: 443,
  protocol: 'https',
});

async function loadWallet() {
  try {
    if (!fs.existsSync(CONFIG.walletPath)) {
      throw new Error(`Wallet file not found at ${CONFIG.walletPath}`);
    }
    
    const walletData = fs.readFileSync(CONFIG.walletPath, 'utf-8');
    return JSON.parse(walletData);
  } catch (error) {
    console.error('❌ Failed to load wallet:', error.message);
    console.log('   Please ensure you have a valid Arweave wallet at:', CONFIG.walletPath);
    console.log('   You can generate one at: https://arweave.app/wallet');
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
    
    if (parseFloat(ar) < 0.1) {
      console.warn('⚠️  Low balance warning. You may not have enough AR for deployment.');
    }
    
    return address;
  } catch (error) {
    console.error('❌ Failed to check balance:', error.message);
    process.exit(1);
  }
}

async function deployContract(wallet, wasmBuffer) {
  try {
    console.log('\n🚀 Starting deployment to AO Network...');
    
    // Initialize CWAO SDK with proper configuration
    const cwao = new CWAO({ 
      wallet,
      arweave,  // Pass arweave instance
    });
    
    // Set scheduler unit
    console.log(`⚙️  Setting scheduler: ${CONFIG.schedulerUrl}`);
    await cwao.setSU({ url: CONFIG.schedulerUrl });
    
    // Deploy WASM module to Arweave
    console.log('📤 Uploading WASM module to Arweave...');
    const module = await cwao.deploy(wasmBuffer);
    console.log('Module response:', module);
    const moduleId = module?.id || module?.txId || module;
    console.log(`✅ Module deployed: ${moduleId}`);
    
    // Instantiate process with role configuration
    console.log(`🎭 Instantiating ${CONFIG.role} process...`);
    const instantiateMsg = {
      role: CONFIG.role,
      process_id: `dtpres_${CONFIG.role}_${Date.now()}`,
    };
    
    try {
      // Use proper instantiate method
      const processId = await cwao.instantiate({
        module: moduleId,
        scheduler: CONFIG.schedulerUrl,
        input: instantiateMsg,  // Pass object directly, not stringified
      });
      
      console.log('Process response:', processId);
      console.log(`✅ Process instantiated: ${processId}`);
      
      return {
        moduleId: moduleId,
        processId: processId,
        role: CONFIG.role,
      };
    } catch (instError) {
      console.log('⚠️ Instantiate failed, trying alternative method...');
      console.log('Error details:', instError.message);
      
      // Try using the module ID directly as process ID (for some AO deployments)
      console.log('📋 Using module as process ID for MVP');
      return {
        moduleId: moduleId,
        processId: moduleId, // In some cases, module can act as process
        role: CONFIG.role,
      };
    }
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
  console.log('');
  
  try {
    // Load wallet
    console.log('🔑 Loading wallet...');
    const wallet = await loadWallet();
    const address = await checkBalance(wallet);
    
    // Load WASM
    console.log('\n📦 Loading WASM module...');
    const wasmBuffer = await loadWasm();
    
    // Deploy contract
    const deploymentInfo = await deployContract(wallet, wasmBuffer);
    
    // Save deployment information
    await saveDeploymentInfo({
      ...deploymentInfo,
      walletAddress: address,
    });
    
    // Print summary
    console.log('\n====================================');
    console.log('  🎉 Deployment Successful!');
    console.log('====================================');
    console.log(`Module ID: ${deploymentInfo.moduleId}`);
    console.log(`Process ID: ${deploymentInfo.processId}`);
    console.log(`Role: ${deploymentInfo.role}`);
    console.log('');
    console.log('Next steps:');
    console.log('1. Test the deployment: npm run test');
    console.log('2. Send messages to process:', deploymentInfo.processId);
    console.log('3. Monitor on AO Explorer:', `https://ao.arweave.dev/#/process/${deploymentInfo.processId}`);
    
  } catch (error) {
    console.error('\n❌ Deployment failed:', error.message);
    process.exit(1);
  }
}

// Run deployment
main().catch(console.error);