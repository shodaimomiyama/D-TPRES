#!/usr/bin/env node

/**
 * Arweave Testnet Faucet Script
 * Request test AR tokens for development
 */

const Arweave = require('arweave');
const fs = require('fs');
const https = require('https');
require('dotenv').config();

// Initialize Arweave client for testnet
function initArweave() {
  return Arweave.init({
    host: 'arweave.net',  // Changed to mainnet host which also serves testnet
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

// Request tokens from faucet
async function requestFromFaucet(address) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'faucet.arweave.dev',
      port: 443,
      path: '/api/faucet',
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
        if (res.statusCode === 200) {
          resolve(JSON.parse(data));
        } else {
          reject(new Error(`Faucet request failed: ${res.statusCode} - ${data}`));
        }
      });
    });
    
    req.on('error', (error) => {
      reject(error);
    });
    
    req.write(JSON.stringify({ address: address }));
    req.end();
  });
}

// Alternative: Use public testnet faucet via fetch
async function requestFromPublicFaucet(address) {
  console.log('🚰 Requesting testnet tokens from public faucet...\n');
  console.log('📋 Manual Option:');
  console.log(`   1. Visit: https://faucet.arweave.dev/`);
  console.log(`   2. Enter your address: ${address}`);
  console.log(`   3. Complete the captcha and request tokens\n`);
  
  console.log('🔄 Automated Option:');
  console.log('   Attempting automated request...\n');
  
  try {
    // Try automated request
    const result = await requestFromFaucet(address);
    console.log('✅ Faucet request successful!');
    return result;
  } catch (error) {
    console.log('⚠️  Automated request failed. Please use the manual option above.');
    console.log(`   Error: ${error.message}`);
    
    // Alternative faucets
    console.log('\n📌 Alternative Testnet Faucets:');
    console.log('   1. ArConnect Faucet: https://faucet.arconnect.io/');
    console.log('   2. Bundlr Faucet: https://bundlr.network/faucet');
    console.log(`\n   Your wallet address: ${address}`);
  }
}

// Main function
async function main() {
  console.log('💧 Arweave Testnet Faucet');
  console.log('=========================\n');
  
  try {
    // Initialize Arweave
    const arweave = initArweave();
    
    // Load wallet
    const wallet = loadWallet();
    
    // Get wallet address
    const address = await arweave.wallets.jwkToAddress(wallet);
    console.log(`📍 Wallet Address: ${address}`);
    
    // Check current balance
    const balance = await arweave.wallets.getBalance(address);
    const ar = arweave.ar.winstonToAr(balance);
    console.log(`💰 Current Balance: ${ar} AR\n`);
    
    if (parseFloat(ar) >= 0.01) {
      console.log('✅ You already have sufficient balance for deployment!');
      console.log('   Run "npm run deploy:owner" to deploy your contract.');
      return;
    }
    
    // Request from faucet
    await requestFromPublicFaucet(address);
    
    // Wait and check new balance
    console.log('\n⏳ Waiting 10 seconds for transaction to process...');
    await new Promise(resolve => setTimeout(resolve, 10000));
    
    const newBalance = await arweave.wallets.getBalance(address);
    const newAr = arweave.ar.winstonToAr(newBalance);
    console.log(`\n💰 New Balance: ${newAr} AR`);
    
    if (parseFloat(newAr) > parseFloat(ar)) {
      console.log('✅ Tokens received successfully!');
      console.log('   You can now run "npm run deploy:owner" to deploy your contract.');
    } else {
      console.log('⏳ Tokens not yet received. Please check again in a few minutes.');
      console.log('   Or visit the faucet URL manually to request tokens.');
    }
    
  } catch (error) {
    console.error('\n❌ Error:', error.message);
    process.exit(1);
  }
}

// Run faucet request
main().catch(error => {
  console.error('❌ Unexpected error:', error);
  process.exit(1);
});