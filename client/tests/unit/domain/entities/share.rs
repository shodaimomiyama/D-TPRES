use formix::domain::{EncryptedShareData, SecretId, ShareCollection};

fn create_test_shares(n: u8) -> Vec<EncryptedShareData> {
    (1..=n)
        .map(|i| EncryptedShareData::new(i, vec![i; 32]))
        .collect()
}

#[test]
fn test_share_collection_new_valid() {
    let secret_id = SecretId::generate();
    let shares = create_test_shares(3);

    let collection = ShareCollection::new(secret_id.clone(), 2, 3, shares).unwrap();

    assert_eq!(collection.threshold_k(), 2);
    assert_eq!(collection.threshold_n(), 3);
    assert_eq!(collection.shares_count(), 3);
    assert_eq!(collection.secret_id(), &secret_id);
}

#[test]
fn test_share_collection_wrong_share_count() {
    let secret_id = SecretId::generate();
    let shares = create_test_shares(2);

    let result = ShareCollection::new(secret_id, 2, 3, shares);
    assert!(result.is_err());
}

#[test]
fn test_share_collection_invalid_index_zero() {
    let secret_id = SecretId::generate();
    let mut shares = create_test_shares(2);
    shares.push(EncryptedShareData::new(0, vec![1; 32]));

    let result = ShareCollection::new(secret_id, 2, 3, shares);
    assert!(result.is_err());
}

#[test]
fn test_share_collection_invalid_index_too_high() {
    let secret_id = SecretId::generate();
    let mut shares = create_test_shares(2);
    shares.push(EncryptedShareData::new(4, vec![1; 32]));

    let result = ShareCollection::new(secret_id, 2, 3, shares);
    assert!(result.is_err());
}

#[test]
fn test_share_collection_duplicate_index() {
    let secret_id = SecretId::generate();
    let shares = vec![
        EncryptedShareData::new(1, vec![1; 32]),
        EncryptedShareData::new(1, vec![2; 32]),
        EncryptedShareData::new(2, vec![3; 32]),
    ];

    let result = ShareCollection::new(secret_id, 2, 3, shares);
    assert!(result.is_err());
}

#[test]
fn test_share_collection_empty_data() {
    let secret_id = SecretId::generate();
    let shares = vec![
        EncryptedShareData::new(1, vec![1; 32]),
        EncryptedShareData::new(2, vec![]),
        EncryptedShareData::new(3, vec![3; 32]),
    ];

    let result = ShareCollection::new(secret_id, 2, 3, shares);
    assert!(result.is_err());
}

#[test]
fn test_share_collection_get_share() {
    let secret_id = SecretId::generate();
    let shares = create_test_shares(3);

    let collection = ShareCollection::new(secret_id, 2, 3, shares).unwrap();

    let share = collection.get_share(2).unwrap();
    assert_eq!(share.index(), 2);

    assert!(collection.get_share(4).is_none());
}

#[test]
fn test_share_collection_get_shares_by_indices() {
    let secret_id = SecretId::generate();
    let shares = create_test_shares(5);

    let collection = ShareCollection::new(secret_id, 3, 5, shares).unwrap();

    let selected = collection.get_shares_by_indices(&[1, 3, 5]);
    assert_eq!(selected.len(), 3);
}

#[test]
fn test_share_collection_set_arweave_tx_id() {
    let secret_id = SecretId::generate();
    let shares = create_test_shares(3);

    let mut collection = ShareCollection::new(secret_id, 2, 3, shares).unwrap();

    assert!(collection.arweave_tx_id().is_none());

    collection.set_arweave_tx_id("tx-123".to_string());

    assert_eq!(collection.arweave_tx_id(), Some("tx-123"));
}
