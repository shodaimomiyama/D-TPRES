//! Arweave HTTP client implementation
//!
//! Provides production implementation of ArweaveClient trait
//! for Arweave network communication using reqwest HTTP client.
//!
//! This module focuses on HTTP operations only. Transaction building,
//! DeepHash, and Merkle tree calculations are in separate modules.

use serde::Deserialize;

use super::config::ArweaveClientConfig;
use super::merkle::compute_data_root;
use super::transaction::{ArweaveTransaction, EncodedTag, build_signature_data};

// =============================================================================
// GraphQL Response Types
// =============================================================================

/// GraphQL response wrapper
#[derive(Debug, Deserialize)]
pub(crate) struct GraphQLResponse {
    pub data: Option<TransactionsData>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// Transactions data from GraphQL response
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionsData {
    pub transactions: TransactionConnection,
}

/// Transaction connection with pagination
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionConnection {
    pub edges: Vec<TransactionEdge>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

/// Transaction edge in GraphQL connection
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionEdge {
    pub node: TransactionNode,
    pub cursor: String,
}

/// Transaction node with ID and block info
#[derive(Debug, Deserialize)]
pub(crate) struct TransactionNode {
    pub id: String,
    #[allow(dead_code)]
    pub block: Option<BlockInfo>,
}

/// Block information
#[derive(Debug, Deserialize)]
pub(crate) struct BlockInfo {
    #[allow(dead_code)]
    pub height: u64,
}

/// Pagination info for GraphQL queries
#[derive(Debug, Deserialize)]
pub(crate) struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

/// GraphQL error response
#[derive(Debug, Deserialize)]
pub(crate) struct GraphQLError {
    pub message: String,
}

// =============================================================================
// Base64URL Encoding Helpers
// =============================================================================

/// Encode bytes to Base64URL format (no padding)
pub fn base64url_encode(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decode Base64URL format to bytes
pub fn base64url_decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(encoded)
}

// =============================================================================
// ArweaveClientImpl
// =============================================================================

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use sha2::{Digest, Sha256};

use crate::adapter::errors::AdapterError;

// Platform-specific sleep implementation for retry backoff
#[cfg(not(target_arch = "wasm32"))]
async fn sleep_backoff(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
async fn sleep_backoff(_duration: Duration) {
    // WASM environment: immediate retry without delay
    // Browser fetch handles its own timeout/retry semantics
}
use crate::adapter::repository_impl::{ArweaveClient, Tag};

use super::wallet::ArweaveWallet;

/// Production implementation of ArweaveClient trait
pub struct ArweaveClientImpl {
    config: ArweaveClientConfig,
    http_client: Client,
    wallet: Option<ArweaveWallet>,
}

impl ArweaveClientImpl {
    /// Create a new ArweaveClientImpl with the given configuration
    pub fn new(config: ArweaveClientConfig) -> Result<Self, AdapterError> {
        #[cfg(not(target_arch = "wasm32"))]
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs()))
            .build()
            .map_err(|e| {
                AdapterError::configuration_error(
                    "http_client",
                    &format!("Failed to create HTTP client: {e}"),
                )
            })?;

        #[cfg(target_arch = "wasm32")]
        let http_client = Client::builder().build().map_err(|e| {
            AdapterError::configuration_error(
                "http_client",
                &format!("Failed to create HTTP client: {e}"),
            )
        })?;

        Ok(Self {
            config,
            http_client,
            wallet: None,
        })
    }

    /// Create a read-only ArweaveClientImpl (without wallet)
    pub fn read_only(config: ArweaveClientConfig) -> Result<Self, AdapterError> {
        Self::new(config)
    }

    /// Add a wallet for signing transactions
    #[must_use]
    pub fn with_wallet(mut self, wallet: ArweaveWallet) -> Self {
        self.wallet = Some(wallet);
        self
    }

    /// Check if wallet is configured
    pub const fn has_wallet(&self) -> bool {
        self.wallet.is_some()
    }

    /// Escape special characters in GraphQL string values to prevent injection
    fn escape_graphql_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Build GraphQL query for transaction search
    fn build_graphql_query(tags: &[Tag], cursor: Option<&str>) -> String {
        let tags_query: Vec<String> = tags
            .iter()
            .map(|t| {
                let name = Self::escape_graphql_string(&t.name);
                let value = Self::escape_graphql_string(&t.value);
                format!(r#"{{ name: "{name}", values: ["{value}"] }}"#)
            })
            .collect();

        let after_clause = cursor
            .map(|c| {
                let escaped = Self::escape_graphql_string(c);
                format!(r#", after: "{escaped}""#)
            })
            .unwrap_or_default();

        format!(
            r"{{
                transactions(
                    tags: [{}]
                    first: 100
                    sort: HEIGHT_DESC
                    {}
                ) {{
                    edges {{
                        node {{
                            id
                            block {{ height }}
                        }}
                        cursor
                    }}
                    pageInfo {{ hasNextPage }}
                }}
            }}",
            tags_query.join(", "),
            after_clause
        )
    }

    /// Fetch transaction anchor from the network
    async fn fetch_anchor(&self) -> Result<String, AdapterError> {
        let url = format!("{}/tx_anchor", self.config.gateway_url());
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            let result = self.http_client.get(&url).send().await;
            let should_retry = match result {
                Ok(response) if response.status().is_success() => {
                    return response
                        .text()
                        .await
                        .map_err(|e| Self::anchor_read_error(e, retries));
                }
                Ok(response) => {
                    if retries >= max_retries {
                        return Err(Self::anchor_http_error(response.status(), retries));
                    }
                    true
                }
                Err(e) => {
                    if retries >= max_retries {
                        return Err(Self::anchor_request_error(e, retries));
                    }
                    true
                }
            };

            if should_retry {
                retries += 1;
                sleep_backoff(Duration::from_millis(backoff_ms * (1 << retries))).await;
            }
        }
    }

