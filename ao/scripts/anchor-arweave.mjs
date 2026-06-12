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

  const { TurboFactory } = await import("@ardrive/turbo-sdk");
  const turbo = TurboFactory.authenticated({ privateKey: jwk });

  const uploaded = [];
  for (const item of items) {
    console.log(`[anchor] uploading ${item.label} (${item.data.length} bytes)...`);
    const result = await turbo.uploadFile({
      fileStreamFactory: () => Readable.from(item.data),
      fileSizeFactory: () => item.data.length,
      dataItemOpts: { tags: item.tags },
    });
    console.log(`[anchor]   -> ${item.label}: ${result.id}`);
    uploaded.push({ label: item.label, id: result.id });
  }

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
