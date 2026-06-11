//! ContractStorage - AO contract communication service (HyperBEAM-native)
//!
//! Updated for new `ao/` module:
//! - `ExecuteMsg::*` enum variants → `AOExecuteMsg::*` builder methods
//! - `QueryMsg::*` → `AOQueryMsg::*` builder methods
//! - `AOResponse` → `AONativeResponse`
//! - `GetCFragResponse` parses `{ kfrag_id, capsule_id, cfrag: [u8] }` format

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::adapter::external::ao::{AOClient, AOExecuteMsg, AOQueryMsg, GetCFragsResponse};
use crate::service::error::{ServiceError, ServiceResult};
use crate::usecase::core::crypto::{CFragData, KeyFragment};

/// Bincode-serializable payload matching the contract's `StoredKeyFrag` layout.
/// The contract validates incoming kFrag bytes via `bincode::deserialize::<StoredKeyFrag>`,
/// so the client must send the full structure — not just `key_data`.
#[derive(Serialize, Deserialize)]
struct StoredKeyFragPayload {
    id: String,
    key_data: Vec<u8>,
    verification_data: Vec<u8>,
    precursor: Vec<u8>,
}

/// Contract storage trait for AO Network communication.
#[async_trait]
pub trait ContractStorage: Send + Sync {
    /// Send kFrags to Owner-Process via AO Network.
    async fn send_kfrags(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()>;

    /// Delegate capsule to AO contract for re-encryption triggering.
    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()>;

    /// Retrieve cFrags for a secret from AO Network.
    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>>;
}

/// ContractStorage implementation using AOClient (HyperBEAM-native).
pub struct ContractStorageImpl<A: AOClient> {
    ao_client: Arc<A>,
    /// Default holder process ID for DelegateCapsule.
    /// In HyperBEAM, Owner-Process delegates to a Holder-Process via AO message.
    /// Set to the same process_id for single-process setups.
    holder_process_id: String,
}

impl<A: AOClient> ContractStorageImpl<A> {
    /// Create a new ContractStorageImpl.
    ///
    /// # DI wiring note
    /// This constructor signature differs from the old `ContractStorageImpl::new(ao_client)`.
    /// `holder_process_id` was added for the HyperBEAM two-process model (Owner + Holder).
    ///
    /// **TODO**: Update `di.rs` (dependency injection wiring) to pass `holder_process_id`.
    /// For single-process setups (Owner == Holder), use `new_single_process()` instead.
    ///
    /// Tracked in: PR #83 todo list.
    pub fn new(ao_client: Arc<A>, holder_process_id: impl Into<String>) -> Self {
        Self {
            ao_client,
            holder_process_id: holder_process_id.into(),
        }
    }