    fn anchor_read_error(e: reqwest::Error, retries: u32) -> AdapterError {
        AdapterError::network_error(
            "fetch_anchor",
            &format!("Failed to read anchor: {e}"),
            retries,
        )
    }

    fn anchor_http_error(status: reqwest::StatusCode, retries: u32) -> AdapterError {
        AdapterError::network_error("fetch_anchor", &format!("HTTP error: {status}"), retries)
    }

    fn anchor_request_error(e: reqwest::Error, retries: u32) -> AdapterError {
        AdapterError::network_error("fetch_anchor", &format!("Request failed: {e}"), retries)
    }

    /// Fetch transaction price (reward) for given data size
    async fn fetch_price(&self, data_size: usize) -> Result<String, AdapterError> {
        let url = format!("{}/price/{data_size}", self.config.gateway_url());
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            let result = self.http_client.get(&url).send().await;
            let should_retry = match result {
                Ok(response) if response.status().is_success() => {
                    return response
                        .text()
                        .await
                        .map_err(|e| Self::price_read_error(e, retries));
                }
                Ok(response) => {
                    if retries >= max_retries {
                        return Err(Self::price_http_error(response.status(), retries));
                    }
                    true
                }
                Err(e) => {
                    if retries >= max_retries {
                        return Err(Self::price_request_error(e, retries));
                    }
                    true
                }
            };

            if should_retry {
                retries += 1;
                sleep_backoff(Duration::from_millis(backoff_ms * (1 << retries))).await;
            }
        }
    }

    fn price_read_error(e: reqwest::Error, retries: u32) -> AdapterError {
        AdapterError::network_error(
            "fetch_price",
            &format!("Failed to read price: {e}"),
            retries,
        )
    }

    fn price_http_error(status: reqwest::StatusCode, retries: u32) -> AdapterError {
        AdapterError::network_error("fetch_price", &format!("HTTP error: {status}"), retries)
    }

    fn price_request_error(e: reqwest::Error, retries: u32) -> AdapterError {
        AdapterError::network_error("fetch_price", &format!("Request failed: {e}"), retries)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ArweaveClient for ArweaveClientImpl {
    async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError> {
        // Use raw data endpoint (/{tx_id}) which returns bytes directly
        let url = format!("{}/{}", self.config.gateway_url(), tx_id);
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status();

                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None);
                    }

                    // 4xx client errors are not retryable - fail fast
                    if status.is_client_error() {
                        return Err(AdapterError::network_error(
                            "get",
                            &format!("Client error: {status}"),
                            retries,
                        ));
                    }

                    if status.is_success() {
                        let bytes = response.bytes().await.map_err(|e| {
                            AdapterError::network_error(
                                "get",
                                &format!("Failed to read response body: {e}"),
                                retries,
                            )
                        })?;

                        return Ok(Some(bytes.to_vec()));
                    }

                    // 5xx server errors are retryable
                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "get",
                            &format!("HTTP error: {status}"),
                            retries,
                        ));
                    }
                }
                Err(e) => {
                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "get",
                            &format!("Request failed: {e}"),
                            retries,
                        ));
                    }
                }
            }

            retries += 1;
            sleep_backoff(Duration::from_millis(backoff_ms * (1 << retries))).await;
        }
    }

    async fn post(&self, payload: &[u8], tags: Vec<Tag>) -> Result<String, AdapterError> {
        use crate::adapter::repository_impl::{tag_names, tag_values};

        // Wallet validation: fail fast if no wallet configured
        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AdapterError::configuration_error(
                "post",
                "Wallet not configured - cannot sign transaction",
            )
        })?;

        // Validate payload size (max 2MB for inline data)
        const MAX_DATA_SIZE: usize = 2 * 1024 * 1024;
        if payload.len() > MAX_DATA_SIZE {
            return Err(AdapterError::validation_error(
                "post",
                &format!(
                    "Data size {} exceeds maximum {} bytes",
                    payload.len(),
                    MAX_DATA_SIZE
                ),
            ));
        }

        // Auto-inject D-TPRES application tag if not already present
        let mut all_tags = tags;
        let has_app_tag = all_tags
            .iter()
            .any(|t| t.name == tag_names::APP_NAME && t.value == tag_values::APP_NAME);
        if !has_app_tag {
            all_tags.push(Tag::new(tag_names::APP_NAME, tag_values::APP_NAME));
        }

        // Encode tags to Base64URL
        let encoded_tags: Vec<EncodedTag> = all_tags
            .iter()
            .map(|t| EncodedTag {
                name: base64url_encode(t.name.as_bytes()),
                value: base64url_encode(t.value.as_bytes()),
            })
            .collect();

        // Fetch anchor and price from network
        let last_tx = self.fetch_anchor().await?;
        let reward = self.fetch_price(payload.len()).await?;

        // Encode payload to Base64URL
        let encoded_payload = base64url_encode(payload);

        // Compute data root using Merkle tree
        let data_root = compute_data_root(payload);

        // Create unsigned transaction structure
        let mut tx = ArweaveTransaction {
            format: 2,
            id: String::new(),
            last_tx,
            owner: wallet.owner(),
            tags: encoded_tags,
            target: String::new(),
            quantity: "0".to_string(),
            data: encoded_payload,
            data_size: payload.len().to_string(),
            data_root,
            reward,
            signature: String::new(),
        };

        // Sign the transaction using DeepHash
        let signature_data = build_signature_data(&tx)?;
        let signature_bytes = wallet.sign(&signature_data)?;
        tx.signature = base64url_encode(&signature_bytes);

        // Compute transaction ID from signature hash
        let mut hasher = Sha256::new();
        hasher.update(&signature_bytes);
        tx.id = base64url_encode(&hasher.finalize());

        // Post transaction
        let url = format!("{}/tx", self.config.gateway_url());
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            match self.http_client.post(&url).json(&tx).send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        // Consume response body (Arweave returns "OK" or empty, not the TX ID)
                        let _ = response.text().await;
                        // Return pre-calculated transaction ID (SHA-256 hash of signature)
                        return Ok(tx.id.clone());
                    }

                    // 4xx client errors are not retryable - fail fast
                    if status.is_client_error() {
                        return Err(AdapterError::network_error(
                            "post",
                            &format!("Client error: {status}"),
                            retries,
                        ));
                    }

                    // 5xx server errors are retryable
                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "post",
                            &format!("HTTP error: {status}"),
                            retries,
                        ));
                    }
                }
                Err(e) => {
                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "post",
                            &format!("Request failed: {e}"),
                            retries,
                        ));
                    }
                }
            }

            retries += 1;
            sleep_backoff(Duration::from_millis(backoff_ms * (1 << retries))).await;
        }
    }

    async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError> {
        let mut all_tx_ids = Vec::new();
        let mut cursor: Option<String> = None;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            let query = Self::build_graphql_query(&tags, cursor.as_deref());
            let payload = serde_json::json!({ "query": query });
            let mut retries = 0;

            let graphql_response: GraphQLResponse = loop {
                match self
                    .http_client
                    .post(self.config.graphql_url())
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(response) => {
                        let status = response.status();

                        if status.is_success() {
                            break response.json().await.map_err(|e| {
                                AdapterError::serialization_error(
                                    "query",
                                    &format!("Failed to parse response: {e}"),
                                )
                            })?;
                        }

                        // 4xx client errors are not retryable - fail fast
                        if status.is_client_error() {
                            return Err(AdapterError::network_error(
                                "query",
                                &format!("Client error: {status}"),
                                retries,
                            ));
                        }

                        // 5xx server errors are retryable
                        if retries >= max_retries {
                            return Err(AdapterError::network_error(
                                "query",
                                &format!("GraphQL request failed: {status}"),
                                retries,
                            ));
                        }
                    }
                    Err(e) => {
                        if retries >= max_retries {
                            return Err(AdapterError::network_error(
                                "query",
                                &format!("Request failed: {e}"),
                                retries,
                            ));
                        }
                    }
                }

                retries += 1;
                sleep_backoff(Duration::from_millis(backoff_ms * (1 << retries))).await;
            };

            if let Some(errors) = graphql_response.errors {
                if !errors.is_empty() {
                    return Err(AdapterError::query_error(
                        "query",
                        &errors
                            .iter()
                            .map(|e| e.message.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
                }
            }

            let response_data = graphql_response
                .data
                .ok_or_else(|| AdapterError::query_error("query", "No data in GraphQL response"))?;

            for edge in &response_data.transactions.edges {
                all_tx_ids.push(edge.node.id.clone());
            }

            // Empty edges guard to prevent infinite loop when has_next_page is true but no results
            if response_data.transactions.edges.is_empty() {
                break;
            }

            if response_data.transactions.page_info.has_next_page {
                cursor = response_data
                    .transactions
                    .edges
                    .last()
                    .map(|e| e.cursor.clone());
            } else {
                break;
            }
        }

        Ok(all_tx_ids)
    }
}
