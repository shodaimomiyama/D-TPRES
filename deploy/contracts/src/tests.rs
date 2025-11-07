#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::handlers::ContractError;
    use crate::msg::{
        ExecuteMsg, GetCFragResponse, InstantiateMsg, ListCapsulesByKFragResponse, QueryMsg,
    };
    use crate::state::{
        CapsuleStatus, CONFIG, HOLDER_CFRAGS, IDEM_FLAGS, INDEX_KFRAG_TO_CAPS, OWNER_CAPSULES,
        OWNER_KFRAGS,
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Binary};
    use serde::{Deserialize, Serialize};
    use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct StoredKeyFrag {
        id: u8,
        key_data: Vec<u8>,
        verification_data: Vec<u8>,
        #[serde(default)]
        precursor: Vec<u8>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct VerificationData {
        verifying_pk: Vec<u8>,
        delegating_pk: Vec<u8>,
        receiving_pk: Vec<u8>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct StoredCFrag {
        fragment_id: u8,
        capsule_fragment: Vec<u8>,
        proof: Vec<u8>,
    }

    #[test]
    fn instantiate_stores_config() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };

        let response =
            instantiate(deps.as_mut(), env.clone(), info, msg).expect("instantiate succeeds");
        assert!(response
            .attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "instantiate"));

        let config = CONFIG.load(&deps.storage).expect("config stored");
        assert_eq!(config.process_id, "test_process");
    }

    #[test]
    fn submit_kfrag_is_idempotent() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        let exec_env = mock_env();
        let exec_info = mock_info("sender", &[]);
        let msg = ExecuteMsg::SubmitKFrag {
            kfrag_id: "kfrag_id".to_string(),
            kfrag: Binary::from(b"kfrag-bytes".as_ref()),
        };

        let response = execute(
            deps.as_mut(),
            exec_env.clone(),
            exec_info.clone(),
            msg.clone(),
        )
        .expect("first submit succeeds");
        assert!(response
            .attributes
            .iter()
            .any(|attr| attr.key == "status" && attr.value == "success"));

        let stored = OWNER_KFRAGS
            .load(
                &deps.storage,
                ("test_process".to_string(), "kfrag_id".to_string()),
            )
            .expect("stored kfrag");
        assert_eq!(stored.kfrag, Binary::from(b"kfrag-bytes".as_ref()));

        let second = execute(deps.as_mut(), exec_env, exec_info, msg).expect("duplicate succeeds");
        assert!(second
            .attributes
            .iter()
            .any(|attr| attr.key == "status" && attr.value == "no_op"));
    }

    #[test]
    fn submit_capsule_without_kfrag_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        let exec_env = mock_env();
        let exec_info = mock_info("sender", &[]);
        let msg = ExecuteMsg::SubmitCapsule {
            kfrag_id: "missing".to_string(),
            capsule_id: "capsule".to_string(),
            capsule: Binary::from(b"capsule".as_ref()),
        };

        let result = execute(deps.as_mut(), exec_env, exec_info, msg);
        assert!(matches!(
            result,
            Err(ContractError::KFragNotFound { ref kfrag_id }) if kfrag_id == "missing"
        ));
    }

    #[test]
    fn submit_capsule_flow_produces_cfrag_and_indexes() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        let submit_env = mock_env();
        let submit_info = mock_info("sender", &[]);

        let delegating_sk = umbral_pre::SecretKey::random();
        let delegating_pk = delegating_sk.public_key();
        let receiving_sk = umbral_pre::SecretKey::random();
        let receiving_pk = receiving_sk.public_key();
        let signing_sk = umbral_pre::SecretKey::random();
        let verifying_pk = signing_sk.public_key();
        let signer = umbral_pre::Signer::new(signing_sk);

        let verified_kfrags =
            umbral_pre::generate_kfrags(&delegating_sk, &receiving_pk, &signer, 1, 1, true, true);
        let kfrag = verified_kfrags
            .first()
            .expect("kfrag exists")
            .clone()
            .unverify();
        let kfrag_bytes = kfrag.to_bytes().expect("serialize kfrag").to_vec();

        let verification = VerificationData {
            verifying_pk: bincode::serialize(&verifying_pk).unwrap(),
            delegating_pk: bincode::serialize(&delegating_pk).unwrap(),
            receiving_pk: bincode::serialize(&receiving_pk).unwrap(),
        };
        let verification_bytes = bincode::serialize(&verification).unwrap();
        let stored_kfrag = StoredKeyFrag {
            id: 0,
            key_data: kfrag_bytes,
            verification_data: verification_bytes.clone(),
            precursor: vec![],
        };
        let serialized_kfrag = bincode::serialize(&stored_kfrag).unwrap();

        let submit_kfrag = ExecuteMsg::SubmitKFrag {
            kfrag_id: "kfrag1".to_string(),
            kfrag: Binary::from(serialized_kfrag),
        };
        execute(
            deps.as_mut(),
            submit_env.clone(),
            submit_info.clone(),
            submit_kfrag,
        )
        .expect("kfrag submission succeeds");

        let plaintext = b"secret payload".to_vec();
        let (capsule_struct, ciphertext_box) =
            umbral_pre::encrypt(&delegating_pk, &plaintext).expect("encrypt");
        let ciphertext = ciphertext_box.to_vec();
        let capsule_bytes = bincode::serialize(&capsule_struct).unwrap();

        let submit_capsule = ExecuteMsg::SubmitCapsule {
            kfrag_id: "kfrag1".to_string(),
            capsule_id: "capsule1".to_string(),
            capsule: Binary::from(capsule_bytes.clone()),
        };
        let response = execute(
            deps.as_mut(),
            submit_env.clone(),
            submit_info.clone(),
            submit_capsule,
        )
        .expect("capsule submit");
        assert!(response
            .attributes
            .iter()
            .any(|attr| attr.key == "status" && attr.value == "success"));

        let cfrag_entry = HOLDER_CFRAGS
            .load(
                &deps.storage,
                (
                    "test_process".to_string(),
                    "kfrag1".to_string(),
                    "capsule1".to_string(),
                ),
            )
            .expect("cfrag stored");
        let stored_cfrag: StoredCFrag =
            bincode::deserialize(cfrag_entry.cfrag.as_slice()).expect("decode cfrag");
        assert_eq!(stored_cfrag.fragment_id, 0);
        assert_eq!(stored_cfrag.proof, verification_bytes);

        let capsule_frag = umbral_pre::CapsuleFrag::from_bytes(&stored_cfrag.capsule_fragment)
            .expect("frag bytes");
        let verified_cfrag = capsule_frag
            .verify(
                &capsule_struct,
                &verifying_pk,
                &delegating_pk,
                &receiving_pk,
            )
            .expect("verify cfrag");

        let recovered = umbral_pre::decrypt_reencrypted(
            &receiving_sk,
            &delegating_pk,
            &capsule_struct,
            vec![verified_cfrag],
            ciphertext.as_slice(),
        )
        .expect("decrypt");
        assert_eq!(recovered.to_vec(), plaintext);

        let capsule_state = OWNER_CAPSULES
            .load(
                &deps.storage,
                (
                    "test_process".to_string(),
                    "kfrag1".to_string(),
                    "capsule1".to_string(),
                ),
            )
            .expect("capsule state");
        assert!(matches!(capsule_state.status, CapsuleStatus::CFragReady));

        let index_state = INDEX_KFRAG_TO_CAPS
            .load(
                &deps.storage,
                (
                    "test_process".to_string(),
                    "kfrag1".to_string(),
                    "capsule1".to_string(),
                ),
            )
            .expect("index state");
        assert!(matches!(index_state.status, CapsuleStatus::CFragReady));

        assert!(IDEM_FLAGS.has(
            &deps.storage,
            (
                "test_process".to_string(),
                "kfrag1".to_string(),
                "capsule1".to_string(),
            ),
        ));

        let query_msg = QueryMsg::GetCFrag {
            kfrag_id: "kfrag1".to_string(),
            capsule_id: "capsule1".to_string(),
        };
        let binary = query(deps.as_ref(), submit_env.clone(), query_msg).expect("query");
        let response: GetCFragResponse = from_json(&binary).expect("cfrag response");
        assert_eq!(response.meta.sha256_hex.len(), 64);
        assert!(!response.cfrag.is_empty());

        let list_msg = QueryMsg::ListCapsulesByKFrag {
            kfrag_id: "kfrag1".to_string(),
            start_after: None,
            limit: None,
        };
        let list_binary = query(deps.as_ref(), submit_env, list_msg).expect("list");
        let list: ListCapsulesByKFragResponse = from_json(&list_binary).expect("list decode");
        assert_eq!(list.capsules.len(), 1);
        assert_eq!(list.capsules[0].capsule_id, "capsule1");
        assert!(list.next_start_after.is_none());
    }
}
