//! CryptoService - 暗号化操作を担当する中核サービス
//!
//! Threshold Proxy Re-Encryption (TPRE) とShamir Secret Sharingの実装を提供します。
//! AO環境の制約に従い、すべての操作は同期的に実行されます。

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use bincode;
use generic_array::{GenericArray, typenum::U32};
use shamirsecretsharing::{DATA_SIZE, combine_shares, create_shares};
use subtle::ConstantTimeEq;
use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::service::error::{BusinessException, ServiceError, ServiceResult};

/// 検証用データ構造体
/// kFragの検証に必要な公開鍵情報を保持
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationData {
    /// 署名検証用の公開鍵
    verifying_pk: Vec<u8>,
    /// 委任者（元の所有者）の公開鍵
    delegating_pk: Vec<u8>,
    /// 受信者（アクセス者）の公開鍵
    receiving_pk: Vec<u8>,
}

use serde::{Deserialize, Serialize};

/// Capsule payload for Arweave storage
///
/// Bundles capsule, ciphertext, and verifying_pk into a single serializable unit.
/// Phase 1 stores this as a single Arweave transaction; Phase 3 deserializes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CapsulePayload {
    pub capsule_bytes: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub verifying_pk: Vec<u8>,
}

// 型エイリアス：秘密鍵のバイト表現
type SecretKeyBytes = umbral_pre::SecretBox<GenericArray<u8, U32>>;

/// 暗号パラメータ定数
pub mod constants {
    /// 最小閾値
    pub const MIN_THRESHOLD: u8 = 2;

    /// シェアの最大生成数
    pub const MAX_SHARES: u8 = 20;

    /// 鍵サイズ（バイト）
    pub const KEY_SIZE_BYTES: usize = 32;

    /// 公開鍵サイズ（バイト、圧縮形式）
    pub const PUBLIC_KEY_SIZE_BYTES: usize = 33;

    /// カプセルサイズ定数
    pub const CAPSULE_POINT_SIZE: usize = 33;
    pub const CAPSULE_SIGNATURE_SIZE: usize = 64;

    /// AES-256-GCM Nonce size (12 bytes)
    pub const AES_GCM_NONCE_SIZE: usize = 12;

    /// AES-256-GCM Tag size (16 bytes)
    pub const AES_GCM_TAG_SIZE: usize = 16;

    /// AES-256 key size (32 bytes)
    pub const AES_KEY_SIZE: usize = 32;
}

/// Shamir Secret Sharingのシェア
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct ShamirShare {
    /// シェアのインデックス（1から開始）
    pub index: u8,
    /// シェアデータ（使用後にゼロ化される）
    pub share_data: Vec<u8>,
}

/// Umbral暗号化のカプセル（シリアライズされた不透明トークン）
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Capsule {
    /// umbral_pre::Capsuleのシリアライズされた完全なデータ
    pub capsule_bytes: Vec<u8>,
}

/// 公開鍵
///
/// プライバシー保護のため、Zeroizeトレイトを実装
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct PublicKey {
    pub key_data: Vec<u8>,
}

/// 秘密鍵（使用後にゼロ化される）
#[non_exhaustive]
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    key_data: Vec<u8>,
}

impl SecretKey {
    /// Check if the key data is empty
    pub fn is_empty(&self) -> bool {
        self.key_data.is_empty()
    }

    /// Create an empty SecretKey for testing purposes only
    #[cfg(test)]
    pub fn empty_for_test() -> Self {
        Self { key_data: vec![] }
    }
}

/// 再暗号化鍵
///
/// 実際にはkFrags生成に必要な情報を保持する中間構造体
#[non_exhaustive]
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct ReencryptionKey {
    /// 委任者の秘密鍵（シリアライズ済み）
    delegating_sk_data: Vec<u8>,
    /// 受信者の公開鍵（シリアライズ済み）
    receiving_pk_data: Vec<u8>,
}

