//! Owner-Process Message Handlers
//!
//! Phase 2: キーフラグメントの分散管理
//! - kFrags受信
//! - Holder-Processへの転送（MVP: 1つのHolderのみ）
//! - 転送状況の管理

use crate::crypto_core::{
    CryptoService, CryptoServiceImpl, KeyFragment, PublicKey, ShamirShare, CryptoResult,
};
use serde::{Deserialize, Serialize};

/// Structure to hold the encryption setup results
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionSetupResult {
    pub kfrags: Vec<KeyFragment>,
    pub shares_count: usize,
    pub threshold: u8,
    pub capsule_id: String,
}

/// Handler for setting up encryption with O-Browser functionality
/// This would typically run in the browser environment, but for MVP it's simulated here
pub fn handle_setup_encryption(
    secret: &[u8],
    threshold: u8,
    total_shares: u8,
) -> CryptoResult<EncryptionSetupResult> {
    let crypto = CryptoServiceImpl::new();
    
    // Step 1: Generate owner keypair (sk_O, pk_O)
    let (owner_sk, owner_pk) = crypto.generate_keypair()?;
    
    // Step 2: Split secret using Shamir's Secret Sharing
    let shares = crypto.split_secret_shamir(secret, threshold, total_shares)?;
    
    // Step 3: Generate encryption key k_O
    let k_o = generate_random_key();
    
    // Step 4: Encrypt each share with k_O
    let encrypted_shares = encrypt_shares(&shares, &k_o)?;
    
    // Step 5: Create capsule (encrypt k_O with owner's public key)
    let (capsule, _ciphertext) = crypto.create_pre_capsule(&owner_pk, &k_o)?;
    
    // Step 6: Generate requester keypair (MVP: auto-generate)
    let (_requester_sk, requester_pk) = crypto.generate_keypair()?;
    
    // Step 7: Generate re-encryption key
    let rekey = crypto.generate_reencryption_key(&owner_sk, &requester_pk)?;
    
    // Step 8: Create kFrags
    let kfrags = crypto.create_kfrags(&rekey, threshold, total_shares)?;
    
    // Generate a capsule ID for tracking
    let capsule_id = generate_capsule_id(&capsule.data);
    
    Ok(EncryptionSetupResult {
        kfrags,
        shares_count: shares.len(),
        threshold,
        capsule_id,
    })
}

/// Handler for storing kFrags in the Owner-Process
pub fn handle_store_kfrags(kfrags: Vec<KeyFragment>) -> CryptoResult<StoreKfragsResult> {
    // In production, this would store kfrags in Arweave
    // For MVP, we return success with metadata
    
    let kfrag_ids: Vec<u8> = kfrags.iter().map(|kf| kf.id).collect();
    
    Ok(StoreKfragsResult {
        stored_count: kfrags.len(),
        kfrag_ids,
        storage_tx_id: generate_storage_tx_id(),
    })
}

/// Handler for retrieving stored kFrags
pub fn handle_get_kfrags(requester_id: &str) -> CryptoResult<Vec<KeyFragment>> {
    // In production, this would:
    // 1. Verify requester's access rights
    // 2. Retrieve kfrags from Arweave
    // 3. Return only authorized kfrags
    
    // MVP: Return mock kfrags
    // In real implementation, these would be retrieved from storage
    Ok(vec![])
}

/// Handler for processing access requests
pub fn handle_access_request(
    requester_id: &str,
    capsule_id: &str,
    requester_pk: &PublicKey,
) -> CryptoResult<AccessRequestResult> {
    // MVP: Auto-approve all access requests
    // In production, this would verify access rights via smart contracts

    Ok(AccessRequestResult {
        approved: true,
        requester_id: requester_id.to_string(),
        capsule_id: capsule_id.to_string(),
        kfrags_available: true,
    })
}

