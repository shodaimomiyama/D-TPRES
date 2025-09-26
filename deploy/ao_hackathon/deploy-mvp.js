#!/usr/bin/env node

/**
 * D-TPRES MVP Deployment Script for AO Network
 * Uses Arweave to deploy the WASM module
 */

const Arweave = require('arweave');
const fs = require('fs');
const path = require('path');
const { Command } = require('commander');
require('dotenv').config();

// Initialize Arweave client
function initArweave() {
  const network = process.env.NETWORK || 'testnet';
  const host = network === 'mainnet' ? 'arweave.net' : 'arweave.dev';
  
  return Arweave.init({
    host: host,
    port: 443,
    protocol: 'https',
  });
}

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    console.error('   Please download your wallet from https://arweave.app/wallet');
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

// Load WASM module
function loadWasmModule() {
  const wasmPath = path.join(__dirname, '..', 'd_tpres_optimized.wasm');
  
  if (!fs.existsSync(wasmPath)) {
    console.error('❌ WASM module not found at', wasmPath);
    console.error('   Run "./build.sh" first to build the module');
    process.exit(1);
  }
  
  return fs.readFileSync(wasmPath);
}

// Deploy to AO Network
async function deployToAO(arweave, wallet, role) {
  console.log('🚀 Starting D-TPRES MVP deployment to AO Network...\n');
  
  // Load WASM module
  const wasmModule = loadWasmModule();
  console.log(`📦 WASM module loaded: ${(wasmModule.length / 1024).toFixed(2)} KB`);
  
  // Create transaction for WASM module
  console.log('📝 Creating transaction for WASM module...');
  const wasmTx = await arweave.createTransaction({
    data: wasmModule,
  }, wallet);
  
  // Add tags for AO
  wasmTx.addTag('Content-Type', 'application/wasm');
  wasmTx.addTag('App-Name', 'D-TPRES');
  wasmTx.addTag('App-Version', '0.1.0-mvp');
  wasmTx.addTag('Module-Format', 'wasm');
  wasmTx.addTag('Output', 'AO-Module');
  
  // Sign and upload WASM module
  console.log('✍️  Signing WASM transaction...');
  await arweave.transactions.sign(wasmTx, wallet);
  
  console.log('📤 Uploading WASM module to Arweave...');
  const response = await arweave.transactions.post(wasmTx);
  
  if (response.status !== 200) {
    throw new Error(`Failed to upload WASM module: ${response.statusText}`);
  }
  
  const moduleId = wasmTx.id;
  console.log(`✅ WASM module uploaded: ${moduleId}`);
  
  // Create process spawn message
  console.log('\n📝 Creating Owner-Process spawn message...');
  const spawnMessage = {
    Target: 'ao',
    Action: 'Spawn',
    Data: JSON.stringify({
      module: moduleId,
      scheduler: process.env.SCHEDULER_URL_TESTNET || 'https://scheduler.ao-testnet.xyz',
    }),
  };
  
  const spawnTx = await arweave.createTransaction({
    data: JSON.stringify(spawnMessage),
  }, wallet);
  
  // Add AO process tags
  spawnTx.addTag('Data-Protocol', 'ao');
  spawnTx.addTag('Type', 'Process');
  spawnTx.addTag('Module', moduleId);
  spawnTx.addTag('Scheduler', process.env.SCHEDULER_URL_TESTNET || 'https://scheduler.ao-testnet.xyz');
  spawnTx.addTag('Process-Role', role);
  spawnTx.addTag('Process-Name', `D-TPRES-${role}-Process`);
  
  // Add instantiate message
  const instantiateMsg = {
    role: role,
    process_id: `d-tpres-${role}-${Date.now()}`,
  };
  spawnTx.addTag('Instantiate', JSON.stringify(instantiateMsg));
  
  // Sign and upload process spawn
  console.log('✍️  Signing process spawn transaction...');
  await arweave.transactions.sign(spawnTx, wallet);
  
  console.log('📤 Spawning Owner-Process on AO Network...');
  const spawnResponse = await arweave.transactions.post(spawnTx);
  
  if (spawnResponse.status !== 200) {
    throw new Error(`Failed to spawn process: ${spawnResponse.statusText}`);
  }
  
  const processId = spawnTx.id;
  console.log(`✅ Owner-Process spawned: ${processId}`);
  
  // Save deployment info
  const deploymentInfo = {
    processId: processId,
    moduleId: moduleId,
    txId: spawnTx.id,
    role: role,
    network: process.env.NETWORK || 'testnet',
    timestamp: Date.now(),
  };
  
  saveDeploymentInfo(deploymentInfo);
  
  return deploymentInfo;
}

