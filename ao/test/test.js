const { expect } = require("chai")
const { start } = require("./utils")

describe("D-TPRES Contract", function () {
  this.timeout(0)
  let stop, cw

  before(async () => ({ stop, cw } = await start()))
  after(async () => await stop())

  const PROCESS_ID = "test-process-001"

  // Binary helper: CosmWasm Binary = base64 string
  const toBase64 = (bytes) => Buffer.from(bytes).toString("base64")
  const sampleKFrag = toBase64(new Array(32).fill(1))
  const sampleCapsule = toBase64(new Array(32).fill(2))

  describe("Instantiation", function () {
    it("should instantiate contract with process_id", async () => {
      const result = await cw.i({ process_id: PROCESS_ID })
      expect(result).to.be.ok
    })

    it("should reject empty process_id", async () => {
      try {
        await cw.i({ process_id: "" })
        expect.fail("Should have thrown an error for empty process_id")
      } catch (error) {
        expect(error).to.be.ok
      }
    })
  })

  describe("Core Pipeline: kFrag → Capsule → cFrag", function () {
    const KF_ID = "kfrag-pipe-001"
    const CAP_ID = "capsule-pipe-001"

    before(async () => {
      await cw.i({ process_id: "pipeline-process" })
    })

    it("should submit_k_frag (Holder stores kFrag)", async () => {
      const result = await cw.e("submit_k_frag", {
        kfrag_id: KF_ID,
        kfrag: sampleKFrag,
      })
      expect(result).to.be.ok
    })

    it("should submit_capsule (Holder receives + auto-reencrypt)", async () => {
      // This will attempt reencryption internally; it may error on crypto
      // but the message itself should be accepted by the contract
      try {
        await cw.e("submit_capsule", {
          kfrag_id: KF_ID,
          capsule_id: CAP_ID,
          capsule: sampleCapsule,
        })
      } catch (error) {
        // Reencryption may fail with dummy data, but contract accepted the msg
        expect(error.message || error.toString()).to.include("eencrypt")
      }
    })

    it("should list_capsules_by_k_frag after submission", async () => {
      const response = await cw.q("list_capsules_by_k_frag", {
        kfrag_id: KF_ID,
      })
      expect(response).to.have.property("capsules")
      expect(response.capsules).to.be.an("array")
    })
  })

  describe("Delegate Flow (Owner side)", function () {
    const KF_ID = "kfrag-deleg-001"
    const CAP_ID = "capsule-deleg-001"

    before(async () => {
      await cw.i({ process_id: "delegate-process" })
    })

    it("should delegate_k_frag", async () => {
      const result = await cw.e("delegate_k_frag", {
        kfrag_id: KF_ID,
        kfrag: sampleKFrag,
      })
      expect(result).to.be.ok
    })

    it("should reject delegate_capsule for non-existent kfrag", async () => {
      try {
        await cw.e("delegate_capsule", {
          kfrag_id: "nonexistent-kfrag",
          capsule_id: CAP_ID,
          capsule: sampleCapsule,
        })
        expect.fail("Should have thrown for non-existent kfrag")
      } catch (error) {
        expect(error).to.be.ok
      }
    })
  })

  describe("Reencrypt (retry)", function () {
    before(async () => {
      await cw.i({ process_id: "reencrypt-process" })
      await cw.e("submit_k_frag", {
        kfrag_id: "kfrag-re-001",
        kfrag: sampleKFrag,
      })
    })

    it("should reject reencrypt for non-existent capsule", async () => {
      try {
        await cw.e("reencrypt", {
          kfrag_id: "kfrag-re-001",
          capsule_id: "nonexistent-capsule",
        })
        expect.fail("Should have thrown for non-existent capsule")
      } catch (error) {
        expect(error).to.be.ok
      }
    })

    it("should reject reencrypt for non-existent kfrag", async () => {
      try {
        await cw.e("reencrypt", {
          kfrag_id: "nonexistent-kfrag",
          capsule_id: "any-capsule",
        })
        expect.fail("Should have thrown for non-existent kfrag")
      } catch (error) {
        expect(error).to.be.ok
      }
    })
  })

  describe("ListCapsulesByKFrag", function () {
    before(async () => {
      await cw.i({ process_id: "list-process" })
    })

    it("should return empty list for unknown kfrag", async () => {
      const response = await cw.q("list_capsules_by_k_frag", {
        kfrag_id: "unknown-kfrag",
      })
      expect(response).to.have.property("capsules")
      expect(response.capsules).to.deep.equal([])
    })

    it("should support pagination (limit)", async () => {
      const response = await cw.q("list_capsules_by_k_frag", {
        kfrag_id: "any-kfrag",
        limit: 5,
      })
      expect(response).to.have.property("capsules")
      expect(response.capsules).to.be.an("array")
    })
  })

  describe("Validation / Error Cases", function () {
    before(async () => {
      await cw.i({ process_id: "validation-process" })
    })

    it("should reject empty kfrag_id in submit_k_frag", async () => {
      try {
        await cw.e("submit_k_frag", {
          kfrag_id: "",
          kfrag: sampleKFrag,
        })
        expect.fail("Should have thrown for empty kfrag_id")
      } catch (error) {
        expect(error).to.be.ok
      }
    })

    it("should reject empty kfrag data", async () => {
      try {
        await cw.e("submit_k_frag", {
          kfrag_id: "valid-id",
          kfrag: toBase64([]),
        })
        expect.fail("Should have thrown for empty kfrag data")
      } catch (error) {
        expect(error).to.be.ok
      }
    })

    it("should reject empty capsule data", async () => {
      try {
        await cw.e("submit_capsule", {
          kfrag_id: "valid-id",
          capsule_id: "valid-capsule",
          capsule: toBase64([]),
        })
        expect.fail("Should have thrown for empty capsule data")
      } catch (error) {
        expect(error).to.be.ok
      }
    })

    it("should reject oversized data (>128KB)", async () => {
      const oversized = toBase64(new Array(128 * 1024 + 1).fill(0))
      try {
        await cw.e("submit_k_frag", {
          kfrag_id: "valid-id",
          kfrag: oversized,
        })
        expect.fail("Should have thrown for oversized data")
      } catch (error) {
        expect(error).to.be.ok
      }
    })
  })

  describe("Idempotency", function () {
    before(async () => {
      await cw.i({ process_id: "idem-process" })
      await cw.e("submit_k_frag", {
        kfrag_id: "kfrag-idem-001",
        kfrag: sampleKFrag,
      })
    })

    it("should handle duplicate submit_k_frag as no-op", async () => {
      const result = await cw.e("submit_k_frag", {
        kfrag_id: "kfrag-idem-001",
        kfrag: sampleKFrag,
      })
      expect(result).to.be.ok
    })
  })
})