    /// Convenience constructor for single-process setups (holder = owner).
    /// Holder process ID defaults to the contract_id passed at call time.
    pub fn new_single_process(ao_client: Arc<A>) -> Self {
        Self {
            ao_client,
            holder_process_id: String::new(),
        }
    }
}

#[async_trait]
impl<A: AOClient> ContractStorage for ContractStorageImpl<A> {
    async fn send_kfrags(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()> {
        for kfrag in kfrags {
            let kfrag_id = format!("{secret_id}_{}", kfrag.id);
            let holder = if self.holder_process_id.is_empty() {
                contract_id.to_string()
            } else {
                self.holder_process_id.clone()
            };
            let payload = StoredKeyFragPayload {
                id: kfrag_id.clone(),
                key_data: kfrag.key_data.clone(),
                verification_data: kfrag.verification_data.clone(),
                precursor: kfrag.precursor.clone(),
            };
            let kfrag_bytes = bincode::serialize(&payload).map_err(|e| {
                ServiceError::validation_error(format!("kFrag serialization failed: {e}"))
            })?;
            let msg = AOExecuteMsg::delegate_kfrag(&kfrag_id, &kfrag_bytes, holder);
            let resp = self
                .ao_client
                .execute(contract_id, msg)
                .await
                .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;

            // Guard: transport succeeded but contract returned logical error
            if !resp.ok {
                let reason = resp
                    .error
                    .unwrap_or_else(|| "DelegateKFrag returned ok:false".into());
                return Err(ServiceError::ao_network_error(format!(
                    "kfrag_id={kfrag_id}: {reason}"
                )));
            }
        }
        Ok(())
    }

    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()> {
        // holder_process_id defaults to contract_id for single-process setups
        let holder = if self.holder_process_id.is_empty() {
            contract_id.to_string()
        } else {
            self.holder_process_id.clone()
        };

        let msg = AOExecuteMsg::delegate_capsule(kfrag_id, capsule_id, capsule_data, holder);
        let resp = self
            .ao_client
            .execute(contract_id, msg)
            .await
            .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;

        // Guard: transport succeeded but contract returned logical error
        if !resp.ok {
            let reason = resp
                .error
                .unwrap_or_else(|| "DelegateCapsule returned ok:false".into());
            return Err(ServiceError::ao_network_error(format!(
                "kfrag_id={kfrag_id}, capsule_id={capsule_id}: {reason}"
            )));
        }

        Ok(())
    }

    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>> {
        // Single batch query: HyperBEAM cannot restore wasm-64 snapshots
        // across requests, so recovery reads all cFrags from one computed slot.
        let msg = AOQueryMsg::get_cfrags(capsule_id);
        let result = self
            .ao_client
            .query(process_id, msg)
            .await
            .map_err(|e| ServiceError::ao_network_error(format!("GetCFrags query: {e}")))?;

        let value: serde_json::Value = serde_json::from_slice(result.as_slice())
            .map_err(|e| ServiceError::ao_network_error(format!("Failed to parse cFrags: {e}")))?;
        // Contract-level errors arrive as a bare string
        // (`{"Error": ...}` response shape → results/data).
        if let Some(err) = value.as_str() {
            return Err(ServiceError::ao_network_error(format!(
                "GetCFrags failed: {err}"
            )));
        }
        let response: GetCFragsResponse = serde_json::from_value(value)
            .map_err(|e| ServiceError::ao_network_error(format!("Failed to parse cFrags: {e}")))?;

        let prefix = format!("{secret_id}_");
        let mut cfrags: Vec<CFragData> = response
            .cfrags
            .into_iter()
            .filter(|entry| entry.kfrag_id.starts_with(&prefix))
            .map(|entry| CFragData {
                cfrag_data: entry.cfrag,
                holder_id: process_id.to_string(),
            })
            .collect();
        cfrags.truncate(total_shares as usize);

        Ok(cfrags)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{ContractStorage, ContractStorageImpl};
    use crate::adapter::external::mock_ao::MockAOClient;
    use crate::usecase::core::crypto::KeyFragment;
    use std::sync::Arc;

    fn make_kfrag(id: u8, data: Vec<u8>) -> KeyFragment {
        KeyFragment {
            id,
            key_data: data,
            verification_data: vec![],
            precursor: vec![],
        }
    }

    // ─── send_kfrags ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_kfrags_stores_to_contract_id() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        let kfrags = vec![make_kfrag(0, vec![1, 2, 3])];
        cs.send_kfrags(&kfrags, "owner-process", "secret-1")
            .await
            .unwrap();

        let stored = mock.get_stored_kfrags("owner-process");
        assert_eq!(stored.len(), 1, "should store 1 kfrag at contract_id");
        let (kfrag_id, data) = &stored[0];
        assert_eq!(kfrag_id, "secret-1_0");
        // Stored bytes are bincode-serialized StoredKeyFragPayload
        let decoded: super::StoredKeyFragPayload =
            bincode::deserialize(data).expect("should deserialize as StoredKeyFragPayload");
        assert_eq!(decoded.key_data, vec![1u8, 2, 3]);
    }

    #[tokio::test]
    async fn send_kfrags_multiple_stores_all() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        let kfrags = vec![
            make_kfrag(0, vec![10, 20]),
            make_kfrag(1, vec![30, 40]),
            make_kfrag(2, vec![50, 60]),
        ];
        cs.send_kfrags(&kfrags, "owner-process", "secret-x")
            .await
            .unwrap();

        let stored = mock.get_stored_kfrags("owner-process");
        assert_eq!(stored.len(), 3);
    }

    #[tokio::test]
    async fn send_kfrags_ok_false_returns_error() {
        let mock = Arc::new(MockAOClient::new());
        // Inject ok:false for the first execute() call
        mock.inject_ok_false("DelegateKFrag returned ok:false");
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        let kfrags = vec![make_kfrag(0, vec![1, 2, 3])];
        let result = cs.send_kfrags(&kfrags, "owner-process", "secret-1").await;
        assert!(result.is_err(), "ok:false should propagate as ServiceError");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("secret-1_0"),
            "error should include kfrag_id"
        );
    }

    // ─── delegate_capsule: holder_process_id フォールバック ──────────────────

    /// holder_process_id が未設定 (new_single_process) → contract_id にフォールバック
    #[tokio::test]
    async fn delegate_capsule_fallback_uses_contract_id() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        use crate::adapter::external::ao::{AOClient, AOExecuteMsg};
        let (kfrag_bytes, capsule_bytes) = create_test_crypto_data("kfrag-0");

