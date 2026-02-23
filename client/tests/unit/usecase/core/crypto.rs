use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl, constants};

#[test]
fn test_create_kfrags() {
    let service = CryptoServiceImpl::new();

    let (owner_sk, _owner_pk) = service
        .generate_keypair()
        .expect("Failed to generate owner keypair");
    let (_accessor_sk, accessor_pk) = service
        .generate_keypair()
        .expect("Failed to generate accessor keypair");

    let reencryption_key = service
        .generate_reencryption_key(&owner_sk, &accessor_pk)
        .expect("Failed to generate re-encryption key");

    let kfrags = service
        .create_kfrags(&reencryption_key, 3, 5)
        .expect("Failed to create kFrags");

    assert_eq!(kfrags.len(), 5);

    for kfrag in &kfrags {
        assert!(!kfrag.key_data.is_empty());
    }

    let result = service.create_kfrags(&reencryption_key, 0, 5);
    assert!(result.is_err());

    let result = service.create_kfrags(&reencryption_key, 6, 5);
    assert!(result.is_err());
}

#[test]
fn test_shamir_split_and_reconstruct() {
    let service = CryptoServiceImpl::new();
    let secret = b"This is a test secret for Shamir!";

    let result = service.split_secret_shamir(secret, 0, 5);
    assert!(result.is_err());

    let result = service.split_secret_shamir(secret, 6, 5);
    assert!(result.is_err());

    let result = service.split_secret_shamir(b"", 3, 5);
    assert!(result.is_err());

    let result = service.split_secret_shamir(secret, 3, 10);
    assert!(result.is_ok());

    let shares = result.unwrap();
    assert_eq!(shares.len(), 10);

    let min_shares = &shares[0..4];
    let reconstruct_result = service.reconstruct_secret_shamir(min_shares, 4);
    assert!(reconstruct_result.is_ok());

    let recovered = reconstruct_result.unwrap();
    assert_eq!(recovered, secret);

    let different_shares = vec![shares[1].clone(), shares[2].clone(), shares[4].clone()];
    let reconstruct_result2 = service.reconstruct_secret_shamir(&different_shares, 3);
    assert!(reconstruct_result2.is_ok());

    let recovered2 = reconstruct_result2.unwrap();
    assert_eq!(recovered2, secret);

    let insufficient_shares = &shares[0..2];
    let reconstruct_fail = service.reconstruct_secret_shamir(insufficient_shares, 3);
    assert!(reconstruct_fail.is_err());
}

#[test]
fn test_proxy_reencrypt_full_flow() {
    let service = CryptoServiceImpl::new();

    let (alice_sk, alice_pk) = service
        .generate_keypair()
        .expect("Failed to generate Alice keypair");
    let (bob_sk, bob_pk) = service
        .generate_keypair()
        .expect("Failed to generate Bob keypair");

    let plaintext = b"Secret message for proxy re-encryption test";
    let (capsule, ciphertext) = service
        .create_pre_capsule(&alice_pk, plaintext)
        .expect("Failed to create capsule");

    assert!(!capsule.capsule_bytes.is_empty());
    assert!(!ciphertext.is_empty());

    let reencryption_key = service
        .generate_reencryption_key(&alice_sk, &bob_pk)
        .expect("Failed to generate re-encryption key");

    let kfrags = service
        .create_kfrags(&reencryption_key, 2, 3)
        .expect("Failed to create kFrags");

    let mut cfrags = Vec::new();
    for kfrag in kfrags.iter().take(2) {
        let cfrag = service
            .proxy_reencrypt(kfrag, &capsule)
            .expect("Failed to re-encrypt");
        cfrags.push(cfrag);
    }

    let decrypted = service
        .combine_and_decrypt(&cfrags, &bob_sk, &capsule, &ciphertext)
        .expect("Failed to decrypt reencrypted data");

    assert_eq!(
        plaintext,
        &decrypted[..],
        "Decrypted data should match original plaintext"
    );
}

