#!/usr/bin/env node

/**
 * CLI Test for D-TPRES MVP
 * Test the deployed module with direct AO messages
 */

// Set up fetch for Node.js
global.fetch = require('node-fetch');

const { CWAO } = require('cwao');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
require('dotenv').config();

// Load deployment info
const deploymentFile = path.join(__dirname, 'deployment.json');
if (!fs.existsSync(deploymentFile)) {
  console.error('❌ No deployment found. Run npm run deploy:owner first');
  process.exit(1);
}

const deployment = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
const moduleId = deployment.mainnet?.owner?.moduleId || deployment.testnet?.owner?.moduleId;
const processId = deployment.mainnet?.owner?.processId || deployment.testnet?.owner?.processId || moduleId;

console.log('🧪 D-TPRES CLI Test');
console.log('===================\n');
console.log(`Module ID: ${moduleId}`);
console.log(`Process ID: ${processId}\n`);

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

// Generate test data
function generateTestData() {
  // Generate a test secret
  const secret = Buffer.from('This is my test secret for D-TPRES MVP');
  
  // Generate test keys (simulated - in real implementation would use crypto library)
  const ownerKeyPair = {
    secretKey: crypto.randomBytes(32),
    publicKey: crypto.randomBytes(33),
  };
  
  const symmetricKey = crypto.randomBytes(32); // k_O
  
  return {
    secret: Array.from(secret),
    ownerKeyPair,
    symmetricKey: Array.from(symmetricKey),
    threshold: 2,
    totalShares: 3,
  };
}

// Test 1: Query process info
async function testQueryProcessInfo(cwao) {
  console.log('📋 Test 1: Query Process Info');
  console.log('------------------------------');
  
  try {
    const result = await cwao.query({
      process: processId,
      action: 'ProcessInfo',
      input: {},
    });
    
    console.log('✅ Process Info Retrieved:');
    console.log(JSON.stringify(result, null, 2));
    return true;
  } catch (error) {
    console.error('❌ Query failed:', error.message);
    return false;
  }
}

// Test 2: Setup Encryption
async function testSetupEncryption(cwao, testData) {
  console.log('\n🔐 Test 2: Setup Encryption');
  console.log('------------------------------');
  
  try {
    const input = {
      secret: testData.secret,
      threshold: testData.threshold,
      total_shares: testData.totalShares,
    };
    
    console.log('Input:', JSON.stringify(input, null, 2));
    
    const result = await cwao.execute({
      process: processId,
      action: 'SetupEncryption',
      input: input,
    });
    
    console.log('✅ Setup Encryption Result:');
    console.log(JSON.stringify(result, null, 2));
    return true;
  } catch (error) {
    console.error('❌ Setup failed:', error.message);
    return false;
  }
}

// Test 3: Store kFrags
async function testStoreKfrags(cwao) {
  console.log('\n📦 Test 3: Store kFrags');
  console.log('------------------------------');
  
  try {
    // Create mock kFrags
    const kfrags = [
      {
        id: 1,
        key_data: Array.from(crypto.randomBytes(32)),
        verification_data: Array.from(crypto.randomBytes(64)),
        precursor: Array.from(crypto.randomBytes(32)),
      },
      {
        id: 2,
        key_data: Array.from(crypto.randomBytes(32)),
        verification_data: Array.from(crypto.randomBytes(64)),
        precursor: Array.from(crypto.randomBytes(32)),
      },
    ];
    
    const result = await cwao.execute({
      process: processId,
      action: 'StoreKfrags',
      input: { kfrags },
    });
    
    console.log('✅ Store kFrags Result:');
    console.log(JSON.stringify(result, null, 2));
    return true;
  } catch (error) {
    console.error('❌ Store failed:', error.message);
    return false;
  }
}

// Test 4: Query stored kFrags
async function testQueryKfrags(cwao) {
  console.log('\n🔍 Test 4: Query Stored kFrags');
  console.log('------------------------------');
  
  try {
    const result = await cwao.query({
      process: processId,
      action: 'GetKfrags',
      input: {},
    });
    
    console.log('✅ kFrags Retrieved:');
    console.log(JSON.stringify(result, null, 2));
    return true;
  } catch (error) {
    console.error('❌ Query failed:', error.message);
    return false;
  }
}

// Test 5: Process Access Request
async function testProcessAccessRequest(cwao) {
  console.log('\n🔓 Test 5: Process Access Request');
  console.log('----------------------------------');
  
  try {
    const input = {
      requester_id: 'test-requester-123',
      capsule_id: 'test-capsule-456',
    };
    
    const result = await cwao.execute({
      process: processId,
      action: 'ProcessAccessRequest',
      input: input,
    });
    
    console.log('✅ Access Request Result:');
    console.log(JSON.stringify(result, null, 2));
    return true;
  } catch (error) {
    console.error('❌ Access request failed:', error.message);
    return false;
  }
}

// Main test runner
async function runTests() {
  console.log('🚀 Starting CLI Tests...\n');
  
  // Load wallet
  const wallet = loadWallet();
  
  // Initialize CWAO
  console.log('Initializing CWAO SDK...');
  const cwao = new CWAO({ wallet });
  
  // Generate test data
  const testData = generateTestData();
  
  // Run tests
  const results = {
    processInfo: await testQueryProcessInfo(cwao),
    setupEncryption: await testSetupEncryption(cwao, testData),
    storeKfrags: await testStoreKfrags(cwao),
    queryKfrags: await testQueryKfrags(cwao),
    accessRequest: await testProcessAccessRequest(cwao),
  };
  
  // Summary
  console.log('\n📊 Test Summary');
  console.log('================');
  const passed = Object.values(results).filter(r => r).length;
  const total = Object.keys(results).length;
  
  console.log(`Passed: ${passed}/${total}`);
  
  Object.entries(results).forEach(([test, result]) => {
    const icon = result ? '✅' : '❌';
    console.log(`${icon} ${test}`);
  });
  
  if (passed === total) {
    console.log('\n🎉 All tests passed!');
  } else {
    console.log('\n⚠️  Some tests failed. Check the output above.');
  }
}

// Run tests
runTests().catch(error => {
  console.error('❌ Unexpected error:', error);
  process.exit(1);
});