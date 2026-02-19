use formix::adapter::external::ao::{
    ArweaveJWK, Binary, DataItemBuilder, DataItemSigner, ExecuteMsg, QueryMsg,
};

#[allow(clippy::many_single_char_names)]
fn test_jwk() -> ArweaveJWK {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rand::rngs::OsRng;
    use rsa::RsaPrivateKey;
    use rsa::traits::{PrivateKeyParts, PublicKeyParts};

    let private_key = RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
    let public_key = private_key.to_public_key();
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
    let d = URL_SAFE_NO_PAD.encode(private_key.d().to_bytes_be());
    let primes = private_key.primes();
    let p = URL_SAFE_NO_PAD.encode(primes[0].to_bytes_be());
    let q = URL_SAFE_NO_PAD.encode(primes[1].to_bytes_be());

    ArweaveJWK {
        kty: "RSA".to_string(),
        n,
        e,
        d,
        p,
        q,
        dp: String::new(),
        dq: String::new(),
        qi: String::new(),
    }
}

#[test]
fn test_execute_msg_to_data_item() {
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };
    let item = DataItemBuilder::build_execute("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
        .unwrap();
    assert_eq!(item.target.len(), 32);
    assert!(!item.data.is_empty());
}

#[test]
fn test_data_item_ao_tags() {
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf1".to_string(),
        kfrag: Binary::new(vec![1]),
    };
    let item = DataItemBuilder::build_execute("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
        .unwrap();
    let tag_names: Vec<&str> = item.tags.iter().map(|t| t.name.as_str()).collect();
    assert!(tag_names.contains(&"Data-Protocol"));
    assert!(tag_names.contains(&"Variant"));
    assert!(tag_names.contains(&"Type"));
    assert!(tag_names.contains(&"SDK"));
    assert!(tag_names.contains(&"Action"));
    assert!(tag_names.contains(&"Target"));

    let dp = item
        .tags
        .iter()
        .find(|t| t.name == "Data-Protocol")
        .unwrap();
    assert_eq!(dp.value, "ao");
}

#[test]
fn test_delegate_kfrag_data_item() {
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };
    let item = DataItemBuilder::build_execute("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
        .unwrap();
    let action_tag = item.tags.iter().find(|t| t.name == "Action").unwrap();
    assert_eq!(action_tag.value, "DelegateKFrag");
}

#[test]
fn test_get_cfrag_data_item() {
    let msg = QueryMsg::GetCFrag {
        kfrag_id: "kf1".to_string(),
        capsule_id: "cap1".to_string(),
    };
    let body =
        DataItemBuilder::build_query_body("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
            .unwrap();
    let tags = body["Tags"].as_array().unwrap();
    let action = tags.iter().find(|t| t["name"] == "Action").unwrap();
    assert_eq!(action["value"], "GetCFrag");
}

#[test]
fn test_data_item_target_tag() {
    let msg = ExecuteMsg::Reencrypt {
        kfrag_id: "kf1".to_string(),
        capsule_id: "cap1".to_string(),
    };
    let item = DataItemBuilder::build_execute("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE", &msg)
        .unwrap();
    let target_tag = item.tags.iter().find(|t| t.name == "Target").unwrap();
    assert_eq!(
        target_tag.value,
        "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
    );
}

#[test]
fn test_dry_run_body_structure() {
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf1".to_string(),
        kfrag: Binary::new(vec![1]),
    };
    let body =
        DataItemBuilder::build_dry_run_body("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
            .unwrap();
    assert_eq!(
        body["Target"],
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    assert!(body["Tags"].is_array());
    assert!(body["Data"].is_string());
}

#[test]
fn test_signer_new_valid_jwk() {
    let jwk = test_jwk();
    let signer = DataItemSigner::new(&jwk);
    assert!(signer.is_ok());
}

#[test]
fn test_signer_invalid_kty() {
    let mut jwk = test_jwk();
    jwk.kty = "EC".to_string();
    let result = DataItemSigner::new(&jwk);
    assert!(result.is_err());
}

#[test]
fn test_signer_missing_fields() {
    let mut jwk = test_jwk();
    jwk.d = String::new();
    let result = DataItemSigner::new(&jwk);
    assert!(result.is_err());
}

#[test]
fn test_sign_data_item() {
    let jwk = test_jwk();
    let signer = DataItemSigner::new(&jwk).unwrap();
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };
    let item = DataItemBuilder::build_execute("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
        .unwrap();
    let signed = signer.sign(&item);
    assert!(signed.is_ok());
    let bytes = signed.unwrap();
    assert_eq!(bytes[0], 1);
    assert_eq!(bytes[1], 0);
    assert!(bytes.len() > 1026);
}

#[test]
fn test_owner_address() {
    let jwk = test_jwk();
    let signer = DataItemSigner::new(&jwk).unwrap();
    let addr = signer.owner_address();
    assert!(!addr.is_empty());
    assert_eq!(addr.len(), 43);
}

#[test]
fn test_all_execute_msg_actions() {
    let cases: Vec<(ExecuteMsg, &str)> = vec![
        (
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "k".to_string(),
                kfrag: Binary::new(vec![1]),
            },
            "DelegateKFrag",
        ),
        (
            ExecuteMsg::DelegateCapsule {
                kfrag_id: "k".to_string(),
                capsule_id: "c".to_string(),
                capsule: Binary::new(vec![1]),
            },
            "DelegateCapsule",
        ),
        (
            ExecuteMsg::SubmitKFrag {
                kfrag_id: "k".to_string(),
                kfrag: Binary::new(vec![1]),
            },
            "SubmitKFrag",
        ),
        (
            ExecuteMsg::SubmitCapsule {
                kfrag_id: "k".to_string(),
                capsule_id: "c".to_string(),
                capsule: Binary::new(vec![1]),
            },
            "SubmitCapsule",
        ),
        (
            ExecuteMsg::Reencrypt {
                kfrag_id: "k".to_string(),
                capsule_id: "c".to_string(),
            },
            "Reencrypt",
        ),
    ];

    for (msg, expected_action) in cases {
        let item =
            DataItemBuilder::build_execute("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &msg)
                .unwrap();
        let action_tag = item.tags.iter().find(|t| t.name == "Action").unwrap();
        assert_eq!(action_tag.value, expected_action);
    }
}