        // Pre-register kfrag with valid Umbral data
        let kfrag_msg = AOExecuteMsg::delegate_kfrag("kfrag-0", &kfrag_bytes, "owner-process");
        mock.execute("owner-process", kfrag_msg).await.unwrap();

        cs.delegate_capsule(&capsule_bytes, "owner-process", "kfrag-0", "cap-001")
            .await
            .unwrap();

        // Capsule should be stored at contract_id = "owner-process"
        let stored = mock.get_stored_capsules("owner-process");
        assert_eq!(
            stored.len(),
            1,
            "capsule should be at contract_id when holder unset"
        );
        let (cap_id, kfrag_id, _data) = &stored[0];
        assert_eq!(cap_id, "cap-001");
        assert_eq!(kfrag_id, "kfrag-0");
    }

    /// holder_process_id が設定済み → そのプロセスIDに送られる
    #[tokio::test]
    async fn delegate_capsule_uses_holder_when_set() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new(Arc::clone(&mock), "holder-process");

        use crate::adapter::external::ao::{AOClient, AOExecuteMsg};
        let (kfrag_bytes, capsule_bytes) = create_test_crypto_data("kfrag-0");

        // Pre-register kfrag with valid Umbral data
        let kfrag_msg = AOExecuteMsg::delegate_kfrag("kfrag-0", &kfrag_bytes, "holder-process");
        mock.execute("owner-process", kfrag_msg).await.unwrap();

        cs.delegate_capsule(&capsule_bytes, "owner-process", "kfrag-0", "cap-001")
            .await
            .unwrap();

        // Mock routes capsule to holder_process_id from the message data
        assert!(
            mock.get_stored_capsules("owner-process").is_empty(),
            "capsule should NOT be at owner-process when holder_process_id is set"
        );
        let stored_holder = mock.get_stored_capsules("holder-process");
        assert_eq!(stored_holder.len(), 1);
        let (cap_id, _, _) = &stored_holder[0];
        assert_eq!(cap_id, "cap-001");
    }

    #[tokio::test]
    async fn delegate_capsule_ok_false_returns_error() {
        let mock = Arc::new(MockAOClient::new());
        mock.inject_ok_false("DelegateCapsule failed");
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        let result = cs
            .delegate_capsule(b"capsule-data", "owner-process", "kfrag-0", "cap-001")
            .await;
        assert!(result.is_err());
    }

    // ─── retrieve_cfrags ─────────────────────────────────────────────────────

    /// Generate valid Umbral kFrag + Capsule test data for a given kfrag_id.
    fn create_test_crypto_data(kfrag_id: &str) -> (Vec<u8>, Vec<u8>) {
        use serde::Serialize as TestSerialize;
        use umbral_pre::{DefaultSerialize, SecretKey};

        #[derive(TestSerialize)]
        struct Payload {
            id: String,
            key_data: Vec<u8>,
            verification_data: Vec<u8>,
            precursor: Vec<u8>,
        }
        #[derive(TestSerialize)]
        struct VD {
            verifying_pk: Vec<u8>,
            delegating_pk: Vec<u8>,
            receiving_pk: Vec<u8>,
        }

        let delegating_sk = SecretKey::random();
        let delegating_pk = delegating_sk.public_key();
        let receiving_sk = SecretKey::random();
        let receiving_pk = receiving_sk.public_key();
        let signing_sk = SecretKey::random();
        let verifying_pk = signing_sk.public_key();
        let signer = umbral_pre::Signer::new(signing_sk);

        let (capsule, _) = umbral_pre::encrypt(&delegating_pk, b"test").unwrap();
        let capsule_bytes = capsule.to_bytes().unwrap().to_vec();

        let verified_kfrags =
            umbral_pre::generate_kfrags(&delegating_sk, &receiving_pk, &signer, 1, 1, true, true);
        let kfrag = verified_kfrags[0].clone().unverify();
        let kfrag_data = kfrag.to_bytes().unwrap().to_vec();

        let vd = VD {
            verifying_pk: bincode::serialize(&verifying_pk).unwrap(),
            delegating_pk: bincode::serialize(&delegating_pk).unwrap(),
            receiving_pk: bincode::serialize(&receiving_pk).unwrap(),
        };

        let payload = Payload {
            id: kfrag_id.to_string(),
            key_data: kfrag_data,
            verification_data: bincode::serialize(&vd).unwrap(),
            precursor: vec![],
        };
        (bincode::serialize(&payload).unwrap(), capsule_bytes)
    }

    #[tokio::test]
    async fn retrieve_cfrags_returns_ready_cfrags() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        use crate::adapter::external::ao::{AOClient, AOExecuteMsg};
        let (kfrag_bytes, capsule_bytes) = create_test_crypto_data("secret-1_0");

        // DelegateKFrag → stores kfrag at holder-process
        let msg = AOExecuteMsg::delegate_kfrag("secret-1_0", &kfrag_bytes, "holder-process");
        mock.execute("holder-process", msg).await.unwrap();

        // DelegateCapsule → triggers real re-encryption → cFrag stored
        let msg = AOExecuteMsg::delegate_capsule(
            "secret-1_0",
            "cap-001",
            &capsule_bytes,
            "holder-process",
        );
        mock.execute("holder-process", msg).await.unwrap();

        let cfrags = cs
            .retrieve_cfrags("secret-1", 1, "cap-001", "holder-process")
            .await
            .unwrap();
        assert_eq!(cfrags.len(), 1);
        assert_eq!(cfrags[0].holder_id, "holder-process");
    }

    #[tokio::test]
    async fn retrieve_cfrags_skips_missing() {
        let mock = Arc::new(MockAOClient::new());
        let cs = ContractStorageImpl::new_single_process(Arc::clone(&mock));

        use crate::adapter::external::ao::{AOClient, AOExecuteMsg};
        // Only create data for kfrag index 1 (out of 3)
        let (kfrag_bytes, capsule_bytes) = create_test_crypto_data("secret-1_1");

        let msg = AOExecuteMsg::delegate_kfrag("secret-1_1", &kfrag_bytes, "holder-process");
        mock.execute("holder-process", msg).await.unwrap();

        let msg = AOExecuteMsg::delegate_capsule(
            "secret-1_1",
            "cap-001",
            &capsule_bytes,
            "holder-process",
        );
        mock.execute("holder-process", msg).await.unwrap();

        let cfrags = cs
            .retrieve_cfrags("secret-1", 3, "cap-001", "holder-process")
            .await
            .unwrap();
        // Only the one that's ready comes back
        assert_eq!(cfrags.len(), 1);
    }

    // ─── action名契約テスト (contract test) ──────────────────────────────────

    /// コントラクト側の action 名とクライアント側が一致していることを型レベルで保証する。
    /// このテストが通れば、action 名ズレによる silent failure を防止できる。
    #[test]
    fn list_capsules_action_name_matches_contract() {
        use crate::adapter::external::ao::AOQueryMsg;
        let msg = AOQueryMsg::list_capsules_by_kfrag("kfrag-0", None, None);
        assert_eq!(
            msg.action(),
            "ListCapsules",
            "action name must match ao/contracts/src/handlers.rs dispatch table"
        );
    }

    #[test]
    fn delegate_kfrag_action_name_matches_contract() {
        use crate::adapter::external::ao::AOExecuteMsg;
        let msg = AOExecuteMsg::delegate_kfrag("kfrag-0", &[1, 2, 3], "holder-0");
        assert_eq!(msg.action(), "DelegateKFrag");
    }

    #[test]
    fn delegate_capsule_action_name_matches_contract() {
        use crate::adapter::external::ao::AOExecuteMsg;
        let msg = AOExecuteMsg::delegate_capsule("kfrag-0", "cap-001", &[1, 2], "holder");
        assert_eq!(msg.action(), "DelegateCapsule");
    }

    #[test]
    fn reencrypt_action_name_matches_contract() {
        use crate::adapter::external::ao::AOExecuteMsg;
        let msg = AOExecuteMsg::reencrypt("kfrag-0", "cap-001");
        assert_eq!(msg.action(), "Reencrypt");
    }

    #[test]
    fn get_cfrag_action_name_matches_contract() {
        use crate::adapter::external::ao::AOQueryMsg;
        let msg = AOQueryMsg::get_cfrag("kfrag-0", "cap-001");
        assert_eq!(msg.action(), "GetCFrag");
    }

    #[test]
    fn init_action_name_matches_contract() {
        use crate::adapter::external::ao::AOExecuteMsg;
        let msg = AOExecuteMsg::init("Owner");
        assert_eq!(msg.action(), "Init");
    }

    #[test]
    fn delegate_kfrag_includes_holder_process_id() {
        use crate::adapter::external::ao::AOExecuteMsg;
        let msg = AOExecuteMsg::delegate_kfrag("kfrag-0", &[1, 2, 3], "holder-0");
        let data = msg.data();
        assert_eq!(data["holder_process_id"].as_str(), Some("holder-0"));
    }
}
