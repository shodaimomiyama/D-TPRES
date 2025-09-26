#!/usr/bin/env node

/**
 * Spawn a process on AO Network using the deployed module
 */

const Arweave = require('arweave');
const fs = require('fs');
const path = require('path');
require('dotenv').config();

// Initialize Arweave
const arweave = Arweave.init({
  host: 'arweave.net',
  port: 443,
  protocol: 'https',
});

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

// Load deployment info
const deploymentFile = path.join(__dirname, 'deployment.json');
if (!fs.existsSync(deploymentFile)) {
  console.error('❌ No deployment found. Run npm run deploy:owner first');
  process.exit(1);
}

const deployment = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
const moduleId = deployment.testnet?.owner?.moduleId;

if (!moduleId) {
  console.error('❌ No module ID found in deployment');
  process.exit(1);
}

async function spawnProcess() {
  console.log('🚀 Spawning AO Process');
  console.log('======================\n');
  console.log(`Module ID: ${moduleId}\n`);
  
  const wallet = loadWallet();
  const address = await arweave.wallets.jwkToAddress(wallet);
  console.log(`Wallet: ${address}\n`);
  
  try {
    // Create spawn transaction
    const spawnData = {
      Target: 'AO',
      Action: 'Spawn',
      Module: moduleId,
      Scheduler: 'https://scheduler.ao-testnet.xyz',
      Tags: [
        { name: 'App-Name', value: 'D-TPRES' },
        { name: 'App-Version', value: '0.1.0-mvp' },
        { name: 'Process-Role', value: 'owner' },
      ]
    };
    
    const tx = await arweave.createTransaction({
      data: JSON.stringify(spawnData),
    }, wallet);
    
    // Add AO protocol tags
    tx.addTag('Data-Protocol', 'ao');
    tx.addTag('Type', 'Process');
    tx.addTag('Module', moduleId);
    tx.addTag('Scheduler', 'https://scheduler.ao-testnet.xyz');
    tx.addTag('SDK', 'ao');
    
    // Add instantiate message
    const instantiateMsg = {
      role: 'owner',
      process_id: `dtpres_owner_${Date.now()}`,
    };
    tx.addTag('Instantiate', JSON.stringify(instantiateMsg));
    
    console.log('✍️  Signing transaction...');
    await arweave.transactions.sign(tx, wallet);
    
    console.log('📤 Spawning process...');
    const response = await arweave.transactions.post(tx);
    
    if (response.status === 200) {
      const processId = tx.id;
      console.log(`\n✅ Process spawned successfully!`);
      console.log(`   Process ID: ${processId}`);
      console.log(`   View at: https://ao.arweave.dev/#/process/${processId}`);
      
      // Update deployment.json
      deployment.testnet.owner.processId = processId;
      deployment.testnet.owner.spawnedAt = new Date().toISOString();
      fs.writeFileSync(deploymentFile, JSON.stringify(deployment, null, 2));
      console.log(`\n💾 Updated deployment.json with process ID`);
      
      return processId;
    } else {
      console.error(`❌ Failed to spawn process: ${response.status} ${response.statusText}`);
      if (response.data) {
        console.error('Response:', response.data);
      }
    }
  } catch (error) {
    console.error('❌ Error spawning process:', error.message);
    
    // Fallback: Use module ID as process ID
    console.log('\n📋 Fallback: Using module ID as process ID');
    console.log(`   Module/Process ID: ${moduleId}`);
    console.log(`   Try: https://ao.arweave.dev/#/process/${moduleId}`);
    
    deployment.testnet.owner.processId = moduleId;
    deployment.testnet.owner.note = 'Using module ID as process ID (MVP fallback)';
    fs.writeFileSync(deploymentFile, JSON.stringify(deployment, null, 2));
    
    return moduleId;
  }
}

// Run spawn
spawnProcess().then(processId => {
  console.log('\n📊 Summary:');
  console.log(`   Module: ${moduleId}`);
  console.log(`   Process: ${processId}`);
  console.log('\nNext steps:');
  console.log('   1. Run "npm test" to verify');
  console.log('   2. Send messages to your process');
}).catch(error => {
  console.error('Unexpected error:', error);
  process.exit(1);
});