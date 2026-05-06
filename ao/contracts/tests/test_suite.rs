use contract::{
    CONFIG, CapsuleStatus, ContractError, DEFAULT_HOLDER_PROCESS_ID, ExecuteMsg, HOLDER_CFRAGS,
    IDEM_FLAGS, INDEX_KFRAG_TO_CAPS, InstantiateMsg, KFRAG_HOLDERS, ListCapsulesByKFragResponse,
    OWNER_CAPSULES, OwnerCapsuleData, OwnerKFragData, QueryMsg, REPLY_DELEGATE_CAPSULE,
    REPLY_DELEGATE_KFRAG, ValidateMessage, execute as contract_execute,
    instantiate as contract_instantiate, query as contract_query, validate_id,
};
use cosmwasm_std::testing::{
    MockApi, MockQuerier, MockStorage, mock_dependencies, mock_env, mock_info,
};
use cosmwasm_std::{Binary, CosmosMsg, Empty, OwnedDeps, WasmMsg, from_json};
use serde::{Deserialize, Serialize};
use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};

type TestDeps = OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>;

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

fn log_test_value<T: std::fmt::Debug>(test: &str, label: &str, value: &T) {
    println!("[{}] {} = {:?}", test, label, value);
}

fn instantiate_process(deps: &mut TestDeps, process_id: &str) {
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        process_id: process_id.to_string(),
    };
    contract_instantiate(deps.as_mut(), env, info, msg).unwrap();
}

struct CryptoFixture {
    delegating_pk: umbral_pre::PublicKey,
    receiving_sk: umbral_pre::SecretKey,
    receiving_pk: umbral_pre::PublicKey,
    verifying_pk: umbral_pre::PublicKey,
    serialized_kfrag: Binary,
    verification_bytes: Vec<u8>,
}

fn build_crypto_fixture() -> CryptoFixture {
    let delegating_sk = umbral_pre::SecretKey::random();
    let delegating_pk = delegating_sk.public_key();
    let receiving_sk = umbral_pre::SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let signing_sk = umbral_pre::SecretKey::random();
    let verifying_pk = signing_sk.public_key();
    let signer = umbral_pre::Signer::new(signing_sk);

    let verified_kfrags =
        umbral_pre::generate_kfrags(&delegating_sk, &receiving_pk, &signer, 1, 1, true, true);
    let kfrag = verified_kfrags.first().unwrap().clone().unverify();
    let kfrag_bytes = kfrag.to_bytes().unwrap().to_vec();

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
    let serialized_kfrag = Binary::from(bincode::serialize(&stored_kfrag).unwrap());

    CryptoFixture {
        delegating_pk,
        receiving_sk,
        receiving_pk,
        verifying_pk,
        serialized_kfrag,
        verification_bytes,
    }
}

fn encrypt_capsule(
    pk: &umbral_pre::PublicKey,
    plaintext: &[u8],
) -> (Binary, Vec<u8>, umbral_pre::Capsule) {
    let (capsule, ciphertext_box) = umbral_pre::encrypt(pk, plaintext).unwrap();
    (
        Binary::from(bincode::serialize(&capsule).unwrap()),
        ciphertext_box.to_vec(),
        capsule,
    )
}

#[test]
fn instantiate_stores_config() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.process_id, "test_process");
    log_test_value("instantiate_stores_config", "config", &config);
}

#[test]
fn instantiate_validation_error() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        process_id: "".into(),
    };
    let err = contract_instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::ValidationError { .. }));
    log_test_value("instantiate_validation_error", "error", &err);
}

#[test]
fn submit_kfrag_is_idempotent() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");

    let env = mock_env();
    let info = mock_info("owner", &[]);
    let exec = ExecuteMsg::SubmitKFrag {
        kfrag_id: "k1".into(),
        kfrag: Binary::from(b"kfrag".as_ref()),
    };
    contract_execute(deps.as_mut(), env.clone(), info.clone(), exec.clone()).unwrap();
    let second = contract_execute(deps.as_mut(), env, info, exec).unwrap();
    log_test_value("submit_kfrag_is_idempotent", "attrs", &second.attributes);
}

