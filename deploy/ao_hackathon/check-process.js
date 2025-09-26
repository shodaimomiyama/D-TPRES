#!/usr/bin/env node

/**
 * Check and retrieve process ID from AO Network
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
const moduleId = deployment.testnet?.owner?.moduleId;

if (!moduleId) {
  console.error('No module ID found');
  process.exit(1);
}

console.log('📋 Checking Module and Process Status');
console.log('=====================================\n');
console.log(`Module ID: ${moduleId}\n`);

// Check module on Arweave
console.log('🔍 Module URLs:');
console.log(`   Arweave TX: https://viewblock.io/arweave/tx/${moduleId}`);
console.log(`   AO Module: https://ao.arweave.dev/#/module/${moduleId}`);
console.log('');

// Try to query AO network for processes using this module
console.log('🔍 Checking for processes using this module...\n');

// Query AO GraphQL endpoint
const query = {
  query: `
    query {
      transactions(
        tags: [
          { name: "Type", values: ["Process"] },
          { name: "Module", values: ["${moduleId}"] }
        ]
      ) {
        edges {
          node {
            id
            tags {
              name
              value
            }
          }
        }
      }
    }
  `
};

const options = {
  hostname: 'arweave.net',
  port: 443,
  path: '/graphql',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  }
};

const req = https.request(options, (res) => {
  let data = '';
  
  res.on('data', (chunk) => {
    data += chunk;
  });
  
  res.on('end', () => {
    try {
      const result = JSON.parse(data);
      
      if (result.data?.transactions?.edges?.length > 0) {
        console.log('✅ Found processes:');
        result.data.transactions.edges.forEach((edge, i) => {
          const processId = edge.node.id;
          console.log(`\n   Process ${i + 1}: ${processId}`);
          console.log(`   URL: https://ao.arweave.dev/#/process/${processId}`);
          
          // Show relevant tags
          const tags = edge.node.tags;
          const appName = tags.find(t => t.name === 'App-Name')?.value;
          const role = tags.find(t => t.name === 'Process-Role')?.value;
          if (appName) console.log(`   App: ${appName}`);
          if (role) console.log(`   Role: ${role}`);
        });
        
        // Update deployment.json with first process ID
        if (result.data.transactions.edges.length > 0) {
          const processId = result.data.transactions.edges[0].node.id;
          deployment.testnet.owner.processId = processId;
          fs.writeFileSync(deploymentFile, JSON.stringify(deployment, null, 2));
          console.log(`\n💾 Updated deployment.json with process ID: ${processId}`);
        }
      } else {
        console.log('❌ No processes found using this module.');
        console.log('\n💡 Alternative: Use the module ID as process ID');
        console.log(`   Try accessing: https://ao.arweave.dev/#/process/${moduleId}`);
      }
    } catch (error) {
      console.error('Error parsing response:', error.message);
      console.log('Raw response:', data);
    }
  });
});

req.on('error', (error) => {
  console.error('Request failed:', error.message);
});

req.write(JSON.stringify(query));
req.end();