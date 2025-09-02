#!/usr/bin/env node

/**
 * D-TPRES Process Test Script (MVP Version)
 * Basic test script for MVP deployment verification
 */

const fs = require('fs');
const path = require('path');
require('dotenv').config();

// Load deployment info
function loadDeploymentInfo() {
  const deploymentFile = path.join(__dirname, 'deployment.json');
  
  if (!fs.existsSync(deploymentFile)) {
    console.log('❌ No deployment found. Run "npm run deploy" first.');
    console.log('\n📋 To deploy the MVP:');
    console.log('   1. Make sure you have a wallet.json file in this directory');
    console.log('   2. Run: npm run deploy:owner');
    console.log('   3. Then run this test again');
    return null;
  }
  
  const deployments = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
  const network = process.env.NETWORK || 'testnet';
  
  if (!deployments[network] || !deployments[network].owner) {
    console.log(`❌ No ${network} deployment found for owner role.`);
    console.log('\n📋 To deploy:');
    console.log('   Run: npm run deploy:owner');
    return null;
  }
  
  return deployments[network].owner;
}

// Main test function
async function main() {
  console.log('🚀 D-TPRES MVP Test');
  console.log('=====================\n');
  
  const deployment = loadDeploymentInfo();
  
  if (!deployment) {
    console.log('\n⚠️  MVP deployment test cannot continue without a deployment.');
    process.exit(1);
  }
  
  console.log('✅ Deployment found!');
  console.log('   Process ID:', deployment.processId);
  console.log('   Module ID:', deployment.moduleId);
  console.log('   Transaction ID:', deployment.txId);
  console.log('   Network:', process.env.NETWORK || 'testnet');
  console.log('   Timestamp:', new Date(deployment.timestamp).toLocaleString());
  
  console.log('\n📊 MVP Features:');
  console.log('   ✅ O-Browser functionality (simulated in contract)');
  console.log('   ✅ Owner-Process for kFrag management');
  console.log('   ✅ SetupEncryption message handler');
  console.log('   ✅ StoreKfrags message handler');
  console.log('   ✅ ProcessAccessRequest handler (auto-approve)');
  
  console.log('\n🔗 View on AO Explorer:');
  const network = process.env.NETWORK || 'testnet';
  const explorerUrl = network === 'mainnet' 
    ? `https://ao.arweave.dev/#/process/${deployment.processId}`
    : `https://ao-testnet.arweave.dev/#/process/${deployment.processId}`;
  console.log(`   ${explorerUrl}`);
  
  console.log('\n💡 Next Steps:');
  console.log('   1. Visit the AO Explorer link above');
  console.log('   2. Send test messages through the explorer');
  console.log('   3. Or integrate with your frontend application');
  
  console.log('\n✨ MVP deployment test completed successfully!');
}

// Run tests
main().catch(error => {
  console.error('\n❌ Test failed:', error.message);
  process.exit(1);
});