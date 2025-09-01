//! CryptoService - 暗号化操作を担当する中核サービス
//!
//! Threshold Proxy Re-Encryption (TPRE) とShamir Secret Sharingの実装を提供します。
//! AO環境の制約に従い、すべての操作は同期的に実行されます。

use bincode;
use generic_array::{GenericArray, typenum::U32};
use shamirsecretsharing::{DATA_SIZE, combine_shares, create_shares};
use subtle::ConstantTimeEq;
use umbral_pre;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::service::error::{ServiceError, ServiceResult};

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
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct ShamirShare {
    /// シェアのインデックス（1から開始）
    pub index: u8,
    /// シェアデータ（使用後にゼロ化される）
    pub data: Vec<u8>,
}

/// Umbral暗号化のカプセル（シリアライズされた不透明トークン）
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Capsule {
    /// umbral_pre::Capsuleのシリアライズされた完全なデータ
    pub data: Vec<u8>,
}

/// 公開鍵
#[derive(Debug, Clone)]
pub struct PublicKey {
    pub key_data: Vec<u8>,
}

/// 秘密鍵（使用後にゼロ化される）
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    key_data: Vec<u8>,
}

/// 再暗号化鍵
/// 
/// 実際にはkFrags生成に必要な情報を保持する中間構造体
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct ReencryptionKey {
    /// 委任者の秘密鍵（シリアライズ済み）
    delegating_sk_data: Vec<u8>,
    /// 受信者の公開鍵（シリアライズ済み）
    receiving_pk_data: Vec<u8>,
}

/// 鍵フラグメント（kFrag）
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct KeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// 暗号フラグメント（cFrag）
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
}

impl CryptoServiceImpl {
    /// 新しいCryptoServiceインスタンスを作成
    pub fn new() -> Self {
        // 署名用の鍵を生成
        let signing_key: umbral_pre::SecretKey = umbral_pre::SecretKey::random();
        let signer: umbral_pre::Signer = umbral_pre::Signer::new(signing_key);

        Self { signer }
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
    #[inline]
    fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.ct_eq(b).into()
    }

    /// SecretKeyからumbral_pre::SecretKeyを復元
    fn deserialize_secret_key(&self, key: &SecretKey) -> ServiceResult<umbral_pre::SecretKey> {
        // key_dataをGenericArray<u8, U32>に変換
        if key.key_data.len() != constants::KEY_SIZE_BYTES {
            return Err(ServiceError::crypto_error(
                format!("Invalid secret key size: expected {}, got {}", 
                    constants::KEY_SIZE_BYTES, key.key_data.len())
            ));
        }
        
        // バイト配列からGenericArrayを作成してSecretBoxにラップ
        // SecretBoxのコンストラクタはprivateなので、一時的な秘密鍵を生成して
        // そのバイト表現を使う方法を採用
        let temp_sk = umbral_pre::SecretKey::random();
        let mut secret_bytes = temp_sk.to_be_bytes();
        
        // 実際のデータをコピー
        secret_bytes.as_mut_secret().copy_from_slice(&key.key_data);
        
        // SecretKeyに変換
        umbral_pre::SecretKey::try_from_be_bytes(&secret_bytes)
            .map_err(|e| ServiceError::crypto_error(format!("Failed to deserialize secret key: {}", e)))
    }

    /// PublicKeyからumbral_pre::PublicKeyを復元
    fn deserialize_public_key(&self, key: &PublicKey) -> ServiceResult<umbral_pre::PublicKey> {
        bincode::deserialize(&key.key_data).map_err(|e| {
            ServiceError::crypto_error(format!("Failed to deserialize public key: {}", e))
        })
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
            return Err(ServiceError::validation_error(format!(
                "Secret too large: {} bytes (max: {} bytes)",
                secret.len(),
                DATA_SIZE - 1
            )));
        }

