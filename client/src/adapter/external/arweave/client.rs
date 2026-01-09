//! Arweave client implementation
//!
//! Provides production implementation of ArweaveClient trait
//! for Arweave network communication using reqwest HTTP client.

use serde::{Deserialize, Serialize};

use super::config::ArweaveClientConfig;

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
    #[allow(dead_code)]
    pub timestamp: u64,
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
// Arweave Transaction Types (for posting)
// =============================================================================

/// Arweave transaction structure for posting data
#[derive(Debug, Serialize)]
pub(crate) struct ArweaveTransaction {
    /// Transaction format (always 2 for format 2)
    pub format: u8,
    /// Transaction ID (empty string, calculated after signing)
    pub id: String,
    /// Last anchor transaction ID
    pub last_tx: String,
    /// Wallet public key (Base64URL encoded)
    pub owner: String,
    /// Tags with Base64URL encoded name/value
    pub tags: Vec<EncodedTag>,
    /// Target address (empty for data-only transactions)
    pub target: String,
    /// Amount in Winston (0 for data-only transactions)
    pub quantity: String,
    /// Base64URL encoded data
    pub data: String,
    /// Data size as string
    pub data_size: String,
    /// Data root hash
    pub data_root: String,
    /// Transaction reward (fee)
    pub reward: String,
    /// RSA-PSS signature
    pub signature: String,
}

/// Tag with Base64URL encoded name and value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EncodedTag {
    pub name: String,
    pub value: String,
}

// =============================================================================
// Base64URL Encoding Helpers
// =============================================================================

/// Encode bytes to Base64URL format (no padding)
pub(crate) fn base64url_encode(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decode Base64URL format to bytes
pub(crate) fn base64url_decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(encoded)
}

// =============================================================================
// ArweaveClientImpl
// =============================================================================

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

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
    pub fn with_wallet(mut self, wallet: ArweaveWallet) -> Self {
        self.wallet = Some(wallet);
        self
    }

    /// Check if wallet is configured
    pub fn has_wallet(&self) -> bool {
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
    fn build_graphql_query(&self, tags: &[Tag], cursor: Option<&str>) -> String {
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
            r#"{{
                transactions(
                    tags: [{}]
                    first: 100
                    {}
                ) {{
                    edges {{
                        node {{ id }}
                        cursor
                    }}
                    pageInfo {{ hasNextPage }}
                }}
            }}"#,
            tags_query.join(", "),
            after_clause
        )
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ArweaveClient for ArweaveClientImpl {
    async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError> {
        let url = format!("{}/tx/{}/data", self.config.gateway_url(), tx_id);
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None);
                    }

                    if response.status().is_success() {
                        let bytes = response.bytes().await.map_err(|e| {
                            AdapterError::network_error(
                                "get",
                                &format!("Failed to read response body: {e}"),
                                retries,
                            )
                        })?;

                        let decoded_bytes =
                            base64url_decode(std::str::from_utf8(&bytes).map_err(|e| {
                                AdapterError::serialization_error(
                                    "get",
                                    &format!("Invalid UTF-8 in response: {e}"),
                                )
                            })?)
                            .map_err(|e| {
                                AdapterError::serialization_error(
                                    "get",
                                    &format!("Failed to decode Base64URL: {e}"),
                                )
                            })?;

                        return Ok(Some(decoded_bytes));
                    }

                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "get",
                            &format!("HTTP error: {}", response.status()),
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

        // Encode tags to Base64URL
        let encoded_tags: Vec<EncodedTag> = tags
            .iter()
            .map(|t| EncodedTag {
                name: base64url_encode(t.name.as_bytes()),
                value: base64url_encode(t.value.as_bytes()),
            })
            .collect();

        // Encode payload to Base64URL
        let encoded_payload = base64url_encode(payload);

        // Create transaction structure
        let tx = ArweaveTransaction {
            format: 2,
            id: String::new(),
            last_tx: String::new(),
            owner: String::new(),
            tags: encoded_tags,
            target: String::new(),
            quantity: "0".to_string(),
            data: encoded_payload,
            data_size: payload.len().to_string(),
            data_root: String::new(),
            reward: "0".to_string(),
            signature: String::new(),
        };

        // Post transaction
        let url = format!("{}/tx", self.config.gateway_url());
        let mut retries = 0;
        let max_retries = self.config.max_retries();
        let backoff_ms = self.config.retry_backoff_ms();

        loop {
            match self.http_client.post(&url).json(&tx).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let tx_id: String = response.text().await.map_err(|e| {
                            AdapterError::network_error(
                                "post",
                                &format!("Failed to read response: {e}"),
                                retries,
                            )
                        })?;
                        return Ok(tx_id);
                    }

                    if retries >= max_retries {
                        return Err(AdapterError::network_error(
                            "post",
                            &format!("HTTP error: {}", response.status()),
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
            let query = self.build_graphql_query(&tags, cursor.as_deref());
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
                        if response.status().is_success() {
                            break response.json().await.map_err(|e| {
                                AdapterError::serialization_error(
                                    "query",
                                    &format!("Failed to parse response: {e}"),
                                )
                            })?;
                        }

                        if retries >= max_retries {
                            return Err(AdapterError::network_error(
                                "query",
                                &format!("GraphQL request failed: {}", response.status()),
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
