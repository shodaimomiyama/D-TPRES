use formix::domain::{CapsuleId, KFragId, Secret, SecretState, ShareCollectionId};

#[test]
fn test_secret_new_valid() {
    let pk = vec![1u8; 33];
    let secret = Secret::new(2, 3, pk).unwrap();

    assert_eq!(secret.threshold_k(), 2);
    assert_eq!(secret.threshold_n(), 3);
    assert_eq!(secret.state(), SecretState::Initialized);
    assert!(secret.capsule_id().is_none());
    assert!(secret.share_collection_id().is_none());
}

#[test]
fn test_secret_new_invalid_threshold_zero() {
    let pk = vec![1u8; 33];
    let result = Secret::new(0, 3, pk);
    assert!(result.is_err());
}

#[test]
fn test_secret_new_invalid_threshold_k_greater_than_n() {
    let pk = vec![1u8; 33];
    let result = Secret::new(5, 3, pk);
    assert!(result.is_err());
}

#[test]
fn test_secret_new_empty_public_key() {
    let result = Secret::new(2, 3, vec![]);
    assert!(result.is_err());
}

#[test]
fn test_secret_split_transition() {
    let pk = vec![1u8; 33];
    let mut secret = Secret::new(2, 3, pk).unwrap();

    let collection_id = ShareCollectionId::generate();
    let capsule_id = CapsuleId::generate();

    secret
        .split(collection_id.clone(), capsule_id.clone())
        .unwrap();

    assert_eq!(secret.state(), SecretState::Split);
    assert_eq!(secret.share_collection_id(), Some(&collection_id));
    assert_eq!(secret.capsule_id(), Some(&capsule_id));
}

#[test]
fn test_secret_invalid_split_from_wrong_state() {
    let pk = vec![1u8; 33];
    let mut secret = Secret::new(2, 3, pk).unwrap();

    secret
        .split(ShareCollectionId::generate(), CapsuleId::generate())
        .unwrap();

    let result = secret.split(ShareCollectionId::generate(), CapsuleId::generate());
    assert!(result.is_err());
}

#[test]
fn test_secret_distribute_transition() {
    let pk = vec![1u8; 33];
    let mut secret = Secret::new(2, 3, pk).unwrap();

    secret
        .split(ShareCollectionId::generate(), CapsuleId::generate())
        .unwrap();

    let kfrag_ids = vec![
        KFragId::generate(),
        KFragId::generate(),
        KFragId::generate(),
    ];

    secret.distribute(kfrag_ids.clone()).unwrap();

    assert_eq!(secret.state(), SecretState::Distributed);
    assert_eq!(secret.kfrag_ids().len(), 3);
}

#[test]
fn test_secret_distribute_wrong_kfrag_count() {
    let pk = vec![1u8; 33];
    let mut secret = Secret::new(2, 3, pk).unwrap();

    secret
        .split(ShareCollectionId::generate(), CapsuleId::generate())
        .unwrap();

    let kfrag_ids = vec![KFragId::generate(), KFragId::generate()];
    let result = secret.distribute(kfrag_ids);
    assert!(result.is_err());
}
