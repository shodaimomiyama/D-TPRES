# D-TPRES MVP - AO Network Deployment Guide

## 🎯 Overview

This is the Minimum Viable Product (MVP) implementation of D-TPRES (Deterministic Threshold Proxy Re-Encryption System) for the AO Network hackathon. The MVP demonstrates the core functionality of threshold proxy re-encryption with O-Browser and Owner-Process components.

## 🚀 Quick Start

### Prerequisites

- Rust (with wasm32-unknown-unknown target)
- Node.js (v16+)
- Arweave wallet with AR tokens
- npm or yarn

### 1. Build the WebAssembly Module

```bash
# Install wasm32 target if not already installed
rustup target add wasm32-unknown-unknown

# Build the WASM module
./build.sh
```

### 2. Setup Deployment

```bash
cd deploy

# Install dependencies with yarn
yarn install

# Create your .env file from the example
cp .env.example .env

# Add your Arweave wallet
# Download from https://arweave.app/wallet and save as wallet.json
```

### 2.5. Get Testnet Tokens

```bash
# Request testnet AR tokens
npm run faucet

# This will show your wallet address and provide links to faucets:
# - https://faucet.arweave.dev/
# - https://faucet.arconnect.io/
# - https://bundlr.network/faucet

# After requesting tokens manually, check your balance:
npm run faucet
```

### 3. Deploy to AO Network

```bash
# Deploy Owner-Process to testnet
npm run deploy:owner

# Or deploy to mainnet
npm run deploy:owner -- --network mainnet
```

### 4. Test the Deployment

```bash
# Run automated tests
npm run test
```

### 5. CLI Testing

After deployment, you can test the module functionality using the CLI test scripts:

```bash
# Test with direct AO messages
node test-direct.js

# This will send the following messages to your deployed module:
# - ProcessInfo: Query process information
# - SetupEncryption: Initialize encryption with secret
# - StoreKfrags: Store key fragments
# - GetKfrags: Query stored fragments
# - ProcessAccessRequest: Request access to encrypted data
```

#### Example Output

```
🧪 Direct AO Message Test
=========================

Module/Process ID: hRvavW-bnwkFiEPEhHgLw5QjQ6nI0_pHMtNdlaZl-gY

📤 Sending test messages to AO module...

1️⃣ Query Process Info
✅ Message sent: ukvfvIpnLOeGCym4kbfSSXEMdHq7zIVNaMIBk8AUkiA

2️⃣ Setup Encryption
✅ Message sent: EjSlNTMQ_WT-nhKgM2Wujem_a3FNiu-BPguoIoP3cEM

3️⃣ Store kFrags
✅ Message sent: CMYQim51A2FPeNVkzVh66bkSmsSEZJ0YZaQwSVKLvKM

4️⃣ Query kFrags
✅ Message sent: NY8g6-UpX3JTGDaHqUiHOnkgBV-n8Y-3FjnfdXhE9rc

5️⃣ Process Access Request
✅ Message sent: Jdi0j6SF6hRU5mzW6EGnUqMFJOvKJUIpKaQkvQKOCgY
```

#### Verify Messages on AO Network

After running the CLI test, you can verify your messages:

1. **Check individual transactions**: 
   - Visit the Arweave transaction URLs shown in the output
   - Example: `https://arweave.net/tx/[MESSAGE_ID]`

2. **View on AO Explorer**:
   - Visit: `https://www.ao.link/#/message/[MODULE_ID]`
   - This shows all messages sent to your module

3. **Check module status**:
   - Run: `node check-module.js`
   - This verifies your module is accessible on various gateways

**Note**: Message processing on AO network may take 1-2 minutes. The CLI test sends messages but doesn't wait for responses.

## 📦 MVP Features

### Implemented Features

1. **O-Browser Functionality** (Simulated in Owner-Process)
   - Key pair generation (sk_O, pk_O)
   - Secret splitting using Shamir's Secret Sharing
   - Share encryption with symmetric key k_O
   - Capsule creation (k_O encrypted with pk_O)
   - Re-encryption key generation
   - kFrag creation and distribution

2. **Owner-Process**
   - kFrag storage and management
   - Access request processing (auto-approval for MVP)
   - CosmWasm contract interface
   - AO message handling

3. **Cryptographic Operations**
   - Umbral PRE implementation
   - Shamir's Secret Sharing
   - Key fragment management

