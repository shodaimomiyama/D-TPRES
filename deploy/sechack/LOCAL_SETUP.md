# D-TPRES Local AO Development Setup

## Quick Start

### 1. Environment Setup
Choose your environment and copy the appropriate .env file:

```bash
# For local development
cp .env.local .env

# For testnet
cp .env.testnet .env

# For mainnet
cp .env.mainnet .env
```

### 2. Wallet Setup
Generate a wallet for deployment:

```bash
npm run wallet:generate
```

### 3. Local Development (Recommended)
For local development, you need to set up local AO infrastructure:

```bash
# Install ArLocal globally
npm install -g arlocal

# Start ArLocal in a separate terminal
arlocal

# In another terminal, start your deployment
npm run deploy:local
```

### 4. Network Deployment

```bash
# Deploy to testnet
npm run deploy:testnet

# Deploy to mainnet (requires mainnet wallet with AR tokens)
npm run deploy:mainnet
```

## Configuration Files

- `.env.local` - Local development environment
- `.env.testnet` - AO testnet environment
- `.env.mainnet` - AO mainnet environment
- `.env.example` - Template for custom configuration

## Troubleshooting

### Local Development Issues
1. Ensure ArLocal is running on port 1984
2. Check that no other services are using ports 1984-1987
3. Verify wallet.json exists and is valid

### Network Issues
1. Ensure your wallet has sufficient AR tokens
2. Check network URLs are accessible
3. Verify AO network is operational

### Common Commands

```bash
# Generate new wallet
npm run wallet:generate

# Build WASM module
npm run build-wasm

# Test local deployment
npm run test-e2e

# Verify deployment
npm run verify:deployment
```
