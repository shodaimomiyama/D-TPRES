const Arweave = require("arweave")
const { CU } = require("cwao-units")
const { resolve } = require("path")
const { mkdirs, keygen } = require("./utils")

const dir = resolve(__dirname, "../.cwao")
const dir_acc = resolve(__dirname, "../.cwao/accounts")
const dirs = [dir, dir_acc]

const start = async () => {
  await mkdirs(dirs)
  const network = {
    host: "arweave.net",
    port: 443,
    protocol: "https",
  }
  const arweave = Arweave.init(network)
  const wallet = await keygen("cu", dir, arweave)

  const cu = new CU({
    port: 1987,
    arweave: network,
    graphql: "https://arweave.net/graphql",
    wallet,
  })

  console.log("CWAO CU started on http://localhost:1987")
  console.log("Using Arweave gateway: https://arweave.net")
  console.log("Press Ctrl+C to stop")
}

start().catch(console.error)
