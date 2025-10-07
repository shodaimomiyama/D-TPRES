const { expect } = require("chai")
const { start } = require("./utils")

describe("D-TPRES Contract", function () {
  this.timeout(0)
  let stop, cw
  before(async () => ({ stop, cw } = await start()))
  after(async () => await stop())

  describe("Owner Process", function () {
    it("should instantiate as Owner and return metadata", async () => {
      await cw.i({
        process_role: "Owner",
        metadata: {
          owner: {
            threshold_k: 3,
            total_holders_n: 5,
            capsule_txid: "test_capsule_txid_123",
            requester_pubkey: "test_requester_pubkey_456"
          }
        }
      })

      const response = await cw.q("get_owner_metadata", {})
      expect(response).to.have.property("metadata")
      expect(response.metadata).to.have.property("threshold_k", 3)
      expect(response.metadata).to.have.property("total_holders_n", 5)
      expect(response.metadata).to.have.property("capsule_txid", "test_capsule_txid_123")
      expect(response.metadata).to.have.property("requester_pubkey", "test_requester_pubkey_456")
    })
  })

  describe("Holder Process", function () {
    it("should instantiate as Holder and return metadata", async () => {
      await cw.i({
        process_role: "Holder",
        metadata: {
          holder: {
            holder_id: "holder_001"
          }
        }
      })

      const response = await cw.q("get_holder_metadata", {})
      expect(response).to.have.property("metadata")
      expect(response.metadata).to.have.property("holder_id", "holder_001")
      expect(response.metadata).to.have.property("process_role", "Holder")
    })
  })

  describe("Requester Process", function () {
    it("should instantiate as Requester and return metadata", async () => {
      await cw.i({
        process_role: "Requester",
        metadata: {
          requester: {
            requester_id: "requester_001"
          }
        }
      })

      const response = await cw.q("get_requester_metadata", {})
      expect(response).to.have.property("metadata")
      expect(response.metadata).to.have.property("requester_id", "requester_001")
      expect(response.metadata).to.have.property("process_role", "Requester")
    })
  })

  describe("Common Queries", function () {
    it("should return process role after instantiation", async () => {
      await cw.i({
        process_role: "Owner",
        metadata: {
          owner: {
            threshold_k: 2,
            total_holders_n: 3,
            capsule_txid: "test_capsule",
            requester_pubkey: "test_pubkey"
          }
        }
      })

      const roleResponse = await cw.q("get_process_role", {})
      expect(roleResponse).to.have.property("role", "Owner")
    })

    it("should return process status", async () => {
      const statusResponse = await cw.q("get_process_status", {})
      expect(statusResponse).to.have.property("status", "active")
      expect(statusResponse).to.have.property("last_updated")
    })
  })
})
