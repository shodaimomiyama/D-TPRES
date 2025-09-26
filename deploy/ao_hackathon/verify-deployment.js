#!/usr/bin/env node

/**
 * Verify deployment on AO Network
 */

const https = require('https');
const fs = require('fs');
const path = require('path');

// Load deployment info
const deploymentFile = path.join(__dirname, 'deployment.json');
if (!fs.existsSync(deploymentFile)) {
  console.error('No deployment found');
  process.exit(1);
}

const deployment = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
const moduleId = deployment.mainnet?.owner?.moduleId || deployment.testnet?.owner?.moduleId;

console.log('🔍 Verifying Deployment');
console.log('=======================\n');
console.log(`Module ID: ${moduleId}\n`);

// Check on Arweave directly
function checkArweave(id) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'arweave.net',
      port: 443,
      path: `/tx/${id}`,
      method: 'GET',
      headers: {
        'Accept': 'application/json',
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        if (res.statusCode === 200) {
          resolve({ found: true, data: JSON.parse(data) });
        } else {
          resolve({ found: false, status: res.statusCode });
        }
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.end();
  });
}

// Check ViewBlock
function checkViewBlock(id) {
  console.log(`📍 ViewBlock URL: https://viewblock.io/arweave/tx/${id}`);
}

// Check AO specific endpoints
function checkAO(id) {
  console.log('\n📍 AO Network URLs:');
  console.log(`   Process: https://www.ao.link/#/process/${id}`);
  console.log(`   Message: https://www.ao.link/#/message/${id}`);
  console.log(`   Entity: https://www.ao.link/#/entity/${id}`);
}

// Alternative AO explorers
function checkAlternatives(id) {
  console.log('\n📍 Alternative Explorers:');
  console.log(`   ArweaveApp: https://arweave.app/tx/${id}`);
  console.log(`   ar.io Gateway: https://ar.io/tx/${id}`);
}

async function main() {
  try {
    // Check Arweave mainnet
    console.log('Checking Arweave mainnet...');
    const result = await checkArweave(moduleId);
    
    if (result.found) {
      console.log('✅ Transaction found on Arweave!');
      console.log(`   Size: ${result.data.data_size} bytes`);
      console.log(`   Owner: ${result.data.owner}`);
      
      // Check tags
      if (result.data.tags) {
        console.log('\n📋 Tags:');
        result.data.tags.forEach(tag => {
          const name = Buffer.from(tag.name, 'base64').toString();
          const value = Buffer.from(tag.value, 'base64').toString();
          console.log(`   ${name}: ${value}`);
        });
      }
    } else {
      console.log(`❌ Not found on Arweave mainnet (status: ${result.status})`);
      console.log('   This might be because:');
      console.log('   1. Transaction is still pending');
      console.log('   2. It was deployed to a testnet-only gateway');
      console.log('   3. The module ID is not a valid transaction');
    }
    
    checkViewBlock(moduleId);
    checkAO(moduleId);
    checkAlternatives(moduleId);
    
    // Check all deployed module IDs
    console.log('\n📦 All deployed modules:');
    const modules = new Set();
    
    // Collect all module IDs from deployment history
    if (deployment.testnet?.owner?.moduleId) {
      modules.add(deployment.testnet.owner.moduleId);
    }
    
    // Check deployment.json for other entries
    for (const [network, roles] of Object.entries(deployment)) {
      for (const [role, info] of Object.entries(roles)) {
        if (info.moduleId) modules.add(info.moduleId);
      }
    }
    
    modules.forEach(id => {
      console.log(`   ${id}`);
    });
    
    console.log('\n💡 Tips:');
    console.log('   1. Try accessing the module via ao.link instead of ao.arweave.dev');
    console.log('   2. Modules may take a few minutes to propagate');
    console.log('   3. Check if the wallet has any balance issues');
    
  } catch (error) {
    console.error('Error:', error.message);
  }
}

main();