const { wallet, module_path } = require("yargs")(
  process.argv.slice(2),
).demandOption(["wallet"]).argv

const { readFileSync } = require("fs")
const { resolve } = require("path")
const { CWAO } = require("cwao")

const getModule = async (
  module_path = "../contracts/target/wasm32-unknown-unknown/release/contract.wasm",
) => {
  const mpath =
    module_path[0] !== "/" ? resolve(__dirname, module_path) : module_path
  return readFileSync(mpath)
}

const deploy = async ({ module_path, wallet }) => {
  const _wallet = JSON.parse(readFileSync(resolve(wallet), "utf8"))
  const wasm = await getModule(module_path)
  const cwao = new CWAO({
    wallet: _wallet,
    arweave: { host: "arweave.net", port: 443, protocol: "https" },
    mu: "https://mu.ao-testnet.xyz",
    su: "https://su.ao-testnet.xyz",
    cu: "https://cu.ao-testnet.xyz",
  })
  const mod_id = await cwao.deploy(wasm)
  console.log(`Module deployed: ${mod_id}`)
}

deploy({ module_path, wallet })
