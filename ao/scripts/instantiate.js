const { wallet, module_id, scheduler, input } = require("yargs")(
  process.argv.slice(2),
).demandOption(["wallet", "module_id", "scheduler"]).argv

const { readFileSync } = require("fs")
const { resolve } = require("path")
const { CWAO } = require("cwao")

const deploy = async ({ module_id, input, scheduler, wallet }) => {
  const _wallet = JSON.parse(readFileSync(resolve(wallet), "utf8"))
  const cwao = new CWAO({
    wallet: _wallet,
    arweave: { host: "arweave.net", port: 443, protocol: "https" },
    mu: "https://mu.ao-testnet.xyz",
    su: "https://su.ao-testnet.xyz",
    cu: "https://cu.ao-testnet.xyz",
  })
  console.log(input)
  const { error, id } = await cwao.instantiate({
    module: module_id,
    scheduler,
    input: JSON.parse(input ?? "{}"),
  })
  console.log({
    module: module_id,
    scheduler,
    input: JSON.parse(input ?? "{}"),
  })
  if (error) {
    console.log(`something went wrong`, error)
  } else {
    console.log(`Process instantiated: ${id}`)
  }
}

deploy({ module_id, scheduler, input, wallet })