        // ライブラリを使用してシェアを作成
        let shares: Vec<Vec<u8>> = create_shares(&padded_secret, total_shares, threshold)
            .map_err(|e| ServiceError::crypto_error(format!("Failed to create shares: {:?}", e)))?;

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
            return Err(ServiceError::validation_error(format!(
                "Insufficient shares: {} (required: {})",
                shares.len(),
                threshold
            )));
        }

        // ShamirShare型からVec<u8>へ変換（shamirsecretsharing::ShareはVec<u8>型）
        let share_vecs: Vec<Vec<u8>> = shares.iter().map(|s| s.data.clone()).collect();

        // ライブラリを使用してシェアを結合
        let recovered = combine_shares(&share_vecs).map_err(|e| {
            ServiceError::crypto_error(format!("Failed to combine shares: {:?}", e))
        })?;

        // 復元されたデータを処理
        match recovered {
            Some(data) => {
                // パディングを除去して元のデータを取得
                if data.is_empty() {
                    return Err(ServiceError::crypto_error("Recovered data is empty"));
                }

                let original_len = data[0] as usize;
                if original_len == 0 || original_len > DATA_SIZE - 1 {
                    return Err(ServiceError::crypto_error(format!(
                        "Invalid recovered data length: {}",
                        original_len
                    )));
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
            .map_err(|e| ServiceError::crypto_error(format!("Encryption failed: {}", e)))?;

        // Capsuleをシリアライズして保存
        let capsule_bytes: Vec<u8> = bincode::serialize(&umbral_capsule).map_err(|e| {
            ServiceError::crypto_error(format!("Failed to serialize capsule: {}", e))
        })?;

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
            delegating_sk_data: owner_secret_key.key_data.clone(),
            receiving_pk_data: accessor_public_key.key_data.clone(),
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

        if reencryption_key.delegating_sk_data.is_empty() || 
           reencryption_key.receiving_pk_data.is_empty() {
            return Err(ServiceError::validation_error("Invalid reencryption key"));
        }

        // 委任者の秘密鍵をデシリアライズ
        let delegating_sk = SecretKey {
            key_data: reencryption_key.delegating_sk_data.clone(),
        };
        let umbral_delegating_sk = self.deserialize_secret_key(&delegating_sk)?;
        
        // 受信者の公開鍵をデシリアライズ
        let receiving_pk = PublicKey {
            key_data: reencryption_key.receiving_pk_data.clone(),
        };
        let umbral_receiving_pk = self.deserialize_public_key(&receiving_pk)?;
        
        // kFragsを生成
        let verified_kfrags = umbral_pre::generate_kfrags(
            &umbral_delegating_sk,
            &umbral_receiving_pk,
            &self.signer,
            threshold as usize,
            total_fragments as usize,
            true,  // sign_delegating_key
            true,  // sign_receiving_key
        );
        
        // VerifiedKeyFragをKeyFragmentに変換
        let mut kfrags = Vec::with_capacity(verified_kfrags.len());
        for (index, verified_kfrag) in verified_kfrags.iter().enumerate() {
            // VerifiedKeyFragをシリアライズ
            let kfrag_bytes = bincode::serialize(verified_kfrag)
                .map_err(|e| ServiceError::crypto_error(
                    format!("Failed to serialize kFrag: {}", e)
                ))?;
            
            kfrags.push(KeyFragment {
                id: index as u8,
                key_data: kfrag_bytes,
                precursor: vec![],  // precursorは現時点では使用しない
            });
        }

        Ok(kfrags)
    }

    fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        _capsule: &Capsule,
    ) -> ServiceResult<CipherFragment> {
        // 入力検証
        if kfrag.key_data.is_empty() {
            return Err(ServiceError::validation_error("Invalid key fragment"));
        }

        // TODO: 実際のUmbral実装を使用
        // 現在はプレースホルダー実装
        Ok(CipherFragment {
            fragment_id: kfrag.id,
            capsule_fragment: vec![0u8; 33], // プレースホルダー
            proof: vec![0u8; 96],            // プレースホルダー
        })
    }

    fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        _original_capsule: &Capsule,
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

        // TODO: 実際のUmbral実装を使用
        // 現在はプレースホルダー実装
        Ok(vec![0u8; ciphertext.len()]) // プレースホルダー
    }

    fn generate_keypair(&self) -> ServiceResult<(SecretKey, PublicKey)> {
        // umbral-preを使用して実際の鍵ペアを生成
        let umbral_sk: umbral_pre::SecretKey = umbral_pre::SecretKey::random();
        let umbral_pk: umbral_pre::PublicKey = umbral_sk.public_key();

        // ヘルパー関数を使用して秘密鍵をシリアライズ
        let sk_vec: Vec<u8> = Self::serialize_secret_key(&umbral_sk);

        // 公開鍵をシリアライズ
        let pk_bytes: Vec<u8> =
            bincode::serialize(&umbral_pk).map_err(|e: Box<bincode::ErrorKind>| {
                ServiceError::crypto_error(format!("Failed to serialize public key: {}", e))
            })?;

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
        let (owner_sk, owner_pk) = service
            .generate_keypair()
            .expect("Failed to generate owner keypair");
        let (accessor_sk, accessor_pk) = service
            .generate_keypair()
            .expect("Failed to generate accessor keypair");

        println!("\n2. 再暗号化鍵を生成中...");
        let reencryption_key = service
            .generate_reencryption_key(&owner_sk, &accessor_pk)
            .expect("Failed to generate re-encryption key");

        println!("\n3. 生成された再暗号化鍵の詳細:");
        println!("   委任者秘密鍵データサイズ: {} bytes", reencryption_key.delegating_sk_data.len());
        println!("   受信者公開鍵データサイズ: {} bytes", reencryption_key.receiving_pk_data.len());

        assert_eq!(reencryption_key.delegating_sk_data.len(), constants::KEY_SIZE_BYTES);
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
        let (owner_sk, owner_pk) = service
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
