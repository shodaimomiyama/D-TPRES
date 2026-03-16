/**
 * FORMIX AO Contract Deployment - HyperBEAM (~wasm64@1.0)
 *
 * Replaces ao_cwao/scripts/deploy.js which used cwao (legacy).
 * Uses @permaweb/aoconnect for HyperBEAM-compatible deployment.
 *
 * Usage:
 *   node scripts/deploy.js --wallet <path-to-jwk.json>
 *
 * TODO: Verify wasm32 vs wasm64 target with the HyperBEAM node being used.
 */

import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { createDataItemSigner, spawn, message, result } from "@permaweb/aoconnect"
import Arweave from "arweave"

const __dirname = dirname(fileURLToPath(import.meta.url))

// ─── Configuration ────────────────────────────────────────────────────────────
// TODO: Replace with production HyperBEAM node URL
const AO_NODE_URL = process.env.AO_NODE_URL || "https://cu.ao-testnet.xyz"

// HyperBEAM process scheduler URL
const SCHEDULER_URL = process.env.AO_SCHEDULER_URL || "https://su.ao-testnet.xyz"

const WASM_PATH = resolve(
  __dirname,
  "../contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm"
)

// ─── Deploy ───────────────────────────────────────────────────────────────────
async function deploy({ walletPath }) {
  console.log("Loading wallet...")
  const wallet = JSON.parse(readFileSync(walletPath, "utf-8"))
  const signer = createDataItemSigner(wallet)

  console.log("Loading WASM binary:", WASM_PATH)
  const wasmBinary = readFileSync(WASM_PATH)
  console.log(`WASM size: ${wasmBinary.length} bytes`)

  // Upload WASM binary to Arweave first
  console.log("Uploading WASM to Arweave...")
  const arweave = Arweave.init({ host: "arweave.net", port: 443, protocol: "https" })
  const tx = await arweave.createTransaction({ data: wasmBinary }, wallet)
  tx.addTag("Content-Type", "application/wasm")
  tx.addTag("App-Name", "FORMIX")
  tx.addTag("Contract-Type", "formix-ao-native")
  await arweave.transactions.sign(tx, wallet)
  const uploadResult = await arweave.transactions.post(tx)
  console.log("WASM uploaded, tx ID:", tx.id)

  if (uploadResult.status !== 200) {
    throw new Error(`Arweave upload failed: ${uploadResult.status}`)
  }

  // Spawn AO process with WASM module
  console.log("Spawning AO process...")
  const processId = await spawn({
    module: tx.id,  // Arweave TX ID of the WASM binary
    scheduler: SCHEDULER_URL,
    signer,
    tags: [
      { name: "App-Name", value: "FORMIX" },
      { name: "Contract-Version", value: "2.0.0-hyperbeam" },
      // wasm64 device specification
      // TODO: Verify exact tag name with HyperBEAM docs
      { name: "Execution-Device", value: "wasm64@1.0" },
    ],
  })

  console.log("Process spawned:", processId)
  console.log("Waiting for process to be ready...")

  // Wait a few seconds for the process to initialize
  await new Promise(r => setTimeout(r, 3000))

  // Test with a simple ping
  console.log("Testing process...")
  const msgId = await message({
    process: processId,
    signer,
    tags: [{ name: "Action", value: "Ping" }],
  })

  const testResult = await result({ process: processId, message: msgId })
  console.log("Test result:", testResult)

  return {
    processId,
    wasmTxId: tx.id,
  }
}

// ─── CLI ──────────────────────────────────────────────────────────────────────
const args = process.argv.slice(2)
const walletIdx = args.indexOf("--wallet")
if (walletIdx === -1 || !args[walletIdx + 1]) {
  console.error("Usage: node scripts/deploy.js --wallet <path-to-jwk.json>")
  process.exit(1)
}

deploy({ walletPath: args[walletIdx + 1] })
  .then(({ processId, wasmTxId }) => {
    console.log("\n=== Deployment successful ===")
    console.log("Process ID:", processId)
    console.log("WASM TX ID:", wasmTxId)
    console.log("\nAdd to .env:")
    console.log(`AO_PROCESS_ID=${processId}`)
    console.log(`AO_WASM_TX_ID=${wasmTxId}`)
  })
  .catch(err => {
    console.error("Deployment failed:", err)
    process.exit(1)
  })