/// MVPのOwner-Processメッセージ形式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OwnerMessage {
    /// kFragsを受信してHolder-Processに転送
    TransferKFrags {
        kfrags: Vec<SerializableKeyFragment>,
        capsule: SerializableCapsule,
        holder_process_id: String,
    },
    /// 転送状況を確認
    GetTransferStatus,
}

/// Owner-Processレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OwnerResponse {
    /// kFrags転送完了
    KFragsTransferred {
        holder_process_id: String,
        transferred_count: usize,
        status: String,
    },
    /// 転送状況
    TransferStatus {
        transfers: Vec<TransferRecord>,
    },
    /// エラーレスポンス
    Error {
        message: String,
    },
}

/// シリアライズ可能なKeyFragment（holder_handlersと共通）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableKeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// シリアライズ可能なCapsule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCapsule {
    pub data: Vec<u8>,
}

/// 転送記録
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    pub holder_process_id: String,
    pub kfrag_count: usize,
    pub timestamp: u64,
    pub status: String,
}

/// Owner-Process状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerState {
    pub process_id: String,
    pub transfer_records: Vec<TransferRecord>,
}

impl OwnerState {
    pub fn new(process_id: String) -> Self {
        Self {
            process_id,
            transfer_records: Vec::new(),
        }
    }
}

/// Owner-Processハンドラー
pub struct OwnerHandler {
    crypto_service: CryptoServiceImpl,
}

impl OwnerHandler {
    pub fn new() -> Self {
        Self {
            crypto_service: CryptoServiceImpl::new(),
        }
    }

    /// メッセージを処理
    pub fn handle_message(&self, message: OwnerMessage, state: &mut OwnerState) -> OwnerResponse {
        match message {
            OwnerMessage::TransferKFrags { kfrags, capsule, holder_process_id } => {
                self.handle_transfer_kfrags(kfrags, capsule, holder_process_id, state)
            },
            OwnerMessage::GetTransferStatus => {
                self.handle_get_transfer_status(state)
            },
        }
    }

    /// kFragsをHolder-Processに転送
    fn handle_transfer_kfrags(
        &self,
        kfrags: Vec<SerializableKeyFragment>,
        capsule: SerializableCapsule,
        holder_process_id: String,
        state: &mut OwnerState
    ) -> OwnerResponse {
        // MVP: 実際の転送はシミュレート
        // 本来はAOメッセージでHolder-Processに送信

        let kfrag_count = kfrags.len();

        // 転送記録を追加
        let transfer_record = TransferRecord {
            holder_process_id: holder_process_id.clone(),
            kfrag_count,
            timestamp: current_timestamp(),
            status: "transferred".to_string(),
        };

        state.transfer_records.push(transfer_record);

        OwnerResponse::KFragsTransferred {
            holder_process_id,
            transferred_count: kfrag_count,
            status: "success".to_string(),
        }
    }

    /// 転送状況を取得
    fn handle_get_transfer_status(&self, state: &OwnerState) -> OwnerResponse {
        OwnerResponse::TransferStatus {
            transfers: state.transfer_records.clone(),
        }
    }
}

impl Default for OwnerHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// AOメッセージ形式でのラッパー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOOwnerMessage {
    pub action: String,
    pub data: serde_json::Value,
}

