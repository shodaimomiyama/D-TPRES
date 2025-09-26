#!/usr/bin/env node

/**
 * Spawn AO Process - The simplest way to create an AO process
 * This creates a proper AO process from the deployed module
 */

global.fetch = require('node-fetch');

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

// Load deployment info
const deploymentFile = path.join(__dirname, 'deployment.json');
const deployment = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
const moduleId = deployment.mainnet?.owner?.moduleId;

// AO mainnet scheduler
const SCHEDULER_ID = '_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA';

console.log('🚀 Spawning AO Process');
console.log('=====================\n');
console.log(`Module ID: ${moduleId}`);
console.log(`Scheduler: ${SCHEDULER_ID}\n`);

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

async function spawnProcess() {
  const wallet = loadWallet();
  const address = await arweave.wallets.jwkToAddress(wallet);
  console.log(`Wallet: ${address}\n`);
  
  // Check balance
  const balance = await arweave.wallets.getBalance(address);
  const ar = arweave.ar.winstonToAr(balance);
  console.log(`Balance: ${ar} AR\n`);
  
  if (parseFloat(ar) < 0.001) {
    console.error('❌ Insufficient balance for transaction');
    process.exit(1);
  }
  
  try {
    console.log('📝 Creating process spawn transaction...');
    
    // Create the spawn message data
    const spawnData = JSON.stringify({
      process: {
        module: moduleId,
        scheduler: SCHEDULER_ID,
      }
    });
    
    // Create transaction
    const tx = await arweave.createTransaction({
      data: spawnData,
    }, wallet);
    
    // Add critical AO process tags
    tx.addTag('Data-Protocol', 'ao');
    tx.addTag('Variant', 'ao.TN.1');
    tx.addTag('Type', 'Process');
    tx.addTag('Module', moduleId);
    tx.addTag('Scheduler', SCHEDULER_ID);
    tx.addTag('SDK', 'aoconnect');
    
    // Add process metadata
    tx.addTag('Name', 'D-TPRES-Owner-Process');
    tx.addTag('Process-Type', 'D-TPRES');
    tx.addTag('Process-Role', 'owner');
    
    // Add instantiate parameters
    const instantiateMsg = {
      role: 'owner',
      process_id: `dtpres_owner_${Date.now()}`,
    };
    tx.addTag('Instantiate', JSON.stringify(instantiateMsg));
    
    // Add memory-limit for the process
    tx.addTag('Memory-Limit', '256-mb');
    tx.addTag('Compute-Limit', '9000000000000');
    
    console.log('✍️  Signing transaction...');
    await arweave.transactions.sign(tx, wallet);
    
    console.log('📤 Sending spawn transaction...');
    const response = await arweave.transactions.post(tx);
    
    if (response.status === 200) {
      const processId = tx.id;
      console.log('\n✅ Process spawn transaction sent!');
      console.log(`   Transaction ID: ${processId}`);
      console.log(`   This will become your Process ID once confirmed`);
      
      // Update deployment.json
      deployment.mainnet.owner.processId = processId;
      deployment.mainnet.owner.processSpawnedAt = new Date().toISOString();
      fs.writeFileSync(deploymentFile, JSON.stringify(deployment, null, 2));
      console.log('\n💾 Updated deployment.json with process ID');
      
      console.log('\n🔍 Verify your process:');
      console.log(`   1. Wait 1-2 minutes for confirmation`);
      console.log(`   2. Check: https://www.ao.link/#/process/${processId}`);
      console.log(`   3. Look for Type: "Process" (not "User")`);
      console.log(`   4. Check: https://arweave.net/tx/${processId}`);
      
      console.log('\n📋 Next steps:');
      console.log(`   1. Run: node check-process.js`);
      console.log(`   2. Run: node test-direct.js`);
      
      return processId;
    } else {
      console.error(`❌ Failed to send transaction: ${response.status}`);
      if (response.data) {
        console.error('Response:', response.data);
      }
      return null;
    }
  } catch (error) {
    console.error('❌ Error spawning process:', error.message);
    return null;
  }
}

// Run spawn
spawnProcess().then(processId => {
  if (processId) {
    console.log('\n🎉 Process spawn initiated successfully!');
  } else {
    console.log('\n❌ Process spawn failed');
    process.exit(1);
  }
}).catch(error => {
  console.error('Unexpected error:', error);
  process.exit(1);
});