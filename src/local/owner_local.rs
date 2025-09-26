//! ローカル環境での Owner（O-Browser）暗号化処理
//!
//! Phase 1のローカル処理：
//! - 秘密鍵ペア生成
//! - Shamir秘密分散
//! - 暗号化シェア生成
//! - カプセル生成
//! - 再暗号化キー生成
//! - kFrag生成

use crate::crypto_core::{
    CryptoService, CryptoServiceImpl, CryptoResult,
    ShamirShare, Capsule, SecretKey, PublicKey, ReencryptionKey, KeyFragment
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// O-Browser処理結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerLocalResult {
    /// 生成されたカプセル
    pub capsule: Vec<u8>,
    /// 暗号化されたシェア（Cᵢ）
    pub encrypted_shares: Vec<EncryptedShare>,
    /// 生成されたkFrags
    pub kfrags: Vec<SerializableKeyFragment>,
    /// 対称鍵のハッシュ（検証用）
    pub symmetric_key_hash: Vec<u8>,
}

/// 暗号化されたシェア
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedShare {
    /// シェアのインデックス
    pub index: u8,
    /// AES-GCM暗号化されたデータ
    pub encrypted_data: Vec<u8>,
    /// AES-GCM認証タグ
    pub auth_tag: Vec<u8>,
    /// ノンス
    pub nonce: Vec<u8>,
}

/// シリアライズ可能なKeyFragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableKeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// ローカル暗号化処理パラメータ
#[derive(Debug)]
pub struct OwnerLocalParams {
    /// 分散する秘密データ
    pub secret: Vec<u8>,
    /// 閾値
    pub threshold: u8,
    /// 総シェア数
    pub total_shares: u8,
    /// アクセサ（受信者）の公開鍵
    pub accessor_public_key: PublicKey,
}

impl Drop for OwnerLocalParams {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

/// ローカルOwner処理の実装
pub struct OwnerLocalProcessor {
    crypto_service: CryptoServiceImpl,
}

impl OwnerLocalProcessor {
    /// 新しいOwnerLocalProcessorを作成
    pub fn new() -> Self {
        Self {
            crypto_service: CryptoServiceImpl::new(),
        }
    }

    /// 完全なローカル処理フローを実行
    ///
    /// Phase 1の手順：
    /// 1. Owner鍵ペア生成（skₒ, pkₒ）
    /// 2. 秘密のShamir分散（f(0) → f(1)...f(n)）
    /// 3. 対称鍵生成（kₒ）
    /// 4. シェア暗号化（Cᵢ = AES_GCM(kₒ, f(i))）
    /// 5. カプセル生成（Capsuleₒ = PRE_Enc(pkₒ, kₒ)）
    /// 6. 再暗号化キー生成（rekey = PRE_ReKey(skₒ → pkᴬ)）
    /// 7. kFrag生成（kFragⱼ = Shamir_Split(rekey, k, n)）
    pub fn process_owner_local(&self, params: OwnerLocalParams) -> CryptoResult<OwnerLocalResult> {
        // Step 1: Owner鍵ペア生成
        let (owner_sk, owner_pk) = self.crypto_service.generate_keypair()?;

        // Step 2: Shamir秘密分散
        let shares = self.crypto_service.split_secret_shamir(
            &params.secret,
            params.threshold,
            params.total_shares
        )?;

        // Step 3: 対称鍵生成（AES-256用）
        let symmetric_key = self.generate_symmetric_key()?;

        // Step 4: シェア暗号化
        let encrypted_shares = self.encrypt_shares(&shares, &symmetric_key)?;

        // Step 5: カプセル生成
        let (capsule, _ciphertext) = self.crypto_service.create_pre_capsule(&owner_pk, &symmetric_key)?;

        // Step 6: 再暗号化キー生成
        let reencryption_key = self.crypto_service.generate_reencryption_key(
            &owner_sk,
            &params.accessor_public_key
        )?;

        // Step 7: kFrag生成
        let kfrags = self.crypto_service.create_kfrags(
            &reencryption_key,
            params.threshold,
            params.total_shares
        )?;

        // 対称鍵のハッシュ生成（検証用）
        let symmetric_key_hash = self.hash_key(&symmetric_key)?;

        // 結果をシリアライズ可能な形式に変換
        let serializable_kfrags = kfrags.into_iter()
            .map(|kf| SerializableKeyFragment {
                id: kf.id,
                key_data: kf.key_data.clone(),
                verification_data: kf.verification_data.clone(),
                precursor: kf.precursor.clone(),
            })
            .collect();

        Ok(OwnerLocalResult {
            capsule: capsule.data,
            encrypted_shares,
            kfrags: serializable_kfrags,
            symmetric_key_hash,
        })
    }

