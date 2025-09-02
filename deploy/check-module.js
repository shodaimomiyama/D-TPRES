#!/usr/bin/env node

/**
 * Check module deployment status
 */

const https = require('https');
const fs = require('fs');
const path = require('path');

// Load deployment info
const deploymentFile = path.join(__dirname, 'deployment.json');
const deployment = JSON.parse(fs.readFileSync(deploymentFile, 'utf-8'));
const moduleId = deployment.mainnet?.owner?.moduleId;

console.log('🔍 Checking Module Deployment');
console.log('==============================\n');
console.log(`Module ID: ${moduleId}\n`);

// Check multiple gateways
const gateways = [
  { name: 'Arweave.net', host: 'arweave.net' },
  { name: 'Arweave.app', host: 'arweave.app' },
  { name: 'AR.io', host: 'ar.io' },
  { name: 'G8way.io', host: 'g8way.io' },
];

function checkGateway(gateway, id) {
  return new Promise((resolve) => {
    const options = {
      hostname: gateway.host,
      port: 443,
      path: `/tx/${id}`,
      method: 'GET',
      timeout: 5000,
    };

    const req = https.request(options, (res) => {
      if (res.statusCode === 200) {
        resolve({ gateway: gateway.name, status: 'Found', code: res.statusCode });
      } else {
        resolve({ gateway: gateway.name, status: 'Not Found', code: res.statusCode });
      }
    });
    
    req.on('error', (error) => {
      resolve({ gateway: gateway.name, status: 'Error', error: error.message });
    });
    
    req.on('timeout', () => {
      req.destroy();
      resolve({ gateway: gateway.name, status: 'Timeout' });
    });
    
    req.end();
  });
}

async function checkAllGateways() {
  console.log('Checking gateways...\n');
  
  const promises = gateways.map(g => checkGateway(g, moduleId));
  const results = await Promise.all(promises);
  
  results.forEach(result => {
    const icon = result.status === 'Found' ? '✅' : '❌';
    console.log(`${icon} ${result.gateway}: ${result.status} ${result.code ? `(${result.code})` : ''}`);
  });
  
  const found = results.some(r => r.status === 'Found');
  
  if (found) {
    console.log('\n🎉 Module successfully deployed!');
    console.log('\n📍 Access URLs:');
    results.filter(r => r.status === 'Found').forEach(r => {
      const gateway = gateways.find(g => g.name === r.gateway);
      console.log(`   https://${gateway.host}/tx/${moduleId}`);
    });
  } else {
    console.log('\n⏳ Module not yet found. This could mean:');
    console.log('   1. Transaction is still propagating (wait 1-2 minutes)');
    console.log('   2. Transaction was rejected');
    console.log('   3. Module ID is invalid');
  }
  
  console.log('\n📋 AO Network URLs (try these regardless):');
  console.log(`   Process: https://www.ao.link/#/process/${moduleId}`);
  console.log(`   Message: https://www.ao.link/#/message/${moduleId}`);
  console.log(`   Entity: https://www.ao.link/#/entity/${moduleId}`);
  
  console.log('\n💡 To use with AOS (when it works):');
  console.log(`   aos ${moduleId}`);
  console.log('\n   Note: AOS may require Node.js v18 or v20 (not v24)');
}

checkAllGateways().catch(console.error);