### Simplified for MVP

- **Auto-generated Requester Keys**: pk_A is automatically generated instead of being shared
- **Auto-approval**: Access requests are automatically approved without smart contract verification
- **In-memory Storage**: Uses in-memory storage instead of Arweave for testing

## 🏗️ Architecture

```
src/
├── lib.rs                 # CosmWasm entry points
├── service/
│   └── core/
│       └── crypto.rs      # Cryptographic operations (existing)
├── usecase/
│   └── owner/
│       └── handlers.rs    # Owner-Process message handlers
├── ao/
│   ├── mod.rs            # AO integration
│   ├── message.rs        # Message routing
│   └── state.rs          # State management
└── domain/               # Domain entities
```

## 📝 Usage Examples

### Setup Encryption (O-Browser Simulation)

```javascript
// Send SetupEncryption message to Owner-Process
const message = {
  action: 'SetupEncryption',
  input: {
    secret: [/* secret bytes */],
    threshold: 2,
    total_shares: 3
  }
};

await cwao.message({
  process: processId,
  data: message,
  tags: [{ name: 'Action', value: 'SetupEncryption' }]
});
```

### Query kFrags

```javascript
// Query stored kFrags
const result = await cwao.query({
  process: processId,
  data: { query: 'GetKfrags' }
});

console.log(`kFrags available: ${result.kfrags.length}`);
```

### Process Access Request

```javascript
// Request access to encrypted data
const message = {
  action: 'ProcessAccessRequest',
  input: {
    requester_id: 'requester_123',
    capsule_id: 'capsule_456'
  }
};

await cwao.message({
  process: processId,
  data: message,
  tags: [{ name: 'Action', value: 'ProcessAccessRequest' }]
});
```

## 🔍 Monitoring

Monitor your deployed process on AO Explorer:
```
https://ao.arweave.dev/#/process/[YOUR_PROCESS_ID]
```

## 🛠️ Development

### Run Tests

```bash
# Rust tests
cargo test

# Integration tests
cd deploy && npm test
```

### Local Development

```bash
# Check code
make check

# Format code
make fmt

# Run linter
make clippy

# All checks
make all
```

## 📚 Documentation

For detailed documentation, see:
- `docs/PRD.md` - Product Requirements
- `docs/development/` - Development documentation
- `CLAUDE.md` - AI assistant instructions

## 🚨 Important Notes

1. **Testnet First**: Always test on testnet before mainnet deployment
2. **Wallet Security**: Keep your Arweave wallet secure
3. **Gas Costs**: Ensure sufficient AR tokens for deployment
4. **MVP Limitations**: This is a simplified MVP - not production-ready

## 🎉 Next Steps

After successful MVP deployment:

1. **Test the core functionality**: Verify encryption, kFrag generation, and access control
2. **Integrate with frontend**: Build O-Browser as a proper web interface
3. **Implement Holder-Process**: Add the holder role for distributed storage
4. **Add EVM verification**: Integrate smart contract access control
5. **Production hardening**: Implement proper error handling and security measures

## ⚠️ Troubleshooting

### Common Issues

1. **getrandom WASM compilation error**
   - Solution: The project uses conditional compilation to exclude crypto libraries from WASM build
   - Crypto operations are mocked in WASM and should be performed off-chain

2. **rust-toolchain.toml conflicts**
   - Solution: The build script automatically handles this by temporarily moving the file

3. **Package not found errors**
   - Solution: Use yarn instead of npm for better compatibility
   - Run `yarn install` in the deploy directory

4. **Build failures**
   - Ensure you're using Rust edition 2021
   - Check that wasm32-unknown-unknown target is installed
   - Try cleaning with `cargo clean` before rebuilding

### WASM Limitations

- **Cryptographic operations**: All actual crypto operations (Umbral PRE, Shamir) are mocked in WASM
- **O-Browser responsibility**: Real encryption happens in the browser (local environment)
- **Owner-Process role**: Only stores and manages kFrags, doesn't perform actual crypto

## 📄 License

MIT License - See LICENSE file in the root directory.

## 🤝 Support

For issues or questions:
- GitHub Issues: [Create an issue](https://github.com/your-repo/issues)
- Documentation: See `docs/` directory

---

**Built for AO Network Hackathon** 🚀