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
            owner_id: "owner_001",
            total_holders_n: 5,
            signer_pubkey: "owner_001_pubkey"
          }
        }
      })

      const response = await cw.q("get_owner_metadata", {})
      expect(response).to.have.property("metadata")
      expect(response.metadata).to.have.property("owner_id", "owner_001")
      expect(response.metadata).to.have.property("total_holders_n", 5)
      expect(response.metadata).to.have.property("creation_time")
    })

    it("should receive kFrags and distribute to holders", async () => {
      await cw.i({
        process_role: "Owner",
        metadata: {
          owner: {
            owner_id: "owner_002",
            total_holders_n: 3
          }
        }
      })

      // kFragsとsignatureをOwner-Processに送信（PRD PHASE 2）
      const result = await cw.e("receive_k_frags", {
        kfrags: [
          {
            kfrag_id: "kfrag_001",
            kfrag_data: new Array(32).fill(1), // 32-byte kFrag data
            signature: Buffer.from("sig_kfrag_001" + "0".repeat(51), 'utf8')   // Valid signature format
          },
          {
            kfrag_id: "kfrag_002",
            kfrag_data: new Array(32).fill(3),
            signature: Buffer.from("sig_kfrag_002" + "0".repeat(51), 'utf8')   // Valid signature format
          }
        ]
      })

      expect(result).to.be.ok
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
            owner_id: "test_owner",
            total_holders_n: 3,
            signer_pubkey: "test_signer_pubkey"
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
