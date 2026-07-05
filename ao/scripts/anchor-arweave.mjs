#!/usr/bin/env node
// anchor-arweave.mjs — Anchor a FORMIX HyperBEAM deployment to Arweave.
//
// AO's trust model derives process state deterministically from
// (WASM module, ordered assignment log). Anchoring both to Arweave makes the
// deployment re-executable and verifiable by anyone running the pinned
// HyperBEAM build, independent of our node's availability.
//
// The pinned node's own scheduler upload silently fails (assignments are
// httpsig-coded and `bundler_httpsig` is unset — see issue #105), so this
// script exports the data from the node and uploads it as ANS-104 items via
// Turbo. The authenticity of the log comes from the scheduler signatures
// *inside* the exported data, so the upload signer does not need to be the
// node operator.
//
// Usage:
//   node ao/scripts/anchor-arweave.mjs --dry-run
//   ANCHOR_WALLET_PATH=/path/to/funded-jwk.json node ao/scripts/anchor-arweave.mjs
//
// Inputs (env overrides take precedence over ao/deploy.json):
//   HB_LOCAL_URL        node to export from        (default http://localhost:10000)
//   PROCESS_ID          process to anchor          (default ao/deploy.json .hyperbeam.process_id)
//   WASM_IMAGE_ID       image message ID           (default ao/deploy.json .hyperbeam.image_id)
//   ANCHOR_WALLET_PATH  Arweave JWK used to sign the uploads (Turbo credits
//                       required for items above the free threshold)

import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { Readable } from "node:stream";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const DRY_RUN = process.argv.includes("--dry-run");
const HB_LOCAL_URL = process.env.HB_LOCAL_URL ?? "http://localhost:10000";
const GATEWAY_GRAPHQL = process.env.ARWEAVE_GRAPHQL ?? "https://arweave.net/graphql";

async function loadDeployJson() {
  try {
    return JSON.parse(await readFile(path.join(REPO_ROOT, "ao", "deploy.json"), "utf8"));
  } catch {
    return {};
  }
}

async function fetchBytes(url) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`GET ${url} -> HTTP ${res.status}`);
  }
  return Buffer.from(await res.arrayBuffer());
}

async function main() {
  const deploy = (await loadDeployJson()).hyperbeam ?? {};
  const processId = process.env.PROCESS_ID ?? deploy.process_id;
  const imageId = process.env.WASM_IMAGE_ID ?? deploy.image_id;
  if (!processId || !imageId) {
    throw new Error("PROCESS_ID / WASM_IMAGE_ID not set and ao/deploy.json incomplete");
  }

  console.log(`[anchor] node:    ${HB_LOCAL_URL}`);
  console.log(`[anchor] process: ${processId}`);
  console.log(`[anchor] image:   ${imageId}`);

  // 1. Export the full assignment log (TABM multipart, scheduler-signed) and
  //    the WASM module bytes from the node.
  const schedule = await fetchBytes(`${HB_LOCAL_URL}/${processId}/schedule`);
  const module_ = await fetchBytes(`${HB_LOCAL_URL}/${imageId}`);
  if (!module_.subarray(0, 4).equals(Buffer.from([0x00, 0x61, 0x73, 0x6d]))) {
    throw new Error("exported image does not look like a WASM module");
  }

  const exportDir = path.join(REPO_ROOT, "ao", "anchor-export");
  await mkdir(exportDir, { recursive: true });
  const schedulePath = path.join(exportDir, `${processId}.schedule.tabm`);
  const modulePath = path.join(exportDir, `${imageId}.wasm`);
  await writeFile(schedulePath, schedule);
  await writeFile(modulePath, module_);
  console.log(`[anchor] exported schedule: ${schedulePath} (${schedule.length} bytes)`);
  console.log(`[anchor] exported module:   ${modulePath} (${module_.length} bytes)`);

  const items = [
    {
      label: "schedule-export",
      data: schedule,
      tags: [
        { name: "App-Name", value: "FORMIX" },
        { name: "Type", value: "FORMIX-Schedule-Export" },
        { name: "Data-Protocol", value: "ao" },
        { name: "Process", value: processId },
        { name: "Image", value: imageId },
        { name: "Content-Type", value: "multipart/form-data" },
      ],
    },
    {
      label: "wasm-module",
      data: module_,
      tags: [
        { name: "App-Name", value: "FORMIX" },
        { name: "Type", value: "FORMIX-Wasm-Module" },
        { name: "Data-Protocol", value: "ao" },
        { name: "Image", value: imageId },
        { name: "Content-Type", value: "application/wasm" },
      ],
    },
  ];

  if (DRY_RUN) {
    console.log("[anchor] --dry-run: skipping upload. Planned items:");
    for (const item of items) {
      console.log(`  - ${item.label}: ${item.data.length} bytes`);
    }
    console.log(
      "[anchor] Turbo free threshold is ~100KiB per item; larger items need a funded wallet.",
    );
    return;
  }

  const walletPath = process.env.ANCHOR_WALLET_PATH ?? process.env.WALLET_PATH;
  if (!walletPath) {
    throw new Error("ANCHOR_WALLET_PATH (or WALLET_PATH) must point to an Arweave JWK");
  }
  const jwk = JSON.parse(await readFile(walletPath, "utf8"));

  // Upload method: `turbo` (bundled, needs Turbo credits) or `l1` (direct
  // top-level Arweave transactions, paid in AR). `auto` (default) uses Turbo
  // when the account has enough credits, otherwise falls back to L1 — L1 is
  // also the stronger attribution (permanent on-chain txs, no bundler).
  const method = (process.env.ANCHOR_METHOD ?? "auto").toLowerCase();
  const uploaded =
    (await tryTurbo(jwk, items, method)) ?? (await uploadL1(jwk, items, method));

  // 2. Record the anchor IDs next to the deployment record.
  const deployJson = await loadDeployJson();
  deployJson.hyperbeam = {
    ...(deployJson.hyperbeam ?? {}),
    anchor: Object.fromEntries(uploaded.map(({ label, id }) => [label, id])),
  };
  await writeFile(
    path.join(REPO_ROOT, "ao", "deploy.json"),
    `${JSON.stringify(deployJson, null, 2)}\n`,
  );
  console.log("[anchor] recorded anchor IDs in ao/deploy.json");

  // 3. Verify the items are queryable on the gateway (indexing can lag).
  for (const { label, id } of uploaded) {
    const found = await pollGateway(id);
    console.log(`[anchor] gateway ${found ? "indexed" : "NOT YET indexed"}: ${label} (${id})`);
  }
}