/// 鍵フラグメント（kFrag）
///
/// Arweaveストレージ用にシリアライズ可能な形式
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct KeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,          // KeyFragのシリアライズデータ
    pub verification_data: Vec<u8>, // 検証用データ（公開鍵情報）
    pub precursor: Vec<u8>,         // 現在は使用しない（将来的な拡張用）
}

/// 暗号フラグメント（cFrag）
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct CipherFragment {
    pub fragment_id: u8,
    pub capsule_fragment: Vec<u8>,
    pub proof: Vec<u8>,
}

/// cFragデータ（AO Network/Arweaveから取得したcFrag）
///
/// WorkflowServiceがAOから取得したcFragデータを表す構造体
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CFragData {
    /// cFragのシリアライズデータ
    pub cfrag_data: Vec<u8>,
    /// このcFragを生成したHolder ID
    pub holder_id: String,
}

/// CryptoService trait - 暗号化操作のインターフェース
pub trait CryptoService: Send + Sync {
    /// Shamir Secret Sharingによる秘密の分割
    fn split_secret_shamir(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> ServiceResult<Vec<ShamirShare>>;

    /// Shamir Secret Sharingによる秘密の復元
    fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> ServiceResult<Vec<u8>>;

    /// PRE暗号化によるカプセル作成
    fn create_pre_capsule(
        &self,
        public_key: &PublicKey,
        plaintext: &[u8],
    ) -> ServiceResult<(Capsule, Vec<u8>)>;

    /// 再暗号化鍵の生成
    fn generate_reencryption_key(
        &self,
        owner_secret_key: &SecretKey,
        accessor_public_key: &PublicKey,
    ) -> ServiceResult<ReencryptionKey>;

    /// 再暗号化鍵からkFragsを作成
    fn create_kfrags(
        &self,
        reencryption_key: &ReencryptionKey,
        threshold: u8,
        total_fragments: u8,
    ) -> ServiceResult<Vec<KeyFragment>>;

    /// プロキシ再暗号化の実行
    fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        _capsule: &Capsule,
    ) -> ServiceResult<CipherFragment>;

    /// cFragsの結合と復号
    fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        _original_capsule: &Capsule,
        ciphertext: &[u8],
    ) -> ServiceResult<Vec<u8>>;

    /// 鍵ペアの生成
    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)>;

    /// 秘密鍵から公開鍵を導出
    ///
    /// # Arguments
    /// * `secret_key` - 公開鍵を導出する秘密鍵
    ///
    /// # Returns
    /// * `ServiceResult<PublicKey>` - 導出された公開鍵
    fn derive_public_key(&self, secret_key: &SecretKey) -> ServiceResult<PublicKey>;

    /// AES-256-GCM暗号化
    ///
    /// 12バイトのランダムnonceを生成し、暗号文の先頭に付加して返します。
    /// 返却形式: nonce (12 bytes) || ciphertext || tag (16 bytes)
    fn aes_gcm_encrypt(&self, key: &[u8], plaintext: &[u8]) -> ServiceResult<Vec<u8>>;

    /// AES-256-GCM復号
    ///
    /// 暗号文の先頭12バイトをnonceとして使用し、残りを復号します。
    /// 入力形式: nonce (12 bytes) || ciphertext || tag (16 bytes)
    fn aes_gcm_decrypt(&self, key: &[u8], ciphertext: &[u8]) -> ServiceResult<Vec<u8>>;

    /// 対称鍵生成
    ///
    /// AES-256用の32バイト暗号学的に安全な乱数を生成します。
    fn generate_symmetric_key(&self) -> ServiceResult<Vec<u8>>;

    /// Serialize the instance's verifying key for storage
    fn verifying_key_bytes(&self) -> ServiceResult<Vec<u8>>;

    /// PRE Capsule decryption using cFrags
    ///
    /// Decrypts a re-encrypted capsule using verified cFrags and recovers the plaintext.
    fn decrypt_pre_capsule(
        &self,
        capsule: &Capsule,
        cfrags: &[CFragData],
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        ciphertext: &[u8],
        verifying_pk_bytes: &[u8],
    ) -> ServiceResult<Vec<u8>>;
}