#[test]
fn submit_capsule_flow_produces_cfrag_and_indexes() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let fixtures = build_crypto_fixture();

    let env = mock_env();
    let info = mock_info("owner", &[]);
    contract_execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::SubmitKFrag {
            kfrag_id: "kfrag1".into(),
            kfrag: fixtures.serialized_kfrag.clone(),
        },
    )
    .unwrap();

    let plaintext = b"secret payload".to_vec();
    let (capsule_bytes, ciphertext, capsule_struct) =
        encrypt_capsule(&fixtures.delegating_pk, &plaintext);

    contract_execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::SubmitCapsule {
            kfrag_id: "kfrag1".into(),
            capsule_id: "capsule1".into(),
            capsule: capsule_bytes,
        },
    )
    .unwrap();

    let stored = HOLDER_CFRAGS
        .load(
            &deps.storage,
            ("test_process".into(), "kfrag1".into(), "capsule1".into()),
        )
        .unwrap();
    let decoded: StoredCFrag = bincode::deserialize(stored.cfrag.as_slice()).unwrap();
    assert_eq!(decoded.proof, fixtures.verification_bytes);

    let capsule_frag = umbral_pre::CapsuleFrag::from_bytes(&decoded.capsule_fragment).unwrap();
    let verified = capsule_frag
        .verify(
            &capsule_struct,
            &fixtures.verifying_pk,
            &fixtures.delegating_pk,
            &fixtures.receiving_pk,
        )
        .unwrap();
    let recovered = umbral_pre::decrypt_reencrypted(
        &fixtures.receiving_sk,
        &fixtures.delegating_pk,
        &capsule_struct,
        vec![verified],
        ciphertext.as_slice(),
    )
    .unwrap();
    assert_eq!(recovered.to_vec(), plaintext);

    assert!(IDEM_FLAGS.has(
        &deps.storage,
        ("test_process".into(), "kfrag1".into(), "capsule1".into()),
    ));
    assert!(INDEX_KFRAG_TO_CAPS.has(
        &deps.storage,
        ("test_process".into(), "kfrag1".into(), "capsule1".into()),
    ));

    log_test_value(
        "submit_capsule_flow_produces_cfrag_and_indexes",
        "cfrag",
        &decoded,
    );
}

#[test]
fn delegate_kfrag_creates_submsg() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");

    let env = mock_env();
    let info = mock_info("owner", &[]);
    let response = contract_execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::DelegateKFrag {
            kfrag_id: "kfrag1".into(),
            kfrag: Binary::from(b"kfrag_data".as_ref()),
        },
    )
    .unwrap();

    assert_eq!(response.messages.len(), 1);
    let sub_msg = &response.messages[0];
    assert_eq!(sub_msg.id, REPLY_DELEGATE_KFRAG);
    match &sub_msg.msg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!(contract_addr, DEFAULT_HOLDER_PROCESS_ID);
            let decoded: ExecuteMsg = from_json(msg).unwrap();
            if let ExecuteMsg::SubmitKFrag { kfrag_id, .. } = decoded {
                assert_eq!(kfrag_id, "kfrag1");
            } else {
                panic!("expected SubmitKFrag message");
            }
        }
        _ => panic!("expected WasmMsg::Execute"),
    }

    let holder = KFRAG_HOLDERS
        .load(&deps.storage, ("test_process".into(), "kfrag1".into()))
        .unwrap();
    assert_eq!(holder, DEFAULT_HOLDER_PROCESS_ID);
}

#[test]
fn delegate_capsule_creates_submsg() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let fixtures = build_crypto_fixture();

    let env = mock_env();
    let info = mock_info("owner", &[]);
    contract_execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::DelegateKFrag {
            kfrag_id: "kfrag1".into(),
            kfrag: fixtures.serialized_kfrag.clone(),
        },
    )
    .unwrap();

    let (capsule_bytes, _, _) = encrypt_capsule(&fixtures.delegating_pk, b"secret payload");
    let response = contract_execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::DelegateCapsule {
            kfrag_id: "kfrag1".into(),
            capsule_id: "capsule1".into(),
            capsule: capsule_bytes,
        },
    )
    .unwrap();

    assert_eq!(response.messages.len(), 1);
    let sub_msg = &response.messages[0];
    assert_eq!(sub_msg.id, REPLY_DELEGATE_CAPSULE);
    match &sub_msg.msg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr, msg, ..
        }) => {
            assert_eq!(contract_addr, DEFAULT_HOLDER_PROCESS_ID);
            let decoded: ExecuteMsg = from_json(msg).unwrap();
            if let ExecuteMsg::SubmitCapsule { capsule_id, .. } = decoded {
                assert_eq!(capsule_id, "capsule1");
            } else {
                panic!("expected SubmitCapsule message");
            }
        }
        _ => panic!("expected WasmMsg::Execute"),
    }
}