impl AOOwnerMessage {
    /// AOメッセージからOwnerMessageに変換
    pub fn to_owner_message(&self) -> Result<OwnerMessage, String> {
        match self.action.as_str() {
            "transfer-kfrags" => {
                let transfer_data: TransferKFragsData = serde_json::from_value(self.data.clone())
                    .map_err(|e| format!("Failed to parse transfer-kfrags data: {}", e))?;
                Ok(OwnerMessage::TransferKFrags {
                    kfrags: transfer_data.kfrags,
                    capsule: transfer_data.capsule,
                    holder_process_id: transfer_data.holder_process_id,
                })
            },
            "get-transfer-status" => {
                Ok(OwnerMessage::GetTransferStatus)
            },
            _ => Err(format!("Unknown action: {}", self.action)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransferKFragsData {
    kfrags: Vec<SerializableKeyFragment>,
    capsule: SerializableCapsule,
    holder_process_id: String,
}

/// MVPテスト用のエントリーポイント
pub fn process_owner_message(
    message_json: &str,
    state: &mut OwnerState
) -> Result<String, String> {
    // JSONメッセージをパース
    let ao_message: AOOwnerMessage = serde_json::from_str(message_json)
        .map_err(|e| format!("Failed to parse message: {}", e))?;

    // OwnerMessageに変換
    let owner_message = ao_message.to_owner_message()?;

    // ハンドラーで処理
    let handler = OwnerHandler::new();
    let response = handler.handle_message(owner_message, state);

    // レスポンスをJSONに変換
    serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

/// Result structure for storing kFrags
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreKfragsResult {
    pub stored_count: usize,
    pub kfrag_ids: Vec<u8>,
    pub storage_tx_id: String,
}

/// Result structure for access requests
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessRequestResult {
    pub approved: bool,
    pub requester_id: String,
    pub capsule_id: String,
    pub kfrags_available: bool,
}

/// Helper function to encrypt shares
fn encrypt_shares(shares: &[ShamirShare], key: &[u8]) -> CryptoResult<Vec<Vec<u8>>> {
    // Simple XOR encryption for MVP
    // In production, use proper symmetric encryption (AES-GCM)
    let encrypted: Vec<Vec<u8>> = shares
        .iter()
        .map(|share| {
            share.data
                .iter()
                .zip(key.iter().cycle())
                .map(|(s, k)| s ^ k)
                .collect()
        })
        .collect();
    
    Ok(encrypted)
}

/// Helper function to generate random encryption key
fn generate_random_key() -> Vec<u8> {
    // MVP: Generate deterministic key for testing
    // In production, use proper random generation
    vec![42u8; 32]
}

/// Helper function to generate capsule ID
fn generate_capsule_id(capsule_data: &[u8]) -> String {
    // Simple hash-based ID generation
    // In production, use proper hashing
    format!("capsule_{:x}", capsule_data.len())
}

/// Helper function to generate storage transaction ID
fn generate_storage_tx_id() -> String {
    // MVP: Generate mock transaction ID
    // In production, this would be the actual Arweave transaction ID
    format!("tx_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs())
}

/// Helper function to get current timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_setup_encryption() {
        let secret = b"test secret data";
        let threshold = 2;
        let total_shares = 3;
        
        let result = handle_setup_encryption(secret, threshold, total_shares);
        
        assert!(result.is_ok());
        let setup = result.unwrap();
        assert_eq!(setup.kfrags.len(), total_shares as usize);
        assert_eq!(setup.threshold, threshold);
        assert_eq!(setup.shares_count, total_shares as usize);
    }
    
    #[test]
    fn test_store_kfrags() {
        let crypto = CryptoServiceImpl::new();
        
        // Generate mock kfrags
        let (owner_sk, _) = crypto.generate_keypair().unwrap();
        let (_, requester_pk) = crypto.generate_keypair().unwrap();
        let rekey = crypto.generate_reencryption_key(&owner_sk, &requester_pk).unwrap();
        let kfrags = crypto.create_kfrags(&rekey, 2, 3).unwrap();
        
        let result = handle_store_kfrags(kfrags);
        
        assert!(result.is_ok());
        let store_result = result.unwrap();
        assert_eq!(store_result.stored_count, 3);
        assert_eq!(store_result.kfrag_ids.len(), 3);
    }
    
    #[test]
    fn test_access_request() {
        let crypto = CryptoServiceImpl::new();
        let (_, requester_pk) = crypto.generate_keypair().unwrap();
        
        let result = handle_access_request(
            "requester_123",
            "capsule_456",
            &requester_pk,
        );
        
        assert!(result.is_ok());
        let access = result.unwrap();
        assert!(access.approved); // MVP auto-approves
        assert_eq!(access.requester_id, "requester_123");
        assert_eq!(access.capsule_id, "capsule_456");
    }
}