#[test]
fn test_proxy_reencrypt() {
    let service = CryptoServiceImpl::new();

    let (alice_sk, alice_pk) = service
        .generate_keypair()
        .expect("Failed to generate Alice keypair");
    let (_bob_sk, bob_pk) = service
        .generate_keypair()
        .expect("Failed to generate Bob keypair");

    let plaintext = b"Test message";
    let (capsule, _ciphertext) = service
        .create_pre_capsule(&alice_pk, plaintext)
        .expect("Failed to create capsule");

    let reencryption_key = service
        .generate_reencryption_key(&alice_sk, &bob_pk)
        .expect("Failed to generate re-encryption key");
    let kfrags = service
        .create_kfrags(&reencryption_key, 2, 2)
        .expect("Failed to create kFrags");

    let cfrag = service
        .proxy_reencrypt(&kfrags[0], &capsule)
        .expect("Failed to re-encrypt");

    assert!(!cfrag.capsule_fragment.is_empty());
}

#[test]
fn test_aes_gcm_encrypt_decrypt_roundtrip() {
    let service = CryptoServiceImpl::new();

    let key = service
        .generate_symmetric_key()
        .expect("Failed to generate key");
    assert_eq!(key.len(), 32);

    let test_cases = [
        b"Hello, World!".to_vec(),
        b"".to_vec(),
        vec![0x42u8; 1024],
        vec![0xABu8; 16],
    ];

    for plaintext in &test_cases {
        let ciphertext = service
            .aes_gcm_encrypt(&key, plaintext)
            .expect("Encryption failed");

        assert!(
            ciphertext.len() >= constants::AES_GCM_NONCE_SIZE + constants::AES_GCM_TAG_SIZE,
            "Ciphertext too short"
        );

        let decrypted = service
            .aes_gcm_decrypt(&key, &ciphertext)
            .expect("Decryption failed");

        assert_eq!(
            plaintext, &decrypted,
            "Decrypted data should match original"
        );
    }
}

#[test]
fn test_aes_gcm_decrypt_invalid_key() {
    let service = CryptoServiceImpl::new();

    let key1 = service
        .generate_symmetric_key()
        .expect("Failed to generate key1");
    let key2 = service
        .generate_symmetric_key()
        .expect("Failed to generate key2");

    let plaintext = b"Secret message";

    let ciphertext = service
        .aes_gcm_encrypt(&key1, plaintext)
        .expect("Encryption failed");

    let result = service.aes_gcm_decrypt(&key2, &ciphertext);
    assert!(result.is_err(), "Decryption with wrong key should fail");

    let short_key = vec![0u8; 16];
    let result = service.aes_gcm_encrypt(&short_key, plaintext);
    assert!(
        result.is_err(),
        "Encryption with invalid key length should fail"
    );
}

#[test]
fn test_aes_gcm_decrypt_corrupted_ciphertext() {
    let service = CryptoServiceImpl::new();
    let key = service
        .generate_symmetric_key()
        .expect("Failed to generate key");

    let plaintext = b"Original message";
    let mut ciphertext = service
        .aes_gcm_encrypt(&key, plaintext)
        .expect("Encryption failed");

    if ciphertext.len() > constants::AES_GCM_NONCE_SIZE {
        ciphertext[constants::AES_GCM_NONCE_SIZE] ^= 0xFF;
    }

    let result = service.aes_gcm_decrypt(&key, &ciphertext);
    assert!(
        result.is_err(),
        "Decryption of corrupted ciphertext should fail"
    );

    let truncated = vec![0u8; 10];
    let result = service.aes_gcm_decrypt(&key, &truncated);
    assert!(
        result.is_err(),
        "Decryption of truncated ciphertext should fail"
    );
}

#[test]
fn test_generate_symmetric_key_length() {
    let service = CryptoServiceImpl::new();

    for _ in 0..10 {
        let key = service
            .generate_symmetric_key()
            .expect("Failed to generate key");
        assert_eq!(key.len(), constants::AES_KEY_SIZE, "Key should be 32 bytes");
    }
}

#[test]
fn test_generate_symmetric_key_randomness() {
    let service = CryptoServiceImpl::new();

    let mut keys: Vec<Vec<u8>> = Vec::new();

    for _ in 0..100 {
        let key = service
            .generate_symmetric_key()
            .expect("Failed to generate key");
        keys.push(key);
    }

    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(keys[i], keys[j], "Keys {} and {} should be different", i, j);
        }
    }

    for (i, key) in keys.iter().take(5).enumerate() {
        let all_same = key.iter().all(|&b| b == key[0]);
        assert!(!all_same, "Key {} should not have all identical bytes", i);
    }
}
