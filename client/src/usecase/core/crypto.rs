//! CryptoService - 暗号化操作を担当する中核サービス
//!
//! Threshold Proxy Re-Encryption (TPRE) とShamir Secret Sharingの実装を提供します。
//! AO環境の制約に従い、すべての操作は同期的に実行されます。

use bincode;
use generic_array::{GenericArray, typenum::U32};
use shamirsecretsharing::{DATA_SIZE, combine_shares, create_shares};
use subtle::ConstantTimeEq;
use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::service::error::{ServiceError, ServiceResult};

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
}

/// Shamir Secret Sharingのシェア
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct ShamirShare {
    /// シェアのインデックス（1から開始）
    pub index: u8,
    /// シェアデータ（使用後にゼロ化される）
    pub data: Vec<u8>,
}

/// Umbral暗号化のカプセル（シリアライズされた不透明トークン）
#[non_exhaustive]
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Capsule {
    /// umbral_pre::Capsuleのシリアライズされた完全なデータ
    pub data: Vec<u8>,
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
        let result = umbral_pre::SecretKey::try_from_be_bytes(&secret_bytes)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize secret key"));

        // secret_bytesは自動的にZeroizeされる（SecretBoxのDrop実装により）
        result
    }

    /// PublicKeyからumbral_pre::PublicKeyを復元
    fn deserialize_public_key(&self, key: &PublicKey) -> ServiceResult<umbral_pre::PublicKey> {
        bincode::deserialize(&key.key_data)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize key"))
    }

    /// Capsuleからumbral_pre::Capsuleを復元
    fn deserialize_capsule(&self, capsule: &Capsule) -> ServiceResult<umbral_pre::Capsule> {
        bincode::deserialize(&capsule.data)
            .map_err(|_| ServiceError::crypto_error("Failed to deserialize capsule"))
    }
}

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
                data: share,
            });
        }

        Ok(result)
    }

    fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> ServiceResult<Vec<u8>> {
        // 入力検証
        if shares.len() < threshold as usize {
            return Err(ServiceError::validation_error(
                "Insufficient shares for reconstruction",
            ));
        }

        // ShamirShare型からVec<u8>へ変換（shamirsecretsharing::ShareはVec<u8>型）
        let share_vecs: Vec<Vec<u8>> = shares.iter().map(|s| s.data.as_slice().to_vec()).collect();

        // ライブラリを使用してシェアを結合
        let recovered = combine_shares(&share_vecs)
            .map_err(|_| ServiceError::crypto_error("Failed to combine shares"))?;

        // 復元されたデータを処理
        match recovered {
            Some(data) => {
                // パディングを除去して元のデータを取得
                if data.is_empty() {
                    return Err(ServiceError::crypto_error("Recovered data is empty"));
                }

                let original_len = data[0] as usize;
                if original_len == 0 || original_len > DATA_SIZE - 1 {
                    return Err(ServiceError::crypto_error("Invalid recovered data format"));
                }

                // 元のデータを抽出
                let result = data[1..original_len + 1].to_vec();
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
        let capsule_bytes: Vec<u8> = bincode::serialize(&umbral_capsule)
            .map_err(|_| ServiceError::crypto_error("Failed to serialize capsule"))?;

        let capsule: Capsule = Capsule {
            data: capsule_bytes,
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

        if capsule.data.is_empty() {
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

        if original_capsule.data.is_empty() {
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
}

impl Default for CryptoServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reencryption_key() {
        println!("\n=== CryptoService: Generate Re-encryption Key Test ===");
        println!("【テスト内容】: 再暗号化鍵生成機能を検証");
        println!("【期待結果】: 委任者の秘密鍵と受信者の公開鍵を保持する中間構造体が生成される");

        let service = CryptoServiceImpl::new();

        println!("\n1. 鍵ペアを生成中...");
        let (owner_sk, _owner_pk) = service
            .generate_keypair()
            .expect("Failed to generate owner keypair");
        let (_accessor_sk, accessor_pk) = service
            .generate_keypair()
            .expect("Failed to generate accessor keypair");

        println!("\n2. 再暗号化鍵を生成中...");
        let reencryption_key = service
            .generate_reencryption_key(&owner_sk, &accessor_pk)
            .expect("Failed to generate re-encryption key");

        println!("\n3. 生成された再暗号化鍵の詳細:");
        println!(
            "   委任者秘密鍵データサイズ: {} bytes",
            reencryption_key.delegating_sk_data.len()
        );
        println!(
            "   受信者公開鍵データサイズ: {} bytes",
            reencryption_key.receiving_pk_data.len()
        );

        assert_eq!(
            reencryption_key.delegating_sk_data.len(),
            constants::KEY_SIZE_BYTES
        );
        assert!(!reencryption_key.receiving_pk_data.is_empty());

        println!("\n✅ テスト成功: 再暗号化鍵が正常に生成されました！");
    }

    #[test]
    fn test_create_kfrags() {
        println!("\n=== CryptoService: Create kFrags Test ===");
        println!("【テスト内容】: kFrags生成機能を検証");
        println!("【期待結果】: 指定された数のkFragsが生成される");

        let service = CryptoServiceImpl::new();

        println!("\n1. 鍵ペアを生成中...");
        let (owner_sk, _owner_pk) = service
            .generate_keypair()
            .expect("Failed to generate owner keypair");
        let (_accessor_sk, accessor_pk) = service
            .generate_keypair()
            .expect("Failed to generate accessor keypair");

        println!("\n2. 再暗号化鍵を生成中...");
        let reencryption_key = service
            .generate_reencryption_key(&owner_sk, &accessor_pk)
            .expect("Failed to generate re-encryption key");

        println!("\n3. kFragsを生成中 (threshold=3, total=5)...");
        let kfrags = service
            .create_kfrags(&reencryption_key, 3, 5)
            .expect("Failed to create kFrags");

        println!("\n4. 生成されたkFragsの詳細:");
        assert_eq!(kfrags.len(), 5);
        println!("   生成されたkFrags数: {}", kfrags.len());

        for (i, kfrag) in kfrags.iter().enumerate() {
            println!("   kFrag {} (id={}):", i, kfrag.id);
            println!("     - サイズ: {} bytes", kfrag.key_data.len());
            assert!(!kfrag.key_data.is_empty());
        }

        println!("\n5. 閾値検証テスト:");

        // 無効な閾値（0）
        let result = service.create_kfrags(&reencryption_key, 0, 5);
        assert!(result.is_err());
        println!("   ✓ 閾値0で期待通りエラー");

        // 閾値が総数より大きい
        let result = service.create_kfrags(&reencryption_key, 6, 5);
        assert!(result.is_err());
        println!("   ✓ 閾値>総数で期待通りエラー");

        println!("\n✅ テスト成功: kFragsが正常に生成されました！");
    }

    #[test]
    fn test_shamir_split_and_reconstruct() {
        println!("\n=== CryptoService: Shamir Secret Sharing Complete Test ===");
        println!("【テスト内容】: Shamir秘密分散の分割と復元の完全な動作検証");
        println!("【テスト対象】: split_secret_shamir()とreconstruct_secret_shamir()メソッド");

        let service: CryptoServiceImpl = CryptoServiceImpl::new();
        let secret = b"This is a test secret for Shamir!";
        println!(
            "\nテスト用秘密データ: \"{}\" ({} bytes)",
            std::str::from_utf8(secret).unwrap(),
            secret.len()
        );

        // テストケース1: 無効な閾値（0）
        println!("\n📋 テストケース1: 閾値が0の場合");
        println!("   入力: threshold=0, total_shares=5");
        let result = service.split_secret_shamir(secret, 0, 5);
        match &result {
            Err(e) => println!("   期待通りエラー発生: {}", e),
            Ok(_) => panic!("エラーが発生すべきところで成功してしまった"),
        }
        assert!(result.is_err());

        // テストケース2: 閾値が総数より大きい
        println!("\n📋 テストケース2: 閾値が総シェア数より大きい場合");
        println!("   入力: threshold=6, total_shares=5");
        let result = service.split_secret_shamir(secret, 6, 5);
        match &result {
            Err(e) => println!("   期待通りエラー発生: {}", e),
            Ok(_) => panic!("エラーが発生すべきところで成功してしまった"),
        }
        assert!(result.is_err());

        // テストケース3: 空の秘密データ
        println!("\n📋 テストケース3: 秘密データが空の場合");
        println!("   入力: secret=[], threshold=3, total_shares=5");
        let result = service.split_secret_shamir(b"", 3, 5);
        match &result {
            Err(e) => println!("   期待通りエラー発生: {}", e),
            Ok(_) => panic!("エラーが発生すべきところで成功してしまった"),
        }
        assert!(result.is_err());

        // 正常系のテスト: 分割と復元
        println!("\n📋 テストケース4: 正常な分割と復元");
        println!("   入力: threshold=3, total_shares=10");
        let result = service.split_secret_shamir(secret, 3, 10);
        assert!(result.is_ok());

        let shares: Vec<ShamirShare> = result.unwrap();
        println!("   ✓ 成功: {}個のシェアを生成", shares.len());
        assert_eq!(shares.len(), 10);

        // シェアの詳細を表示
        println!("\n   シェアの詳細:");
        for (i, share) in shares.iter().enumerate() {
            println!("   Share {} (index={}):", i + 1, share.index);
            println!("     - サイズ: {} bytes", share.data.len());
            // 最初の16バイトだけを表示
            print!("     - データ先頭 (16進数): ");
            for byte in share.data.iter().take(16) {
                print!("{:02x} ", byte);
            }
            println!("...");
        }

        // 最小限のシェア(閾値と同じ数)で復元
        println!("\n📋 テストケース5: 最小限のシェアで復元 (3/5)");
        let min_shares = &shares[0..4];
        let reconstruct_result = service.reconstruct_secret_shamir(min_shares, 4);
        assert!(reconstruct_result.is_ok());

        let recovered = reconstruct_result.unwrap();
        println!("   ✓ 復元成功");
        println!(
            "   復元データ: \"{}\" ({} bytes)",
            std::str::from_utf8(&recovered).unwrap(),
            recovered.len()
        );
        assert_eq!(recovered, secret);
        println!("   ✓ 元の秘密と完全一致！");

        // 異なるシェアの組み合わせで復元
        println!("\n📋 テストケース6: 異なるシェアの組み合わせで復元 (shares 2,3,5)");
        let different_shares = vec![shares[1].clone(), shares[2].clone(), shares[4].clone()];
        let reconstruct_result2 = service.reconstruct_secret_shamir(&different_shares, 3);
        assert!(reconstruct_result2.is_ok());

        let recovered2 = reconstruct_result2.unwrap();
        assert_eq!(recovered2, secret);
        println!("   ✓ 異なる組み合わせでも復元成功！");

        // 不十分なシェアでの復元試行
        println!("\n📋 テストケース7: 不十分なシェアでの復元試行 (2/3)");
        let insufficient_shares = &shares[0..2];
        let reconstruct_fail = service.reconstruct_secret_shamir(insufficient_shares, 3);
        match &reconstruct_fail {
            Err(e) => println!("   期待通りエラー発生: {}", e),
            Ok(_) => panic!("エラーが発生すべきところで成功してしまった"),
        }
        assert!(reconstruct_fail.is_err());

        println!("\n✅ すべてのShamir Secret Sharingテストが成功しました！");
    }

    #[test]
    fn test_keypair_generation() {
        println!("\n=== CryptoService: Keypair Generation Test ===");
        println!("【テスト内容】: Umbral PRE用の鍵ペア生成機能を検証");
        println!("【期待結果】: 秘密鍵32バイト、公開鍵33バイトが生成される");

        let service = CryptoServiceImpl::new();

        println!("\n1. 鍵ペアを生成中...");
        let result = service.generate_keypair();
        assert!(result.is_ok());

        let (secret_key, public_key) = result.unwrap();

        println!("\n2. 生成された鍵の詳細:");
        println!("   秘密鍵 (プレースホルダー実装):");
        println!("     - サイズ: {} bytes", secret_key.key_data.len());
        println!("     - 完全なデータ (16進数):");
        for (i, chunk) in secret_key.key_data.chunks(16).enumerate() {
            print!("       {:04x}: ", i * 16);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            println!();
        }
        println!("     - 注: 実際の実装では32バイトのランダムな秘密鍵が生成されます");

        println!("\n   公開鍵 (プレースホルダー実装):");
        println!("     - サイズ: {} bytes", public_key.key_data.len());
        println!("     - 完全なデータ (16進数):");
        for (i, chunk) in public_key.key_data.chunks(16).enumerate() {
            print!("       {:04x}: ", i * 16);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            println!();
        }
        println!("     - 注: 実際の実装では楕円曲線上の点(圧縮形式)が生成されます");

        println!("\n3. 検証:");
        assert!(!secret_key.key_data.is_empty());
        assert!(!public_key.key_data.is_empty());
        println!("   ✓ 秘密鍵が空でない");
        println!("   ✓ 公開鍵が空でない");

        println!("\n✅ テスト成功: 鍵ペアが正常に生成されました！");
    }

    #[test]
    fn test_proxy_reencrypt_full_flow() {
        println!("\n=== CryptoService: Full Proxy Re-encryption Flow Test ===");
        println!("【テスト内容】: 完全な暗号化→再暗号化→復号フローを検証");
        println!("【期待結果】: 元の平文が正しく復元される");

        let service = CryptoServiceImpl::new();

        // Step 1: 鍵ペアの生成
        println!("\n1. 鍵ペアを生成中...");
        let (alice_sk, alice_pk) = service
            .generate_keypair()
            .expect("Failed to generate Alice keypair");
        let (bob_sk, bob_pk) = service
            .generate_keypair()
            .expect("Failed to generate Bob keypair");

        println!("   ✓ Alice（委任者）の鍵ペア生成完了");
        println!("   ✓ Bob（受信者）の鍵ペア生成完了");

        // Step 2: Aliceが平文を暗号化
        println!("\n2. Aliceが平文を暗号化中...");
        let plaintext = b"Secret message for proxy re-encryption test";
        let (capsule, ciphertext) = service
            .create_pre_capsule(&alice_pk, plaintext)
            .expect("Failed to create capsule");

        println!("   平文: \"{}\"", std::str::from_utf8(plaintext).unwrap());
        println!("   カプセルサイズ: {} bytes", capsule.data.len());
        println!("   暗号文サイズ: {} bytes", ciphertext.len());

        // Step 3: AliceがBobへの再暗号化鍵を生成
        println!("\n3. AliceがBobへの再暗号化鍵を生成中...");
        let reencryption_key = service
            .generate_reencryption_key(&alice_sk, &bob_pk)
            .expect("Failed to generate re-encryption key");
        println!("   ✓ 再暗号化鍵生成完了");

        // Step 4: 再暗号化鍵からkFragsを生成
        println!("\n4. kFragsを生成中 (threshold=2, total=3)...");
        let kfrags = service
            .create_kfrags(&reencryption_key, 2, 3)
            .expect("Failed to create kFrags");
        println!("   ✓ {}個のkFrags生成完了", kfrags.len());

        // Step 5: プロキシがkFragsを使用して再暗号化
        println!("\n5. プロキシが再暗号化を実行中...");
        let mut cfrags = Vec::new();
        for (i, kfrag) in kfrags.iter().take(2).enumerate() {
            // 閾値分だけ使用
            let cfrag = service
                .proxy_reencrypt(kfrag, &capsule)
                .expect("Failed to re-encrypt");
            println!(
                "   ✓ cFrag {} 生成完了 (サイズ: {} bytes)",
                i,
                cfrag.capsule_fragment.len()
            );
            cfrags.push(cfrag);
        }

        // Step 6: Bobが再暗号化されたデータを復号
        println!("\n6. Bobが再暗号化されたデータを復号中...");

        // combine_and_decryptを呼び出して復号
        let decrypted = service
            .combine_and_decrypt(&cfrags, &bob_sk, &capsule, &ciphertext)
            .expect("Failed to decrypt reencrypted data");

        println!("   ✓ 復号成功");
        println!(
            "   復号データ: \"{}\"",
            std::str::from_utf8(&decrypted).unwrap()
        );

        // 元の平文と一致することを確認
        assert_eq!(
            plaintext,
            &decrypted[..],
            "Decrypted data should match original plaintext"
        );
        println!("   ✓ 元の平文と完全に一致！");

        // テスト完了
        println!("\n✅ 完全な再暗号化フローのエンドツーエンドテストが成功しました！");
    }

    #[test]
    fn test_proxy_reencrypt() {
        println!("\n=== CryptoService: Proxy Re-encrypt Test ===");
        println!("【テスト内容】: proxy_reencrypt機能を検証");
        println!("【期待結果】: kFragとCapsuleからcFragが生成される");

        let service = CryptoServiceImpl::new();

        // 鍵ペアとカプセルの準備
        let (alice_sk, alice_pk) = service
            .generate_keypair()
            .expect("Failed to generate Alice keypair");
        let (_bob_sk, bob_pk) = service
            .generate_keypair()
            .expect("Failed to generate Bob keypair");

        // カプセルの生成
        let plaintext = b"Test message";
        let (capsule, _ciphertext) = service
            .create_pre_capsule(&alice_pk, plaintext)
            .expect("Failed to create capsule");

        // kFragsの生成
        let reencryption_key = service
            .generate_reencryption_key(&alice_sk, &bob_pk)
            .expect("Failed to generate re-encryption key");
        let kfrags = service
            .create_kfrags(&reencryption_key, 2, 2)
            .expect("Failed to create kFrags");

        println!("\n1. proxy_reencryptを実行中...");
        let cfrag = service
            .proxy_reencrypt(&kfrags[0], &capsule)
            .expect("Failed to re-encrypt");

        println!("\n2. 生成されたcFragの詳細:");
        println!("   Fragment ID: {}", cfrag.fragment_id);
        println!(
            "   Capsule Fragment サイズ: {} bytes",
            cfrag.capsule_fragment.len()
        );
        assert!(!cfrag.capsule_fragment.is_empty());

        println!("\n✅ テスト成功: proxy_reencryptが正常に動作しました！");
    }

    #[test]
    fn test_combine_and_decrypt() {
        println!("\n=== CryptoService: Combine and Decrypt Test ===");
        println!("【テスト内容】: combine_and_decrypt機能の包括的な検証");
        println!("【テスト項目】:");
        println!("  - 正常な復号処理");
        println!("  - エラーハンドリング");
        println!("  - 閾値検証");

        let service = CryptoServiceImpl::new();

        // テスト準備: 完全な再暗号化フローの設定
        println!("\n1. テスト環境の準備...");

        // 鍵ペアの生成
        let (alice_sk, alice_pk) = service
            .generate_keypair()
            .expect("Failed to generate Alice keypair");
        let (bob_sk, bob_pk) = service
            .generate_keypair()
            .expect("Failed to generate Bob keypair");
        println!("   ✓ 鍵ペア生成完了");

        // 平文の暗号化
        let plaintext = b"Test data for combine_and_decrypt";
        let (capsule, ciphertext) = service
            .create_pre_capsule(&alice_pk, plaintext)
            .expect("Failed to create capsule");
        println!("   ✓ 暗号化完了");

        // kFragsの生成（閾値2、総数3）
        let reencryption_key = service
            .generate_reencryption_key(&alice_sk, &bob_pk)
            .expect("Failed to generate re-encryption key");
        let kfrags = service
            .create_kfrags(&reencryption_key, 2, 3)
            .expect("Failed to create kFrags");
        println!("   ✓ kFrags生成完了 (threshold=2, total=3)");

        // cFragsの生成
        let mut cfrags = Vec::new();
        for kfrag in kfrags.iter().take(2) {
            let cfrag = service
                .proxy_reencrypt(kfrag, &capsule)
                .expect("Failed to re-encrypt");
            cfrags.push(cfrag);
        }
        println!("   ✓ cFrags生成完了 (2個)");

        // テスト1: 正常な復号
        println!("\n2. 正常な復号テスト...");
        let decrypted = service
            .combine_and_decrypt(&cfrags, &bob_sk, &capsule, &ciphertext)
            .expect("Failed to decrypt");

        assert_eq!(plaintext, &decrypted[..]);
        println!("   ✓ 復号成功: 元の平文と一致");

        // テスト2: 空のcFragsでエラー
        println!("\n3. エラーハンドリングテスト...");
        println!("   3-1. 空のcFrags:");
        let result = service.combine_and_decrypt(&[], &bob_sk, &capsule, &ciphertext);
        assert!(result.is_err());
        println!("      ✓ 期待通りエラー発生");

        // テスト3: 不正な秘密鍵でエラー
        println!("   3-2. 不正な秘密鍵:");
        let invalid_sk = SecretKey {
            key_data: vec![0u8; 32], // ダミーの秘密鍵
        };
        let result = service.combine_and_decrypt(&cfrags, &invalid_sk, &capsule, &ciphertext);
        assert!(result.is_err());
        println!("      ✓ 期待通りエラー発生");

        // テスト4: 不正なカプセルでエラー
        println!("   3-3. 不正なカプセル:");
        let invalid_capsule = Capsule {
            data: vec![0u8; 100], // ダミーのカプセル
        };
        let result = service.combine_and_decrypt(&cfrags, &bob_sk, &invalid_capsule, &ciphertext);
        assert!(result.is_err());
        println!("      ✓ 期待通りエラー発生");

        // テスト5: 空の暗号文でエラー
        println!("   3-4. 空の暗号文:");
        let result = service.combine_and_decrypt(&cfrags, &bob_sk, &capsule, &[]);
        assert!(result.is_err());
        println!("      ✓ 期待通りエラー発生");

        // テスト6: 閾値以下のcFragsで復号（1個だけ使用）
        println!("\n4. 閾値検証テスト...");
        println!("   閾値未満のcFrags (1/2):");
        let single_cfrag = vec![cfrags[0].clone()];
        let result = service.combine_and_decrypt(&single_cfrag, &bob_sk, &capsule, &ciphertext);
        // umbral-preの実装によっては、閾値未満でもエラーにならない場合がある
        // （実際のエラーは復号時に発生）
        if result.is_err() {
            println!("      ✓ 閾値未満でエラー発生");
        } else {
            println!("      ⚠ 閾値未満でも処理が進行（復号結果の検証が必要）");
            // 復号結果が正しくないことを確認
            if let Ok(decrypted) = result {
                assert_ne!(
                    plaintext,
                    &decrypted[..],
                    "閾値未満では正しく復号できないはず"
                );
                println!("      ✓ 復号結果は不正（期待通り）");
            }
        }

        // テスト7: 3個すべてのcFragsを使用（閾値以上）
        println!("\n5. 閾値以上のcFragsテスト...");
        let mut all_cfrags = cfrags.clone();
        // 3個目のcFragを生成
        let third_cfrag = service
            .proxy_reencrypt(&kfrags[2], &capsule)
            .expect("Failed to re-encrypt third kFrag");
        all_cfrags.push(third_cfrag);

        let decrypted = service
            .combine_and_decrypt(&all_cfrags, &bob_sk, &capsule, &ciphertext)
            .expect("Failed to decrypt with all cFrags");

        assert_eq!(plaintext, &decrypted[..]);
        println!("   ✓ 3個のcFragsでも正常に復号");

        println!("\n✅ すべてのcombine_and_decryptテストが成功しました！");
    }

    #[test]
    fn test_create_pre_capsule() {
        println!("\n=== CryptoService: Create PRE Capsule Test ===");
        println!("【テスト内容】: Umbral PRE暗号化カプセル生成機能を検証");
        println!("【テスト項目】:");
        println!("  - 正常な暗号化処理");
        println!("  - エラーハンドリング");
        println!("  - ランダム性の検証");
        println!("  - 大容量データの暗号化");

        let service = CryptoServiceImpl::new();

        // テスト1: 正常な暗号化
        println!("\n1. 正常な暗号化テスト:");
        println!("   鍵ペアを生成中...");
        let (_owner_sk, owner_pk) = service
            .generate_keypair()
            .expect("Failed to generate keypair");

        let plaintext = b"Test message for PRE encryption";
        println!("   平文: {:?}", std::str::from_utf8(plaintext).unwrap());

        println!("   暗号化を実行中...");
        let (capsule, ciphertext) = service
            .create_pre_capsule(&owner_pk, plaintext)
            .expect("Failed to create capsule");

        println!("   ✓ カプセル生成成功");
        println!("   ✓ カプセルサイズ: {} bytes", capsule.data.len());
        println!("   ✓ 暗号文サイズ: {} bytes", ciphertext.len());

        // カプセルデータの詳細ログ出力
        println!("\n   カプセルデータの詳細 (16進数):");
        for (i, chunk) in capsule.data.chunks(16).enumerate() {
            print!("     {:04x}: ", i * 16);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            // ASCII表示 (印字可能文字のみ)
            print!("  |");
            for byte in chunk {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    print!("{}", *byte as char);
                } else {
                    print!(".");
                }
            }
            println!("|");
        }

        // 暗号文の先頭部分も表示
        println!("\n   暗号文データの先頭32バイト (16進数):");
        for (i, chunk) in ciphertext
            .iter()
            .take(32)
            .collect::<Vec<_>>()
            .chunks(16)
            .enumerate()
        {
            print!("     {:04x}: ", i * 16);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            println!();
        }

        assert!(!capsule.data.is_empty(), "Capsule should not be empty");
        assert!(!ciphertext.is_empty(), "Ciphertext should not be empty");
        assert_ne!(
            plaintext,
            &ciphertext[..plaintext.len().min(ciphertext.len())],
            "Ciphertext should differ from plaintext"
        );

        // テスト2: 無効な公開鍵でのエラーハンドリング
        println!("\n2. エラーハンドリングテスト (無効な公開鍵):");
        let invalid_pk = PublicKey {
            key_data: vec![0u8; 10],
        }; // 不正なサイズ

        println!("   無効な公開鍵で暗号化を試行...");
        let result = service.create_pre_capsule(&invalid_pk, plaintext);
        assert!(result.is_err(), "Should fail with invalid public key");
        println!("   ✓ 期待通りエラーが発生");

        if let Err(e) = result {
            println!("   エラー内容: {:?}", e);
        }

        // テスト3: 空の平文でのエラーハンドリング
        println!("\n3. エラーハンドリングテスト (空の平文):");
        let empty_plaintext = b"";

        println!("   空の平文で暗号化を試行...");
        let result = service.create_pre_capsule(&owner_pk, empty_plaintext);
        assert!(result.is_err(), "Should fail with empty plaintext");
        println!("   ✓ 期待通りエラーが発生");

        // テスト4: ランダム性の検証（同じ平文でも異なる暗号文）
        println!("\n4. ランダム性の検証:");
        println!("   同じ平文を2回暗号化...");

        let (capsule1, ciphertext1) = service
            .create_pre_capsule(&owner_pk, plaintext)
            .expect("Failed to create first capsule");
        let (capsule2, ciphertext2) = service
            .create_pre_capsule(&owner_pk, plaintext)
            .expect("Failed to create second capsule");

        assert_ne!(
            capsule1.data, capsule2.data,
            "Capsules should be different due to randomness"
        );

        // ランダム性を視覚的に確認
        println!("\n   カプセル1の先頭16バイト:");
        print!("     ");
        for byte in capsule1.data.iter().take(16) {
            print!("{:02x} ", byte);
        }
        println!();

        println!("   カプセル2の先頭16バイト:");
        print!("     ");
        for byte in capsule2.data.iter().take(16) {
            print!("{:02x} ", byte);
        }
        println!();
        assert_ne!(
            ciphertext1, ciphertext2,
            "Ciphertexts should be different due to randomness"
        );

        println!("   ✓ カプセル1 != カプセル2");
        println!("   ✓ 暗号文1 != 暗号文2");
        println!("   ✓ ランダム性が確認されました");

        // テスト5: 大きなデータの暗号化
        println!("\n5. 大容量データの暗号化テスト:");
        let large_data = vec![0x42u8; 1024]; // 1KB of data
        println!("   1KBのデータを暗号化中...");

        let (large_capsule, large_ciphertext) = service
            .create_pre_capsule(&owner_pk, &large_data)
            .expect("Failed to encrypt large data");

        println!("   ✓ 大容量データの暗号化成功");
        println!("   ✓ カプセルサイズ: {} bytes", large_capsule.data.len());
        println!("   ✓ 暗号文サイズ: {} bytes", large_ciphertext.len());

        // 大容量データでもカプセルサイズが一定であることを確認
        println!("\n   大容量データ用カプセルのサイズ確認:");
        println!("     通常データ用カプセル: {} bytes", capsule.data.len());
        println!(
            "     大容量データ用カプセル: {} bytes",
            large_capsule.data.len()
        );
        assert_eq!(
            capsule.data.len(),
            large_capsule.data.len(),
            "Capsule size should be constant regardless of plaintext size"
        );

        assert!(!large_capsule.data.is_empty());
        // 暗号化にはオーバーヘッドがあるため、暗号文は平文より大きくなる
        assert!(
            large_ciphertext.len() >= large_data.len(),
            "Ciphertext should be at least as long as plaintext"
        );
        println!(
            "   ✓ 暗号文オーバーヘッド: {} bytes",
            large_ciphertext.len() - large_data.len()
        );

        println!("\n✅ すべてのcreate_pre_capsuleテストが成功しました！");
    }
}