// Upload via Turbo if the account has enough credits (or method forces it).
// Returns the uploaded list, or null to signal "fall back to L1".
async function tryTurbo(jwk, items, method) {
  if (method === "l1") return null;

  const { TurboFactory } = await import("@ardrive/turbo-sdk");
  const turbo = TurboFactory.authenticated({ privateKey: jwk });
  const totalBytes = items.reduce((n, it) => n + it.data.length, 0);

  const [{ winc: costWinc }] = await turbo.getUploadCosts({ bytes: [totalBytes] });
  const { winc: balanceWinc } = await turbo.getBalance();
  const enough = BigInt(balanceWinc) >= BigInt(costWinc);
  console.log(`[anchor] Turbo balance ${balanceWinc} winc, need ${costWinc} winc`);

  if (!enough) {
    if (method === "turbo") {
      throw new Error(
        `insufficient Turbo credits (${balanceWinc} < ${costWinc}); top up or use ANCHOR_METHOD=l1`,
      );
    }
    console.log("[anchor] insufficient Turbo credits — falling back to direct L1 upload");
    return null;
  }

  const uploaded = [];
  for (const item of items) {
    console.log(`[anchor] uploading ${item.label} via Turbo (${item.data.length} bytes)...`);
    const result = await turbo.uploadFile({
      fileStreamFactory: () => Readable.from(item.data),
      fileSizeFactory: () => item.data.length,
      dataItemOpts: { tags: item.tags },
    });
    console.log(`[anchor]   -> ${item.label}: ${result.id}`);
    uploaded.push({ label: item.label, id: result.id });
  }
  return uploaded;
}

// Upload each item as a direct top-level Arweave transaction, paid in AR.
async function uploadL1(jwk, items, method) {
  const { default: Arweave } = await import("arweave");
  const arweave = Arweave.init({ host: "arweave.net", port: 443, protocol: "https" });

  const address = await arweave.wallets.jwkToAddress(jwk);
  const balanceWinston = await arweave.wallets.getBalance(address);

  let totalCost = 0n;
  for (const item of items) {
    totalCost += BigInt(await arweave.transactions.getPrice(item.data.length));
  }
  console.log(
    `[anchor] L1 wallet ${address}: balance ${balanceWinston} winston, tx cost ~${totalCost} winston`,
  );
  if (BigInt(balanceWinston) < totalCost) {
    throw new Error(
      `insufficient AR for L1 upload (${balanceWinston} < ${totalCost} winston). Fund ${address}.`,
    );
  }

  const uploaded = [];
  for (const item of items) {
    console.log(`[anchor] uploading ${item.label} via L1 (${item.data.length} bytes)...`);
    const tx = await arweave.createTransaction({ data: item.data }, jwk);
    for (const tag of item.tags) tx.addTag(tag.name, tag.value);
    await arweave.transactions.sign(tx, jwk);
    const uploader = await arweave.transactions.getUploader(tx);
    while (!uploader.isComplete) {
      await uploader.uploadChunk();
      process.stdout.write(`\r[anchor]   ${item.label}: ${uploader.pctComplete}%   `);
    }
    process.stdout.write("\n");
    console.log(`[anchor]   -> ${item.label}: ${tx.id}`);
    uploaded.push({ label: item.label, id: tx.id });
  }
  return uploaded;
}

async function pollGateway(id, attempts = 10, intervalMs = 15_000) {
  const query = `{ transactions(ids: ["${id}"]) { edges { node { id } } } }`;
  for (let i = 0; i < attempts; i++) {
    try {
      const res = await fetch(GATEWAY_GRAPHQL, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ query }),
      });
      const body = await res.json();
      if (body?.data?.transactions?.edges?.length > 0) {
        return true;
      }
    } catch {
      // transient gateway errors: keep polling
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  return false;
}

main().catch((err) => {
  console.error(`[anchor] FAILED: ${err.message}`);
  process.exit(1);
});
