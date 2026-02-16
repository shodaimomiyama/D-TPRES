# ArLocal E2E Test Investigation

## Background

Task 10 investigated running E2E tests for `ProductionAOClient` against a local AO environment.

## Finding: cwao-units is NOT ao-connect compatible

The `ao/` directory uses the `cwao-units` package, which exposes HTTP APIs that are **incompatible** with the ao-connect SDK that `ProductionAOClient` targets.

| API Endpoint | ao-connect (expected) | cwao-units (actual) |
|---|---|---|
| MU `POST /` | signed DataItem → `{ id: "msg_id" }` | Unknown format (no `id` field) |
| CU `GET /result/{id}` | Execution result JSON | Empty response |
| CU `POST /dry-run` | Dry-run execution result | **404 Not Found** |

### What works with cwao-units

- `yarn test` (mocha) in `ao/contracts/` uses the `cwao` library's `cw.e()` / `cw.q()` methods, which communicate via cwao-units-specific APIs.

### What does NOT work with cwao-units

- `ProductionAOClient` expects ao-connect compatible APIs (`POST /`, `GET /result/`, `POST /dry-run`).
- `scripts/start.js` starts ArLocal (port 1984) only; no separate MU/SU/CU HTTP servers.
- `test/utils.js` starts ArLocal (1994) + MU (1995) / SU (1996) / CU (1997), but the APIs are cwao-units-specific.

## Solution: ao-localnet (Docker Compose)

The official ao-connect compatible local environment is **[ao-localnet](https://github.com/permaweb/ao-localnet/)**.

### Endpoints

| Service | URL |
|---|---|
| ArLocal (gateway) | `http://localhost:4000` |
| MU | `http://localhost:4002` |
| SU | `http://localhost:4003` |
| CU | `http://localhost:4004` |

### Setup

```bash
git clone https://github.com/permaweb/ao-localnet.git
cd ao-localnet
cd wallets && ./generateAll.sh && ./printWalletAddresses.mjs && cd ..
docker compose up --detach
cd seed && ./download-aos-module.sh && ./seed-for-aos.sh
```

### Running E2E tests

```bash
AO_TEST_PROCESS_ID=<PROCESS_ID> \
AO_MU_URL=http://localhost:4002 \
AO_CU_URL=http://localhost:4004 \
AO_GATEWAY_URL=http://localhost:4000 \
ARWEAVE_WALLET_PATH=<path/to/wallet.json> \
cargo test --test e2e_arlocal --features production-ao -- --ignored --nocapture --test-threads=1
```

## Next Steps

1. Set up ao-localnet Docker environment
2. Deploy WASM contract to ao-localnet
3. Instantiate a process and obtain a Process ID
4. Run E2E tests against ao-localnet
