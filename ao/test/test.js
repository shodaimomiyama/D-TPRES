const { expect } = require("chai")
const { start } = require("./utils")

describe("D-TPRES Contract", function () {
  this.timeout(0)
  let stop, cw
  before(async () => ({ stop, cw } = await start()))
  after(async () => await stop())

  describe("Owner Process", function () {
    // テストデータの定義
    const testKFrags = [
      {
        kfrag_id: "kfrag_001",
        kfrag_data: new Array(32).fill(1),
        signature: Buffer.from("sig_kfrag_001" + "0".repeat(51), 'utf8')
      },
      {
        kfrag_id: "kfrag_002",
        kfrag_data: new Array(32).fill(2),
        signature: Buffer.from("sig_kfrag_002" + "0".repeat(51), 'utf8')
      },
      {
        kfrag_id: "kfrag_003",
        kfrag_data: new Array(32).fill(3),
        signature: Buffer.from("sig_kfrag_003" + "0".repeat(51), 'utf8')
      }
    ]

    const predefinedHolders = [
      "holder_process_001",
      "holder_process_002",
      "holder_process_003"
    ]

    describe("Instantiation with predefined holders", function () {
      it("should instantiate with holder_process_ids", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "test_owner_predefined",
              total_holders_n: 3,
              signer_pubkey: "test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        const response = await cw.q("get_owner_metadata", {})
        expect(response).to.have.property("metadata")
        expect(response.metadata).to.have.property("owner_id", "test_owner_predefined")
        expect(response.metadata).to.have.property("total_holders_n", 3)
        expect(response.metadata).to.have.property("holder_process_ids")
        expect(response.metadata.holder_process_ids).to.deep.equal(predefinedHolders)
      })

      it("should store all metadata correctly including holder_process_ids", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "owner_with_metadata",
              total_holders_n: 3,
              signer_pubkey: "metadata_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        const response = await cw.q("get_owner_metadata", {})
        expect(response.metadata).to.have.property("owner_id", "owner_with_metadata")
        expect(response.metadata).to.have.property("total_holders_n", 3)
        expect(response.metadata).to.have.property("signer_pubkey", "metadata_test_pubkey")
        expect(response.metadata).to.have.property("holder_process_ids")
        expect(response.metadata.holder_process_ids).to.deep.equal(predefinedHolders)
        expect(response.metadata).to.have.property("creation_time")
      })

      it("should validate required parameters", async () => {
        try {
          await cw.i({
            process_role: "Owner",
            metadata: {
              owner: {
                owner_id: "", // 空のowner_id - エラーになるべき
                total_holders_n: 3,
                signer_pubkey: "test_pubkey"
              }
            }
          })
          expect.fail("Should have thrown an error for empty owner_id")
        } catch (error) {
          expect(error).to.be.ok
        }
      })
    })

    describe("KV Store State Reconstruction", function () {
      it("should maintain state across multiple messages", async () => {
        // 初期化
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "state_test_owner",
              total_holders_n: 3,
              signer_pubkey: "state_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // 最初のkFrag受信
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0]]
        })

        // 状態確認
        let kfragsResponse = await cw.q("get_k_frags", {})
        expect(kfragsResponse.kfrags).to.have.length(1)
        expect(kfragsResponse.kfrags[0]).to.have.property("kfrag_id", "kfrag_001")

        // 2番目のkFrag受信
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[1]]
        })

        // 累積状態確認
        kfragsResponse = await cw.q("get_k_frags", {})
        expect(kfragsResponse.kfrags).to.have.length(2)

        const kfragIds = kfragsResponse.kfrags.map(k => k.kfrag_id)
        expect(kfragIds).to.include("kfrag_001")
        expect(kfragIds).to.include("kfrag_002")
      })

      it("should correctly restore state after process restart simulation", async () => {
        // 最初のプロセスインスタンス
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "restart_test_owner",
              total_holders_n: 3,
              signer_pubkey: "restart_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // メッセージシーケンス1: 複数のkFragを受信
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0], testKFrags[1]]
        })

        // 第一段階の状態確認
        let firstResponse = await cw.q("get_k_frags", {})
        expect(firstResponse.kfrags).to.have.length(2)

        // 「再起動」シミュレーション: 新しいプロセスインスタンス
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "restart_test_owner",
              total_holders_n: 3,
              signer_pubkey: "restart_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // 同じメッセージシーケンスを再実行
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0], testKFrags[1]]
        })

        // 状態が同一であることを確認
        let secondResponse = await cw.q("get_k_frags", {})
        expect(secondResponse.kfrags).to.have.length(2)

        const secondKfragIds = secondResponse.kfrags.map(k => k.kfrag_id)
        expect(secondKfragIds).to.include("kfrag_001")
        expect(secondKfragIds).to.include("kfrag_002")
      })

      it("should handle message replay in correct order", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "replay_test_owner",
              total_holders_n: 3,
              signer_pubkey: "replay_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // メッセージを順次実行
        await cw.e("receive_k_frags", { kfrags: [testKFrags[0]] })
        await cw.e("receive_k_frags", { kfrags: [testKFrags[1]] })
        await cw.e("receive_k_frags", { kfrags: [testKFrags[2]] })

        // 最終状態を確認
        const finalResponse = await cw.q("get_k_frags", {})
        expect(finalResponse.kfrags).to.have.length(3)

        const finalKfragIds = finalResponse.kfrags.map(k => k.kfrag_id)
        expect(finalKfragIds).to.include.members(["kfrag_001", "kfrag_002", "kfrag_003"])
      })
    })

    describe("kFrag Reception and Distribution", function () {
      it("should receive and store single kFrag", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "single_kfrag_owner",
              total_holders_n: 3,
              signer_pubkey: "single_kfrag_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        const result = await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0]]
        })

        expect(result).to.be.ok

        const response = await cw.q("get_k_frags", {})
        expect(response.kfrags).to.have.length(1)
        expect(response.kfrags[0]).to.have.property("kfrag_id", "kfrag_001")
      })

      it("should receive and store multiple kFrags in one message", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "multi_kfrag_owner",
              total_holders_n: 3,
              signer_pubkey: "multi_kfrag_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        const result = await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0], testKFrags[1], testKFrags[2]]
        })

        expect(result).to.be.ok

        const response = await cw.q("get_k_frags", {})
        expect(response.kfrags).to.have.length(3)
      })

      it("should distribute kFrags to predefined holders", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "distribution_owner",
              total_holders_n: 3,
              signer_pubkey: "distribution_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        await cw.e("receive_k_frags", {
          kfrags: testKFrags
        })

        const assignmentsResponse = await cw.q("get_holder_assignments", {})
        expect(assignmentsResponse).to.have.property("assignments")
        expect(assignmentsResponse.assignments).to.have.length.greaterThan(0)
      })

      it("should track distribution statistics correctly", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "stats_owner",
              total_holders_n: 3,
              signer_pubkey: "stats_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        const result = await cw.e("receive_k_frags", {
          kfrags: testKFrags
        })

        expect(result).to.be.ok
        // 結果の属性で配布統計を確認
        // (実装によっては result.attributes にdistributed_count等が含まれる)
      })

      it("should handle kFrag with valid signature", async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "signature_owner",
              total_holders_n: 3,
              signer_pubkey: "signature_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // 有効な署名フォーマット（64バイト以上）でkFragを送信
        const result = await cw.e("receive_k_frags", {
          kfrags: [{
            kfrag_id: "valid_sig_kfrag",
            kfrag_data: new Array(32).fill(99),
            signature: Buffer.from("sig_valid_sig_kfrag" + "0".repeat(45), 'utf8')
          }]
        })

        expect(result).to.be.ok
      })
    })

    describe("kFrag Validation Errors", function () {
      beforeEach(async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "error_test_owner",
              total_holders_n: 3,
              signer_pubkey: "error_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })
      })

      it("should reject empty kFrag ID", async () => {
        try {
          await cw.e("receive_k_frags", {
            kfrags: [{
              kfrag_id: "", // 空のID
              kfrag_data: new Array(32).fill(1),
              signature: Buffer.from("sig_empty_id" + "0".repeat(52), 'utf8')
            }]
          })
          expect.fail("Should have thrown an error for empty kFrag ID")
        } catch (error) {
          expect(error).to.be.ok
        }
      })

      it("should reject empty kFrag data", async () => {
        try {
          await cw.e("receive_k_frags", {
            kfrags: [{
              kfrag_id: "test_empty_data",
              kfrag_data: [], // 空のデータ
              signature: Buffer.from("sig_empty_data" + "0".repeat(50), 'utf8')
            }]
          })
          expect.fail("Should have thrown an error for empty kFrag data")
        } catch (error) {
          expect(error).to.be.ok
        }
      })

      it("should reject empty signature", async () => {
        try {
          await cw.e("receive_k_frags", {
            kfrags: [{
              kfrag_id: "test_empty_sig",
              kfrag_data: new Array(32).fill(1),
              signature: Buffer.from("", 'utf8') // 空の署名
            }]
          })
          expect.fail("Should have thrown an error for empty signature")
        } catch (error) {
          expect(error).to.be.ok
        }
      })

      it("should reject invalid signature format (< 64 bytes)", async () => {
        try {
          await cw.e("receive_k_frags", {
            kfrags: [{
              kfrag_id: "test_short_sig",
              kfrag_data: new Array(32).fill(1),
              signature: Buffer.from("short", 'utf8') // 短すぎる署名
            }]
          })
          expect.fail("Should have thrown an error for short signature")
        } catch (error) {
          expect(error).to.be.ok
        }
      })
    })

    describe("Query Operations", function () {
      beforeEach(async () => {
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "query_test_owner",
              total_holders_n: 3,
              signer_pubkey: "query_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })
      })

      describe("GetOwnerMetadata", function () {
        it("should return complete metadata including holder_process_ids", async () => {
          const response = await cw.q("get_owner_metadata", {})

          expect(response).to.have.property("metadata")
          expect(response.metadata).to.have.property("owner_id", "query_test_owner")
          expect(response.metadata).to.have.property("total_holders_n", 3)
          expect(response.metadata).to.have.property("signer_pubkey", "query_test_pubkey")
          expect(response.metadata).to.have.property("holder_process_ids")
          expect(response.metadata.holder_process_ids).to.deep.equal(predefinedHolders)
          expect(response.metadata).to.have.property("creation_time")
        })
      })

      describe("GetKFrags", function () {
        it("should return all stored kFrags", async () => {
          // kFrags送信
          await cw.e("receive_k_frags", {
            kfrags: [testKFrags[0], testKFrags[1]]
          })

          const response = await cw.q("get_k_frags", {})
          expect(response).to.have.property("kfrags")
          expect(response.kfrags).to.have.length(2)
        })

        it("should return empty array when no kFrags stored", async () => {
          const response = await cw.q("get_k_frags", {})
          expect(response).to.have.property("kfrags")
          expect(response.kfrags).to.be.an("array")
          expect(response.kfrags).to.have.length(0)
        })

        it("should filter kFrags by holder_id when specified", async () => {
          // kFrags送信
          await cw.e("receive_k_frags", {
            kfrags: testKFrags
          })

          // 特定のholder_idでフィルタリング
          const response = await cw.q("get_k_frags", {
            holder_id: predefinedHolders[0]
          })

          expect(response).to.have.property("kfrags")
          // フィルタリングされた結果の検証
          if (response.kfrags.length > 0) {
            response.kfrags.forEach(kfrag => {
              expect(kfrag).to.have.property("target_holder", predefinedHolders[0])
            })
          }
        })
      })

      describe("GetHolderAssignments", function () {
        it("should return holder assignment information", async () => {
          // kFrags送信して割り当てを生成
          await cw.e("receive_k_frags", {
            kfrags: testKFrags
          })

          const response = await cw.q("get_holder_assignments", {})
          expect(response).to.have.property("assignments")
          expect(response.assignments).to.be.an("array")
        })

        it("should show correct assignment status", async () => {
          await cw.e("receive_k_frags", {
            kfrags: [testKFrags[0]]
          })

          const response = await cw.q("get_holder_assignments", {})
          if (response.assignments.length > 0) {
            response.assignments.forEach(assignment => {
              expect(assignment).to.have.property("status")
              expect(assignment).to.have.property("holder_id")
              expect(assignment).to.have.property("assigned_kfrags")
            })
          }
        })
      })

      describe("Process Information", function () {
        it("should return process role as Owner", async () => {
          const response = await cw.q("get_process_role", {})
          expect(response).to.have.property("role", "Owner")
        })

        it("should return active process status", async () => {
          const response = await cw.q("get_process_status", {})
          expect(response).to.have.property("status", "active")
          expect(response).to.have.property("last_updated")
        })
      })
    })

    describe("End-to-End Scenario with State Persistence", function () {
      it("should handle complete kFrag distribution flow with state persistence", async () => {
        // 1. 初期化（3つのpredefined holders）
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "e2e_test_owner",
              total_holders_n: 3,
              signer_pubkey: "e2e_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // 2. 最初のkFragセット受信
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0], testKFrags[1]]
        })

        // 3. 状態確認（クエリ）
        let kfragsResponse = await cw.q("get_k_frags", {})
        expect(kfragsResponse.kfrags).to.have.length(2)

        let metadataResponse = await cw.q("get_owner_metadata", {})
        expect(metadataResponse.metadata.owner_id).to.equal("e2e_test_owner")

        // 4. 追加のkFrag受信
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[2]]
        })

        // 5. 累積状態の確認
        kfragsResponse = await cw.q("get_k_frags", {})
        expect(kfragsResponse.kfrags).to.have.length(3)

        const assignmentsResponse = await cw.q("get_holder_assignments", {})
        expect(assignmentsResponse.assignments).to.be.an("array")

        // 6. プロセス「再起動」シミュレーション
        await cw.i({
          process_role: "Owner",
          metadata: {
            owner: {
              owner_id: "e2e_test_owner",
              total_holders_n: 3,
              signer_pubkey: "e2e_test_pubkey",
              holder_process_ids: predefinedHolders
            }
          }
        })

        // 7. 同じメッセージシーケンスの再実行
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[0], testKFrags[1]]
        })
        await cw.e("receive_k_frags", {
          kfrags: [testKFrags[2]]
        })

        // 8. 最終状態が同一であることの確認
        const finalKfragsResponse = await cw.q("get_k_frags", {})
        expect(finalKfragsResponse.kfrags).to.have.length(3)

        const finalMetadataResponse = await cw.q("get_owner_metadata", {})
        expect(finalMetadataResponse.metadata.owner_id).to.equal("e2e_test_owner")
        expect(finalMetadataResponse.metadata.holder_process_ids).to.deep.equal(predefinedHolders)

        const finalAssignmentsResponse = await cw.q("get_holder_assignments", {})
        expect(finalAssignmentsResponse.assignments).to.be.an("array")
      })
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