#[test]
fn delegate_capsule_requires_mapping() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");

    let env = mock_env();
    let info = mock_info("owner", &[]);
    let err = contract_execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::DelegateCapsule {
            kfrag_id: "missing".into(),
            capsule_id: "caps".into(),
            capsule: Binary::from(b"caps".as_ref()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::KFragNotFound { .. }));
    log_test_value("delegate_capsule_requires_mapping", "error", &err);
}

#[test]
fn reencrypt_after_manual_fix_succeeds() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let fixtures = build_crypto_fixture();

    let env = mock_env();
    let info = mock_info("owner", &[]);
    contract_execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::SubmitKFrag {
            kfrag_id: "kfrag1".into(),
            kfrag: fixtures.serialized_kfrag.clone(),
        },
    )
    .unwrap();

    let err = contract_execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::SubmitCapsule {
            kfrag_id: "kfrag1".into(),
            capsule_id: "capsule1".into(),
            capsule: Binary::from(b"corrupted".as_ref()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ReencryptionFailed { .. }));

    let (capsule_bytes, _, _) = encrypt_capsule(&fixtures.delegating_pk, b"fixed");
    let mut replacement = OwnerCapsuleData::new(capsule_bytes, "2025-01-01T00:00:00Z".into());
    replacement.update_status(CapsuleStatus::Error, "2025-01-01T00:00:00Z".into());
    OWNER_CAPSULES
        .save(
            &mut deps.storage,
            ("test_process".into(), "kfrag1".into(), "capsule1".into()),
            &replacement,
        )
        .unwrap();

    let resp = contract_execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Reencrypt {
            kfrag_id: "kfrag1".into(),
            capsule_id: "capsule1".into(),
        },
    )
    .unwrap();
    assert!(IDEM_FLAGS.has(
        &deps.storage,
        ("test_process".into(), "kfrag1".into(), "capsule1".into()),
    ));
    log_test_value(
        "reencrypt_after_manual_fix_succeeds",
        "attrs",
        &resp.attributes,
    );
}

#[test]
fn query_get_cfrag_not_ready() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let env = mock_env();
    let q = QueryMsg::GetCFrag {
        kfrag_id: "missing".into(),
        capsule_id: "caps".into(),
    };
    let err = contract_query(deps.as_ref(), env, q).unwrap_err();
    assert!(err.to_string().contains("ERR_CFRAG_NOT_READY"));
}

#[test]
fn query_list_capsules_empty() {
    let mut deps = mock_dependencies();
    instantiate_process(&mut deps, "test_process");
    let env = mock_env();
    let q = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "k1".into(),
        start_after: None,
        limit: None,
    };
    let resp = contract_query(deps.as_ref(), env, q).unwrap();
    let parsed: ListCapsulesByKFragResponse = from_json(&resp).unwrap();
    assert!(parsed.capsules.is_empty());
}

#[test]
fn message_validation_behaves() {
    let instantiate = InstantiateMsg {
        process_id: "pid".into(),
    };
    assert!(instantiate.validate().is_ok());

    let exec = ExecuteMsg::SubmitKFrag {
        kfrag_id: "valid".into(),
        kfrag: Binary::from(b"bytes".as_ref()),
    };
    assert!(exec.validate().is_ok());

    let bad = ExecuteMsg::SubmitKFrag {
        kfrag_id: "bad@id".into(),
        kfrag: Binary::from(b"bytes".as_ref()),
    };
    assert!(bad.validate().is_err());

    let query = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "valid".into(),
        start_after: None,
        limit: Some(10),
    };
    assert!(query.validate().is_ok());
}

#[test]
fn state_helpers_work() {
    assert!(validate_id("good", 128).is_ok());
    assert!(validate_id("", 128).is_err());

    let blob = OwnerKFragData::new(
        Binary::from(b"data".as_ref()),
        "2025-01-01T00:00:00Z".into(),
    );
    assert_eq!(blob.meta.size_bytes, 4);
    log_test_value("state_helpers_work", "blob", &blob);
}