/// CryptoService実装 - Umbral-PREとShamirライブラリを使用
pub struct CryptoServiceImpl {
    /// kFrag生成用のSigner
    signer: umbral_pre::Signer,
    /// 検証用の公開鍵
    verifying_key: umbral_pre::PublicKey,
}

impl CryptoServiceImpl {
    /// 新しいCryptoServiceインスタンスを作成
    pub fn new() -> Self {
        // 署名用の鍵を生成
        let signing_key: umbral_pre::SecretKey = umbral_pre::SecretKey::random();
        let verifying_key: umbral_pre::PublicKey = signing_key.public_key();
        let signer: umbral_pre::Signer = umbral_pre::Signer::new(signing_key);

        Self {
            signer,
            verifying_key,
        }
    }

    /// 秘密鍵をバイト配列に変換するヘルパー関数
    fn serialize_secret_key(sk: &umbral_pre::SecretKey) -> Vec<u8> {
        let sk_bytes: SecretKeyBytes = sk.to_be_bytes();
        sk_bytes.as_secret().to_vec()
    }

    /// 定数時間でバイト配列を比較
    ///
    /// # セキュリティ
    /// この関数はタイミング攻撃を防ぐため定数時間で実行されます
    #[allow(dead_code)]
    #[inline]
    fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.ct_eq(b).into()
    }

    /// SecretKeyからumbral_pre::SecretKeyを復元
    ///
    /// # セキュリティ
    /// - メモリ上の秘密鍵は使用後自動的にクリアされます
    /// - 一時的な秘密鍵は生成されますが、スコープ外で自動的にクリアされます
    fn deserialize_secret_key(&self, key: &SecretKey) -> ServiceResult<umbral_pre::SecretKey> {
        // key_dataをGenericArray<u8, U32>に変換
        if key.key_data.len() != constants::KEY_SIZE_BYTES {
            return Err(ServiceError::crypto_error("Invalid secret key format"));
        }

        // 一時的な秘密鍵を生成してバイト表現を取得
        // この方法が現在のumbral-pre APIでは必要
        let temp_sk = umbral_pre::SecretKey::random();
        let mut secret_bytes = temp_sk.to_be_bytes();

        // 実際のデータをコピー（メモリ安全性のため直ちに実行）
        secret_bytes.as_mut_secret().copy_from_slice(&key.key_data);

        // SecretKeyに変換
        // secret_bytesは自動的にZeroizeされる（SecretBoxのDrop実装により）
        umbral_pre::SecretKey::try_from_be_bytes(&secret_bytes)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize secret key"))
    }

    /// PublicKeyからumbral_pre::PublicKeyを復元
    fn deserialize_public_key(&self, key: &PublicKey) -> ServiceResult<umbral_pre::PublicKey> {
        bincode::deserialize(&key.key_data)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))
    }

    /// Capsuleからumbral_pre::Capsuleを復元
    fn deserialize_capsule(&self, capsule: &Capsule) -> ServiceResult<umbral_pre::Capsule> {
        bincode::deserialize(&capsule.capsule_bytes)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize capsule"))
    }
}