// Save deployment info
function saveDeploymentInfo(info) {
  const deploymentFile = path.join(__dirname, 'deployment.json');
  
  let deployments = {};
  if (fs.existsSync(deploymentFile)) {
    deployments = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
  }
  
  const network = info.network;
  if (!deployments[network]) {
    deployments[network] = {};
  }
  
  deployments[network][info.role] = info;
  
  fs.writeFileSync(deploymentFile, JSON.stringify(deployments, null, 2));
  console.log(`\n💾 Deployment info saved to deployment.json`);
}

// Main function
async function main() {
  const program = new Command();
  
  program
    .name('deploy-mvp')
    .description('Deploy D-TPRES MVP to AO Network')
    .option('--role <role>', 'Process role (owner|holder|requester)', 'owner')
    .option('--network <network>', 'Network (mainnet|testnet)', process.env.NETWORK || 'testnet')
    .parse(process.argv);
  
  const options = program.opts();
  
  // Validate role
  const validRoles = ['owner', 'holder', 'requester'];
  if (!validRoles.includes(options.role)) {
    console.error(`❌ Invalid role: ${options.role}`);
    console.error(`   Valid roles: ${validRoles.join(', ')}`);
    process.exit(1);
  }
  
  // Update environment
  process.env.NETWORK = options.network;
  
  console.log('🎯 D-TPRES MVP Deployment');
  console.log('========================');
  console.log(`📍 Network: ${options.network}`);
  console.log(`👤 Role: ${options.role}`);
  console.log(`📂 Wallet: ${process.env.WALLET_PATH || './wallet.json'}\n`);
  
  try {
    // Initialize Arweave
    const arweave = initArweave();
    
    // Load wallet
    const wallet = loadWallet();
    
    // Check wallet balance
    const address = await arweave.wallets.jwkToAddress(wallet);
    const balance = await arweave.wallets.getBalance(address);
    const ar = arweave.ar.winstonToAr(balance);
    console.log(`💰 Wallet balance: ${ar} AR`);
    
    // For AO testnet, we might not need AR tokens
    if (options.network === 'testnet') {
      console.log('📝 Note: AO testnet may not require AR tokens for deployment.');
      console.log('   Attempting deployment anyway...\n');
    } else if (parseFloat(ar) < 0.01) {
      console.error('❌ Insufficient balance. You need at least 0.01 AR for mainnet deployment.');
      process.exit(1);
    }
    
    // Deploy to AO
    const deployment = await deployToAO(arweave, wallet, options.role);
    
    // Success message
    console.log('\n✨ Deployment successful!');
    console.log('=========================');
    console.log(`🆔 Process ID: ${deployment.processId}`);
    console.log(`📦 Module ID: ${deployment.moduleId}`);
    console.log(`🔗 Transaction: ${deployment.txId}`);
    
    const explorerUrl = options.network === 'mainnet'
      ? `https://ao.arweave.dev/#/process/${deployment.processId}`
      : `https://ao-testnet.arweave.dev/#/process/${deployment.processId}`;
    
    console.log(`\n🔍 View on AO Explorer:`);
    console.log(`   ${explorerUrl}`);
    
    console.log('\n📋 Next steps:');
    console.log('   1. Wait 1-2 minutes for the transaction to be confirmed');
    console.log('   2. Run "npm test" to verify the deployment');
    console.log('   3. Send messages to your process through the AO Explorer');
    
  } catch (error) {
    console.error('\n❌ Deployment failed:', error.message);
    if (error.response) {
      console.error('   Response:', error.response.data || error.response.statusText);
    }
    process.exit(1);
  }
}

// Run deployment
main().catch(error => {
  console.error('❌ Unexpected error:', error);
  process.exit(1);
});