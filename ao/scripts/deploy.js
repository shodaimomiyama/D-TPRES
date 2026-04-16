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

// AO scheduler wallet address (NOT a URL).
// Default: ao-testnet default scheduler.
const SCHEDULER = process.env.AO_SCHEDULER || "_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA"

const WASM_PATH = resolve(
  __dirname,
  "../contracts/target/wasm32-unknown-unknown/release/formix_contract.wasm"
)

// ─── Helpers ──────────────────────────────────────────────────────────────────
const GRAPHQL_URL = process.env.GRAPHQL_URL || "https://arweave.net/graphql"

async function queryModule(txId) {
  const query = `
    query($ids: [ID!]!) {
      transactions(ids: $ids) {
        edges { node { id tags { name value } } }
      }
    }`
  const res = await fetch(GRAPHQL_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables: { ids: [txId] } }),
  })
  if (!res.ok) return null
  const json = await res.json()
  return json?.data?.transactions?.edges?.[0]?.node || null
}

// Arweave mainnet mining is typically 1-2 minutes. We poll for up to ~10 min.
async function waitForModuleIndexed(txId, { intervalMs = 10_000, timeoutMs = 600_000 } = {}) {
  const start = Date.now()
  let attempt = 0
  while (Date.now() - start < timeoutMs) {
    attempt++
    const node = await queryModule(txId)
    if (node) {
      const hasModuleType = node.tags?.some(
        t => t.name === "Type" && t.value === "Module"
      )
      if (hasModuleType) {
        console.log(`Module indexed after ${attempt} attempt(s)`)
        return
      }
    }
    const elapsed = Math.round((Date.now() - start) / 1000)
    console.log(`  ...not yet indexed (${elapsed}s elapsed, attempt ${attempt})`)
    await new Promise(r => setTimeout(r, intervalMs))
  }
  throw new Error(
    `Module ${txId} was not indexed within ${timeoutMs / 1000}s. ` +
      `Check https://viewblock.io/arweave/tx/${txId} for status.`
  )
}

// ─── Deploy ───────────────────────────────────────────────────────────────────
async function deploy({ walletPath }) {
  console.log("Loading wallet...")
  const wallet = JSON.parse(readFileSync(walletPath, "utf-8"))
  const signer = createDataItemSigner(wallet)

  console.log("Loading WASM binary:", WASM_PATH)
  const wasmBinary = readFileSync(WASM_PATH)
  console.log(`WASM size: ${wasmBinary.length} bytes`)

  // Upload WASM as an AO Module. The tags below are required by AO so that
  // `spawn({ module })` can instantiate a process from this TX.
  console.log("Uploading WASM as AO Module to Arweave...")
  const arweave = Arweave.init({ host: "arweave.net", port: 443, protocol: "https" })
  const tx = await arweave.createTransaction({ data: wasmBinary }, wallet)
  tx.addTag("Data-Protocol", "ao")
  tx.addTag("Variant", "ao.TN.1")
  tx.addTag("Type", "Module")
  tx.addTag("Module-Format", "wasm32-unknown-unknown")
  tx.addTag("Input-Encoding", "JSON-1")
  tx.addTag("Output-Encoding", "JSON-1")
  tx.addTag("Memory-Limit", "500-mb")
  tx.addTag("Compute-Limit", "9000000000000")
  tx.addTag("Content-Type", "application/wasm")
  tx.addTag("App-Name", "FORMIX")
  tx.addTag("Contract-Type", "formix-ao-native")
  await arweave.transactions.sign(tx, wallet)
  const uploadResult = await arweave.transactions.post(tx)
  console.log("Module uploaded, tx ID:", tx.id)

  if (uploadResult.status !== 200) {
    throw new Error(`Arweave upload failed: ${uploadResult.status}`)
  }

  // aoconnect's spawn() fetches the module TX via arweave.net/graphql. A freshly
  // posted tx is not yet visible there (mainnet mining takes minutes), so we
  // poll until GraphQL returns the edge with matching Type=Module tag.
  await waitForModuleIndexed(tx.id)

  // Spawn AO process with WASM module
  console.log("Spawning AO process...")
  const processId = await spawn({
    module: tx.id,
    scheduler: SCHEDULER,
    signer,
    tags: [
      { name: "App-Name", value: "FORMIX" },
      { name: "Contract-Version", value: "2.0.0-hyperbeam" },
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