#[allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
impl CryptoService for CryptoServiceImpl {
    fn split_secret_shamir(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> ServiceResult<Vec<ShamirShare>> {
        // 入力検証
        if threshold < constants::MIN_THRESHOLD {
            return Err(ServiceError::validation_error(
                "Threshold below minimum requirement",
            ));
        }

        if total_shares > constants::MAX_SHARES {
            return Err(ServiceError::validation_error("Too many shares requested"));
        }

        if threshold > total_shares {
            return Err(ServiceError::validation_error(
                "Threshold cannot exceed total shares",
            ));
        }

        if secret.is_empty() {
            return Err(ServiceError::validation_error("Secret cannot be empty"));
        }

        // shamirsecretsharing は 64バイト固定長を要求
        let mut padded_secret: Vec<u8> = vec![0u8; DATA_SIZE];

        if secret.len() < DATA_SIZE {
            // パディング: 最初にデータ長を格納し、その後に実データをコピー
            padded_secret[0] = secret.len() as u8;
            padded_secret[1..secret.len() + 1].copy_from_slice(secret);
        } else {
            return Err(ServiceError::validation_error(
                "Secret data exceeds maximum size",
            ));
        }

        // ライブラリを使用してシェアを作成
        let shares: Vec<Vec<u8>> = create_shares(&padded_secret, total_shares, threshold)
            .map_err(|_| ServiceError::crypto_error("Failed to create shares"))?;

        // Vec<Vec<u8>>からShamirShare型へ変換
        let mut result: Vec<ShamirShare> = Vec::with_capacity(shares.len());
        for (index, share) in shares.into_iter().enumerate() {
            result.push(ShamirShare {
                index: (index + 1) as u8, // 1から開始
                share_data: share,
            });
        }

        Ok(result)
    }

    fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> ServiceResult<Vec<u8>> {
        if shares.len() < threshold as usize {
            return Err(ServiceError::Business(BusinessException::ThresholdNotMet {
                required: threshold,
                actual: shares.len() as u8,
            }));
        }

        // ShamirShare型からVec<u8>へ変換（shamirsecretsharing::ShareはVec<u8>型）
        let share_vecs: Vec<Vec<u8>> = shares
            .iter()
            .map(|s| s.share_data.as_slice().to_vec())
            .collect();

        // ライブラリを使用してシェアを結合
        let recovered = combine_shares(&share_vecs)
            .map_err(|_| ServiceError::crypto_error("Failed to combine shares"))?;

        // 復元されたデータを処理
        match recovered {
            Some(recovered_bytes) => {
                // パディングを除去して元のデータを取得
                if recovered_bytes.is_empty() {
                    return Err(ServiceError::crypto_error("Recovered data is empty"));
                }

                let original_len = recovered_bytes[0] as usize;
                if original_len == 0 || original_len > DATA_SIZE - 1 {
                    return Err(ServiceError::crypto_error("Invalid recovered data format"));
                }

                // 元のデータを抽出
                let result = recovered_bytes[1..original_len + 1].to_vec();
                Ok(result)
            }
            None => Err(ServiceError::crypto_error(
                "Failed to recover secret from shares",
            )),
        }
    }

    fn create_pre_capsule(
        &self,
        public_key: &PublicKey,
        plaintext: &[u8],
    ) -> ServiceResult<(Capsule, Vec<u8>)> {
        // 入力検証
        if public_key.key_data.is_empty() {
            return Err(ServiceError::validation_error("Invalid public key"));
        }

        if plaintext.is_empty() {
            return Err(ServiceError::validation_error("Plaintext cannot be empty"));
        }

        // PublicKeyをデシリアライズ
        let umbral_pk: umbral_pre::PublicKey = self.deserialize_public_key(public_key)?;

        // umbral-preで暗号化を実行
        let (umbral_capsule, ciphertext_box) = umbral_pre::encrypt(&umbral_pk, plaintext)
            .map_err(|_| ServiceError::crypto_error("Encryption failed"))?;

        // Capsuleをシリアライズして保存
        let serialized_capsule: Vec<u8> = bincode::serialize(&umbral_capsule)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize capsule"))?;

        let capsule: Capsule = Capsule {
            capsule_bytes: serialized_capsule,
        };

        // ciphertextをVec<u8>に変換
        let ciphertext: Vec<u8> = ciphertext_box.to_vec();

        Ok((capsule, ciphertext))
    }

    fn generate_reencryption_key(
        &self,
        owner_secret_key: &SecretKey,
        accessor_public_key: &PublicKey,
    ) -> ServiceResult<ReencryptionKey> {
        // 入力検証
        if owner_secret_key.key_data.is_empty() {
            return Err(ServiceError::validation_error("Invalid owner secret key"));
        }

        if accessor_public_key.key_data.is_empty() {
            return Err(ServiceError::validation_error(
                "Invalid accessor public key",
            ));
        }

        // 再暗号化鍵の生成
        // umbral-preではgenerate_kfragsで直接kFragsを生成するため、
        // ここでは必要な情報を保持する中間構造体を返す
        Ok(ReencryptionKey {
            delegating_sk_data: owner_secret_key.key_data.to_vec(),
            receiving_pk_data: accessor_public_key.key_data.to_vec(),
        })
    }

    fn create_kfrags(
        &self,
        reencryption_key: &ReencryptionKey,
        threshold: u8,
        total_fragments: u8,
    ) -> ServiceResult<Vec<KeyFragment>> {
        // 入力検証
        if threshold < constants::MIN_THRESHOLD {
            return Err(ServiceError::validation_error(
                "Threshold below minimum requirement",
            ));
        }

        if total_fragments > constants::MAX_SHARES {
            return Err(ServiceError::validation_error(
                "Too many fragments requested",
            ));
        }

        if threshold > total_fragments {
            return Err(ServiceError::validation_error(
                "Threshold cannot exceed total fragments",
            ));
        }

        if reencryption_key.delegating_sk_data.is_empty()
            || reencryption_key.receiving_pk_data.is_empty()
        {
            return Err(ServiceError::validation_error("Invalid reencryption key"));
        }

        // 委任者の秘密鍵をデシリアライズ
        let delegating_sk = SecretKey {
            key_data: reencryption_key.delegating_sk_data.to_vec(),
        };
        let umbral_delegating_sk = self.deserialize_secret_key(&delegating_sk)?;

        // 受信者の公開鍵をデシリアライズ
        let receiving_pk = PublicKey {
            key_data: reencryption_key.receiving_pk_data.to_vec(),
        };
        let umbral_receiving_pk = self.deserialize_public_key(&receiving_pk)?;

        // 委任者の公開鍵を生成（検証用）
        let umbral_delegating_pk = umbral_delegating_sk.public_key();

        // kFragsを生成
        let verified_kfrags = umbral_pre::generate_kfrags(
            &umbral_delegating_sk,
            &umbral_receiving_pk,
            &self.signer,
            threshold as usize,
            total_fragments as usize,
            true, // sign_delegating_key
            true, // sign_receiving_key
        );

        // 検証用データを作成
        let verification_data = VerificationData {
            verifying_pk: bincode::serialize(&self.verifying_key)
                .map_err(|_| ServiceError::crypto_error("Failed to serialize key"))?,
            delegating_pk: bincode::serialize(&umbral_delegating_pk)
                .map_err(|_| ServiceError::crypto_error("Failed to serialize key"))?,
            receiving_pk: reencryption_key.receiving_pk_data.to_vec(),
        };
        let verification_bytes = bincode::serialize(&verification_data)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize verification data"))?;

        // VerifiedKeyFragをKeyFragmentに変換
        let mut kfrags = Vec::with_capacity(verified_kfrags.len());
        for (index, verified_kfrag) in verified_kfrags.iter().enumerate() {
            // VerifiedKeyFragをKeyFragに変換（unverify）してからシリアライズ
            let kfrag = verified_kfrag.clone().unverify();
            let kfrag_bytes = kfrag
                .to_bytes()
                .map_err(|_| ServiceError::crypto_error("Failed to serialize fragment"))?;

            kfrags.push(KeyFragment {
                id: index as u8,
                key_data: kfrag_bytes.to_vec(),
                verification_data: verification_bytes.to_vec(), // 検証用データを保存
                precursor: vec![],                              // 現在は使用しない
            });
        }

        Ok(kfrags)
    }

    fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        capsule: &Capsule,
    ) -> ServiceResult<CipherFragment> {
        // 入力検証
        if kfrag.key_data.is_empty() {
            return Err(ServiceError::validation_error("Invalid key fragment"));
        }

        if capsule.capsule_bytes.is_empty() {
            return Err(ServiceError::validation_error("Invalid capsule"));
        }

        // 検証データを取得（必須）
        if kfrag.verification_data.is_empty() {
            return Err(ServiceError::validation_error(
                "Missing verification data for kFrag",
            ));
        }

        let verification_data = bincode::deserialize::<VerificationData>(&kfrag.verification_data)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize verification data"))?;

        // KeyFragをデシリアライズ
        let umbral_kfrag = umbral_pre::KeyFrag::from_bytes(&kfrag.key_data)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize fragment"))?;

        // 常に署名検証を実行
        let verifying_pk: umbral_pre::PublicKey =
            bincode::deserialize(&verification_data.verifying_pk)
                .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))?;
        let delegating_pk: umbral_pre::PublicKey =
            bincode::deserialize(&verification_data.delegating_pk)
                .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))?;
        let receiving_pk = self.deserialize_public_key(&PublicKey {
            key_data: verification_data.receiving_pk.to_vec(),
        })?;

        let verified_kfrag = umbral_kfrag
            .verify(&verifying_pk, Some(&delegating_pk), Some(&receiving_pk))
            .map_err(|_| ServiceError::crypto_error("Failed to verify fragment"))?;

        // Capsuleをデシリアライズ
        let umbral_capsule = self.deserialize_capsule(capsule)?;

        // 再暗号化を実行
        let verified_cfrag = umbral_pre::reencrypt(&umbral_capsule, verified_kfrag);

        // VerifiedCapsuleFragをunverifyしてからシリアライズ
        let cfrag = verified_cfrag.unverify();
        let cfrag_bytes = cfrag
            .to_bytes()
            .map_err(|_| ServiceError::crypto_error("Failed to serialize capsule fragment"))?
            .to_vec();

        // CapsuleFrag検証用データを準備（既存の検証データを再利用）
        let cfrag_verification_bytes = bincode::serialize(&verification_data)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize verification data"))?;

        Ok(CipherFragment {
            fragment_id: kfrag.id,
            capsule_fragment: cfrag_bytes,
            proof: cfrag_verification_bytes, // 検証データを保存
        })
    }

    fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        original_capsule: &Capsule,
        ciphertext: &[u8],
    ) -> ServiceResult<Vec<u8>> {
        // 入力検証
        if cfrags.is_empty() {
            return Err(ServiceError::validation_error(
                "No cipher fragments provided",
            ));
        }

        if accessor_secret_key.key_data.is_empty() {
            return Err(ServiceError::validation_error(
                "Invalid accessor secret key",
            ));
        }

        if original_capsule.capsule_bytes.is_empty() {
            return Err(ServiceError::validation_error("Invalid original capsule"));
        }

        if ciphertext.is_empty() {
            return Err(ServiceError::validation_error("Ciphertext cannot be empty"));
        }

        // accessor（受信者）の秘密鍵をデシリアライズ
        let receiving_sk = self.deserialize_secret_key(accessor_secret_key)?;

        // オリジナルのCapsuleをデシリアライズ
        let umbral_capsule = self.deserialize_capsule(original_capsule)?;

        // CapsuleFragsをデシリアライズして検証
        let mut verified_cfrags = Vec::with_capacity(cfrags.len());

        // 最初のcFragから検証データを取得（すべてのcFragは同じ検証データを持つべき）
        if cfrags.is_empty() {
            return Err(ServiceError::validation_error(
                "No cipher fragments to decrypt",
            ));
        }

        // 検証データを取得
        if cfrags[0].proof.is_empty() {
            return Err(ServiceError::validation_error(
                "Missing verification data for CapsuleFrag",
            ));
        }

        let verification_data = bincode::deserialize::<VerificationData>(&cfrags[0].proof)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize verification data"))?;

        // 検証用の公開鍵をデシリアライズ
        let verifying_pk: umbral_pre::PublicKey =
            bincode::deserialize(&verification_data.verifying_pk)
                .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))?;
        let delegating_pk: umbral_pre::PublicKey =
            bincode::deserialize(&verification_data.delegating_pk)
                .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))?;
        let receiving_pk = self.deserialize_public_key(&PublicKey {
            key_data: verification_data.receiving_pk.to_vec(),
        })?;

        for cfrag in cfrags {
            // CapsuleFragをbytesからデシリアライズ
            let capsule_frag = umbral_pre::CapsuleFrag::from_bytes(&cfrag.capsule_fragment)
                .map_err(|_| ServiceError::crypto_error("Failed to deserialize fragment"))?;

            // 常に署名検証を実行
            let verified_cfrag = capsule_frag
                .verify(
                    &umbral_capsule,
                    &verifying_pk,
                    &delegating_pk,
                    &receiving_pk,
                )
                .map_err(|_| ServiceError::crypto_error("Failed to verify fragment"))?;

            verified_cfrags.push(verified_cfrag);
        }

        // 再暗号化されたデータを復号（検証データから取得したdelegating_pkを使用）
        let plaintext = umbral_pre::decrypt_reencrypted(
            &receiving_sk,
            &delegating_pk,
            &umbral_capsule,
            verified_cfrags,
            ciphertext,
        )
        .map_err(|_| ServiceError::crypto_error("Failed to decrypt"))?;

        Ok(plaintext.to_vec())
    }

    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)> {
        // umbral-preを使用して実際の鍵ペアを生成
        let umbral_sk: umbral_pre::SecretKey = umbral_pre::SecretKey::random();
        let umbral_pk: umbral_pre::PublicKey = umbral_sk.public_key();

        // ヘルパー関数を使用して秘密鍵をシリアライズ
        let sk_vec: Vec<u8> = Self::serialize_secret_key(&umbral_sk);

        // 公開鍵をシリアライズ
        let pk_bytes: Vec<u8> = bincode::serialize(&umbral_pk)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize key"))?;

        // 既存の構造体に格納
        let secret_key: SecretKey = SecretKey { key_data: sk_vec };

        let public_key: PublicKey = PublicKey { key_data: pk_bytes };

        Ok((secret_key, public_key))
    }

    fn derive_public_key(&self, secret_key: &SecretKey) -> ServiceResult<PublicKey> {
        // Deserialize the secret key
        let umbral_sk = self.deserialize_secret_key(secret_key)?;

        // Derive the public key
        let umbral_pk: umbral_pre::PublicKey = umbral_sk.public_key();

        // Serialize the public key
        let pk_bytes: Vec<u8> = bincode::serialize(&umbral_pk)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize public key"))?;

        Ok(PublicKey { key_data: pk_bytes })
    }

    fn aes_gcm_encrypt(&self, key: &[u8], plaintext: &[u8]) -> ServiceResult<Vec<u8>> {
        // Validate key length
        if key.len() != constants::AES_KEY_SIZE {
            return Err(ServiceError::validation_error(
                "Invalid key length: expected 32 bytes for AES-256",
            ));
        }

        // Create cipher from key
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| ServiceError::crypto_error("Failed to create AES-GCM cipher"))?;

        // Generate random 12-byte nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt plaintext
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| ServiceError::crypto_error("AES-GCM encryption failed"))?;

        // Prepend nonce to ciphertext: nonce (12 bytes) || ciphertext || tag (16 bytes)
        let mut result = Vec::with_capacity(constants::AES_GCM_NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    fn aes_gcm_decrypt(&self, key: &[u8], ciphertext: &[u8]) -> ServiceResult<Vec<u8>> {
        // Validate key length
        if key.len() != constants::AES_KEY_SIZE {
            return Err(ServiceError::validation_error(
                "Invalid key length: expected 32 bytes for AES-256",
            ));
        }

        // Validate minimum ciphertext length (nonce + at least tag)
        let min_length = constants::AES_GCM_NONCE_SIZE + constants::AES_GCM_TAG_SIZE;
        if ciphertext.len() < min_length {
            return Err(ServiceError::validation_error(
                "Ciphertext too short: must include nonce and authentication tag",
            ));
        }

        // Extract nonce from first 12 bytes
        let nonce = Nonce::from_slice(&ciphertext[..constants::AES_GCM_NONCE_SIZE]);

        // Create cipher from key
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| ServiceError::crypto_error("Failed to create AES-GCM cipher"))?;

        // Decrypt remaining bytes (ciphertext + tag)
        let plaintext = cipher
            .decrypt(nonce, &ciphertext[constants::AES_GCM_NONCE_SIZE..])
            .map_err(|_| {
                ServiceError::crypto_error(
                    "AES-GCM decryption failed: invalid ciphertext or authentication tag",
                )
            })?;

        Ok(plaintext)
    }

    fn generate_symmetric_key(&self) -> ServiceResult<Vec<u8>> {
        use rand::RngCore;

        let mut key = vec![0u8; constants::AES_KEY_SIZE];
        OsRng.fill_bytes(&mut key);

        Ok(key)
    }

    fn verifying_key_bytes(&self) -> ServiceResult<Vec<u8>> {
        bincode::serialize(&self.verifying_key)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize verifying key"))
    }

    fn decrypt_pre_capsule(
        &self,
        capsule: &Capsule,
        cfrags: &[CFragData],
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        ciphertext: &[u8],
        verifying_pk_bytes: &[u8],
    ) -> ServiceResult<Vec<u8>> {
        if capsule.capsule_bytes.is_empty() {
            return Err(ServiceError::validation_error("Capsule cannot be empty"));
        }

        if cfrags.is_empty() {
            return Err(ServiceError::validation_error("No cFrags provided"));
        }

        if requester_secret_key.key_data.is_empty() {
            return Err(ServiceError::validation_error(
                "Requester secret key cannot be empty",
            ));
        }

        if owner_public_key.key_data.is_empty() {
            return Err(ServiceError::validation_error(
                "Owner public key cannot be empty",
            ));
        }

        if ciphertext.is_empty() {
            return Err(ServiceError::validation_error("Ciphertext cannot be empty"));
        }

        if verifying_pk_bytes.is_empty() {
            return Err(ServiceError::validation_error(
                "Verifying public key cannot be empty",
            ));
        }

        let umbral_capsule = self.deserialize_capsule(capsule)?;
        let receiving_sk = self.deserialize_secret_key(requester_secret_key)?;
        let delegating_pk = self.deserialize_public_key(owner_public_key)?;
        let verifying_pk: umbral_pre::PublicKey = bincode::deserialize(verifying_pk_bytes)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize verifying key"))?;
        let receiving_pk = receiving_sk.public_key();

        let mut verified_cfrags = Vec::with_capacity(cfrags.len());

        for cfrag_data in cfrags {
            if cfrag_data.cfrag_data.is_empty() {
                return Err(ServiceError::validation_error(format!(
                    "Empty cFrag data from holder {}",
                    cfrag_data.holder_id
                )));
            }

            let capsule_frag = umbral_pre::CapsuleFrag::from_bytes(&cfrag_data.cfrag_data)
                .map_err(|_| {
                    ServiceError::crypto_error(format!(
                        "Failed to deserialize cFrag from holder {}",
                        cfrag_data.holder_id
                    ))
                })?;

            let verified_cfrag = capsule_frag
                .verify(
                    &umbral_capsule,
                    &verifying_pk,
                    &delegating_pk,
                    &receiving_pk,
                )
                .map_err(|_| {
                    ServiceError::crypto_error(format!(
                        "Failed to verify cFrag from holder {}",
                        cfrag_data.holder_id
                    ))
                })?;

            verified_cfrags.push(verified_cfrag);
        }

        let plaintext = umbral_pre::decrypt_reencrypted(
            &receiving_sk,
            &delegating_pk,
            &umbral_capsule,
            verified_cfrags,
            ciphertext,
        )
        .map_err(|_| ServiceError::crypto_error("PRE decryption failed"))?;

        Ok(plaintext.to_vec())
    }
}

impl Default for CryptoServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}
