#!/usr/bin/env node

/**
 * Direct AO Message Test
 * Send messages directly to the deployed module via Arweave
 */

global.fetch = require('node-fetch');

const Arweave = require('arweave');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
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

console.log('🧪 Direct AO Message Test');
console.log('=========================\n');
console.log(`Module/Process ID: ${moduleId}\n`);

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

// Create and send AO message
async function sendAOMessage(wallet, action, input) {
  try {
    const address = await arweave.wallets.jwkToAddress(wallet);
    
    // Create message data
    const messageData = {
      Target: moduleId,
      Action: action,
      Input: JSON.stringify(input),
      Timestamp: Date.now().toString(),
    };
    
    // Create transaction
    const tx = await arweave.createTransaction({
      data: JSON.stringify(messageData),
    }, wallet);
    
    // Add AO tags
    tx.addTag('Data-Protocol', 'ao');
    tx.addTag('Type', 'Message');
    tx.addTag('Target', moduleId);
    tx.addTag('Action', action);
    tx.addTag('From', address);
    tx.addTag('SDK', 'D-TPRES-CLI');
    
    // Add input as tag for visibility
    if (input) {
      tx.addTag('Input', JSON.stringify(input));
    }
    
    // Sign and send
    await arweave.transactions.sign(tx, wallet);
    const response = await arweave.transactions.post(tx);
    
    if (response.status === 200) {
      console.log(`✅ Message sent: ${tx.id}`);
      console.log(`   View at: https://arweave.net/tx/${tx.id}`);
      return tx.id;
    } else {
      console.error(`❌ Failed to send message: ${response.status}`);
      return null;
    }
  } catch (error) {
    console.error(`❌ Error: ${error.message}`);
    return null;
  }
}

// Test messages
async function runTests() {
  const wallet = loadWallet();
  const address = await arweave.wallets.jwkToAddress(wallet);
  console.log(`Wallet: ${address}\n`);
  
  // Check balance
  const balance = await arweave.wallets.getBalance(address);
  const ar = arweave.ar.winstonToAr(balance);
  console.log(`Balance: ${ar} AR\n`);
  
  if (parseFloat(ar) < 0.001) {
    console.error('⚠️  Low balance. You may not have enough AR for transactions.');
  }
  
  console.log('📤 Sending test messages to AO module...\n');
  
  // Test 1: Query Process Info
  console.log('1️⃣ Query Process Info');
  const msgId1 = await sendAOMessage(wallet, 'ProcessInfo', {});
  
  // Test 2: Setup Encryption
  console.log('\n2️⃣ Setup Encryption');
  const secret = Buffer.from('Test secret for D-TPRES MVP');
  const setupInput = {
    secret: Array.from(secret),
    threshold: 2,
    total_shares: 3,
  };
  const msgId2 = await sendAOMessage(wallet, 'SetupEncryption', setupInput);
  
  // Test 3: Store kFrags
  console.log('\n3️⃣ Store kFrags');
  const kfrags = [
    {
      id: 1,
      key_data: Array.from(crypto.randomBytes(32)),
      verification_data: Array.from(crypto.randomBytes(64)),
      precursor: Array.from(crypto.randomBytes(32)),
    },
  ];
  const msgId3 = await sendAOMessage(wallet, 'StoreKfrags', { kfrags });
  
  // Test 4: Query kFrags
  console.log('\n4️⃣ Query kFrags');
  const msgId4 = await sendAOMessage(wallet, 'GetKfrags', {});
  
  // Test 5: Process Access Request
  console.log('\n5️⃣ Process Access Request');
  const accessInput = {
    requester_id: 'test-requester-123',
    capsule_id: 'test-capsule-456',
  };
  const msgId5 = await sendAOMessage(wallet, 'ProcessAccessRequest', accessInput);
  
  // Summary
  console.log('\n📊 Summary');
  console.log('===========');
  console.log('Messages sent to AO module:');
  if (msgId1) console.log(`  ProcessInfo: ${msgId1}`);
  if (msgId2) console.log(`  SetupEncryption: ${msgId2}`);
  if (msgId3) console.log(`  StoreKfrags: ${msgId3}`);
  if (msgId4) console.log(`  GetKfrags: ${msgId4}`);
  if (msgId5) console.log(`  ProcessAccessRequest: ${msgId5}`);
  
  console.log('\n💡 Notes:');
  console.log('  - Messages are sent to Arweave/AO network');
  console.log('  - Results can be checked via AO Explorer or GraphQL');
  console.log('  - Processing may take 1-2 minutes');
  console.log(`  - Module URL: https://www.ao.link/#/message/${moduleId}`);
}

// Run tests
runTests().catch(error => {
  console.error('❌ Unexpected error:', error);
  process.exit(1);
});