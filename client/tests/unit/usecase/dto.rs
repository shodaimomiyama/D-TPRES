use formix::usecase::dto::{SecretMetadata, SecretStatus};

#[test]
fn test_secret_status_can_recover() {
    assert!(SecretStatus::Created.can_recover());
    assert!(SecretStatus::KFragsDistributed.can_recover());
    assert!(SecretStatus::Recovered.can_recover());
    assert!(!SecretStatus::Revoked.can_recover());
}

#[test]
fn test_secret_status_is_active() {
    assert!(SecretStatus::Created.is_active());
    assert!(SecretStatus::KFragsDistributed.is_active());
    assert!(SecretStatus::Recovered.is_active());
    assert!(!SecretStatus::Revoked.is_active());
}

#[test]
fn test_secret_status_display() {
    assert_eq!(format!("{}", SecretStatus::Created), "created");
    assert_eq!(
        format!("{}", SecretStatus::KFragsDistributed),
        "kfrags_distributed"
    );
    assert_eq!(format!("{}", SecretStatus::Recovered), "recovered");
    assert_eq!(format!("{}", SecretStatus::Revoked), "revoked");
}

#[test]
fn test_secret_metadata_default() {
    let metadata = SecretMetadata::default();
    assert!(metadata.name.is_none());
    assert!(metadata.description.is_none());
    assert!(metadata.expires_at.is_none());
    assert!(metadata.tags.is_empty());
}
