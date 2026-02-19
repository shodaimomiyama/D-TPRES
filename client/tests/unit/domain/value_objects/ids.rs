use formix::domain::{CFragId, CapsuleId, KFragId, SecretId, ShareCollectionId};

#[test]
fn test_secret_id_new_and_as_str() {
    let id = SecretId::new("test-secret-123");
    assert_eq!(id.as_str(), "test-secret-123");
}

#[test]
fn test_secret_id_generate() {
    let id1 = SecretId::generate();
    let id2 = SecretId::generate();
    assert_ne!(id1, id2);
}

#[test]
fn test_secret_id_equality() {
    let id1 = SecretId::new("same-id");
    let id2 = SecretId::new("same-id");
    assert_eq!(id1, id2);
}

#[test]
fn test_share_collection_id() {
    let id = ShareCollectionId::new("collection-1");
    assert_eq!(id.as_str(), "collection-1");
}

#[test]
fn test_capsule_id() {
    let id = CapsuleId::new("capsule-1");
    assert_eq!(id.as_str(), "capsule-1");
}

#[test]
fn test_kfrag_id() {
    let id = KFragId::new("kfrag-1");
    assert_eq!(id.as_str(), "kfrag-1");
}

#[test]
fn test_cfrag_id() {
    let id = CFragId::new("cfrag-1");
    assert_eq!(id.as_str(), "cfrag-1");
}

#[test]
fn test_type_safety_compile_time() {
    let _secret_id = SecretId::new("secret");
    let _collection_id = ShareCollectionId::new("collection");
}