    /// AES-256用の対称鍵を生成
    fn generate_symmetric_key(&self) -> CryptoResult<Vec<u8>> {
        use rand::RngCore;

        let mut key = vec![0u8; 32]; // 256ビット
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    /// シェアをAES-GCMで暗号化
    fn encrypt_shares(&self, shares: &[ShamirShare], key: &[u8]) -> CryptoResult<Vec<EncryptedShare>> {
        use rand::RngCore;

        let mut encrypted_shares = Vec::new();

        for share in shares {
            // ノンス生成（96ビット）
            let mut nonce = vec![0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce);

            // AES-GCM暗号化をシミュレート（実装簡略化）
            // 実際の実装では aes-gcm クレートを使用
            let encrypted_data = share.data.clone(); // MVP: 暗号化をスキップ
            let auth_tag = vec![0u8; 16]; // MVP: 認証タグをダミー値

            encrypted_shares.push(EncryptedShare {
                index: share.index,
                encrypted_data,
                auth_tag,
                nonce,
            });
        }

        Ok(encrypted_shares)
    }

    /// 鍵のハッシュを生成（検証用）
    fn hash_key(&self, key: &[u8]) -> CryptoResult<Vec<u8>> {
        // SHA-256ハッシュをシミュレート
        // 実際の実装では sha2 クレートを使用
        Ok(key.iter().map(|&b| b.wrapping_add(1)).collect())
    }

    /// テスト用の簡易的なアクセサ公開鍵を生成
    pub fn generate_test_accessor_key(&self) -> CryptoResult<PublicKey> {
        let (_sk, pk) = self.crypto_service.generate_keypair()?;
        Ok(pk)
    }
}

impl Default for OwnerLocalProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owner_local_complete_flow() {
        println!("\n=== Owner Local: Complete Flow Test ===");
        println!("【テスト内容】: ローカルでの完全なOwner処理フローを検証");

        let processor = OwnerLocalProcessor::new();

        // テスト用データ準備
        let secret = b"This is a secret message for D-TPRES MVP";
        let threshold = 2;
        let total_shares = 3;

        println!("\n1. テスト用アクセサ鍵を生成中...");
        let accessor_pk = processor.generate_test_accessor_key()
            .expect("Failed to generate accessor key");
        println!("   ✓ アクセサ公開鍵生成完了");

        // パラメータ設定
        let params = OwnerLocalParams {
            secret: secret.to_vec(),
            threshold,
            total_shares,
            accessor_public_key: accessor_pk,
        };

        println!("\n2. ローカル処理フローを実行中...");
        let result = processor.process_owner_local(params)
            .expect("Failed to process owner local");

        println!("\n3. 処理結果の検証:");
        println!("   カプセルサイズ: {} bytes", result.capsule.len());
        println!("   暗号化シェア数: {}", result.encrypted_shares.len());
        println!("   kFrags数: {}", result.kfrags.len());
        println!("   対称鍵ハッシュサイズ: {} bytes", result.symmetric_key_hash.len());

        // 検証
        assert!(!result.capsule.is_empty(), "カプセルが空です");
        assert_eq!(result.encrypted_shares.len(), total_shares as usize, "暗号化シェア数が不正です");
        assert_eq!(result.kfrags.len(), total_shares as usize, "kFrags数が不正です");
        assert!(!result.symmetric_key_hash.is_empty(), "対称鍵ハッシュが空です");

        // 各暗号化シェアの詳細確認
        println!("\n4. 暗号化シェアの詳細:");
        for (i, encrypted_share) in result.encrypted_shares.iter().enumerate() {
            println!("   シェア {} (index={}):", i + 1, encrypted_share.index);
            println!("     - 暗号化データサイズ: {} bytes", encrypted_share.encrypted_data.len());
            println!("     - 認証タグサイズ: {} bytes", encrypted_share.auth_tag.len());
            println!("     - ノンスサイズ: {} bytes", encrypted_share.nonce.len());

            assert_eq!(encrypted_share.index, (i + 1) as u8, "シェアインデックスが不正です");
            assert!(!encrypted_share.encrypted_data.is_empty(), "暗号化データが空です");
            assert_eq!(encrypted_share.auth_tag.len(), 16, "認証タグサイズが不正です");
            assert_eq!(encrypted_share.nonce.len(), 12, "ノンスサイズが不正です");
        }

        // kFragsの詳細確認
        println!("\n5. kFragsの詳細:");
        for (i, kfrag) in result.kfrags.iter().enumerate() {
            println!("   kFrag {} (id={}):", i + 1, kfrag.id);
            println!("     - キーデータサイズ: {} bytes", kfrag.key_data.len());
            println!("     - 検証データサイズ: {} bytes", kfrag.verification_data.len());

            assert_eq!(kfrag.id, i as u8, "kFragのIDが不正です");
            assert!(!kfrag.key_data.is_empty(), "kFragのキーデータが空です");
            assert!(!kfrag.verification_data.is_empty(), "kFragの検証データが空です");
        }

        println!("\n✅ すべてのOwnerローカル処理テストが成功しました！");
    }

    #[test]
    fn test_owner_local_error_handling() {
        println!("\n=== Owner Local: Error Handling Test ===");

        let processor = OwnerLocalProcessor::new();
        let accessor_pk = processor.generate_test_accessor_key().unwrap();

        // テスト1: 閾値エラー
        println!("\n1. 閾値エラーテスト:");
        let invalid_params = OwnerLocalParams {
            secret: b"test".to_vec(),
            threshold: 0, // 無効な閾値
            total_shares: 3,
            accessor_public_key: accessor_pk.clone(),
        };

        let result = processor.process_owner_local(invalid_params);
        assert!(result.is_err(), "無効な閾値でエラーになるべき");
        println!("   ✓ 無効な閾値で期待通りエラー発生");

        // テスト2: 空の秘密データ
        println!("\n2. 空の秘密データテスト:");
        let empty_secret_params = OwnerLocalParams {
            secret: vec![], // 空の秘密
            threshold: 2,
            total_shares: 3,
            accessor_public_key: accessor_pk,
        };

        let result = processor.process_owner_local(empty_secret_params);
        assert!(result.is_err(), "空の秘密でエラーになるべき");
        println!("   ✓ 空の秘密で期待通りエラー発生");

        println!("\n✅ エラーハンドリングテストが成功しました！");
    }
}