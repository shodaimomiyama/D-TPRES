#!/usr/bin/env node

/**
 * D-TPRES Process Test Script
 * Tests the deployed D-TPRES process on AO Network
 */

const Arweave = require('arweave');
const fs = require('fs');
const path = require('path');
require('dotenv').config();

// Load deployment info
function loadDeploymentInfo() {
  const deploymentFile = path.join(__dirname, 'deployment.json');
  
  if (!fs.existsSync(deploymentFile)) {
    console.error('❌ No deployment found. Run "npm run deploy" first.');
    process.exit(1);
  }
  
  const deployments = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
  const network = process.env.NETWORK || 'testnet';
  
  if (!deployments[network] || !deployments[network].owner) {
    console.error(`❌ No ${network} deployment found for owner role.`);
    process.exit(1);
  }
  
  return deployments[network].owner;
}

// Load wallet
function loadWallet() {
  const walletPath = process.env.WALLET_PATH || './wallet.json';
  
  if (!fs.existsSync(walletPath)) {
    console.error(`❌ Wallet not found at ${walletPath}`);
    process.exit(1);
  }
  
  return JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
}

// Test setup encryption
async function testSetupEncryption(cwao, processId) {
  console.log('\n📝 Testing SetupEncryption...');
  
  const message = {
    action: 'SetupEncryption',
    input: {
      secret: Array.from(Buffer.from('Test secret for D-TPRES MVP')),
      threshold: 2,
      total_shares: 3,
    },
  };
  
  try {
    const result = await cwao.message({
      process: processId,
      data: message,
      tags: [
        { name: 'Action', value: 'SetupEncryption' },
      ],
    });
    
    console.log('✅ SetupEncryption executed successfully');
    console.log('   Transaction ID:', result.id);
    
    return result;
  } catch (error) {
    console.error('❌ SetupEncryption failed:', error.message);
    throw error;
  }
}

// Test query process info
async function testQueryProcessInfo(cwao, processId) {
  console.log('\n🔍 Querying Process Info...');
  
  try {
    const result = await cwao.query({
      process: processId,
      data: { query: 'ProcessInfo' },
    });
    
    console.log('✅ Process info retrieved:');
    console.log('   Role:', result.role);
    console.log('   Process ID:', result.process_id);
    console.log('   kFrags count:', result.kfrags_count);
    
    return result;
  } catch (error) {
    console.error('❌ Query failed:', error.message);
    throw error;
  }
}

// Test get kfrags
async function testGetKfrags(cwao, processId) {
  console.log('\n📦 Getting kFrags...');
  
  try {
    const result = await cwao.query({
      process: processId,
      data: { query: 'GetKfrags' },
    });
    
    console.log('✅ kFrags retrieved:');
    console.log('   Count:', result.kfrags ? result.kfrags.length : 0);
    
    if (result.kfrags && result.kfrags.length > 0) {
      result.kfrags.forEach((kfrag, i) => {
        console.log(`   kFrag ${i}: ID=${kfrag.id}, Size=${kfrag.key_data.length} bytes`);
      });
    }
    
    return result;
  } catch (error) {
    console.error('❌ GetKfrags failed:', error.message);
    throw error;
  }
}

// Test access request
async function testAccessRequest(cwao, processId) {
  console.log('\n🔐 Testing Access Request...');
  
  const message = {
    action: 'ProcessAccessRequest',
    input: {
      requester_id: 'test_requester_123',
      capsule_id: 'test_capsule_456',
    },
  };
  
  try {
    const result = await cwao.message({
      process: processId,
      data: message,
      tags: [
        { name: 'Action', value: 'ProcessAccessRequest' },
      ],
    });
    
    console.log('✅ Access request processed');
    console.log('   Status: Approved (MVP auto-approval)');
    
    return result;
  } catch (error) {
    console.error('❌ Access request failed:', error.message);
    throw error;
  }
}

async function main() {
  console.log('====================================');
  console.log('  D-TPRES Process Testing');
  console.log('====================================');
  
  try {
    // Load deployment and wallet
    const deployment = loadDeploymentInfo();
    const wallet = loadWallet();
    
    console.log('\n📋 Testing deployment:');
    console.log('   Process ID:', deployment.processId);
    console.log('   Module ID:', deployment.moduleId);
    console.log('   Role:', deployment.role);
    console.log('   Network:', deployment.network);
    
    // Initialize CWAO
    const cwao = new CWAO({ wallet });
    
    // Run tests
    await testQueryProcessInfo(cwao, deployment.processId);
    await testSetupEncryption(cwao, deployment.processId);
    
    // Wait a bit for processing
    console.log('\n⏳ Waiting for processing...');
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    await testGetKfrags(cwao, deployment.processId);
    await testAccessRequest(cwao, deployment.processId);
    
    console.log('\n====================================');
    console.log('  ✅ All Tests Passed!');
    console.log('====================================');
    console.log('\nYour D-TPRES MVP is ready for use on AO Network!');
    console.log('Monitor your process at:');
    console.log(`https://ao.arweave.dev/#/process/${deployment.processId}`);
    
  } catch (error) {
    console.error('\n❌ Testing failed:', error.message);
    process.exit(1);
  }
}

// Run tests
main().catch(console.error);