//! Arweave client configuration
//!
//! Provides configuration structure and builder pattern for ArweaveClientImpl.

use std::env;

use crate::adapter::errors::AdapterError;

const DEFAULT_GATEWAY_URL: &str = "https://arweave.net";
const DEFAULT_GRAPHQL_URL: &str = "https://arweave.net/graphql";
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 1000;

/// Configuration for ArweaveClientImpl
#[derive(Debug, Clone)]
pub struct ArweaveClientConfig {
    pub(crate) gateway_url: String,
    pub(crate) graphql_url: String,
    pub(crate) timeout_secs: u64,
    pub(crate) max_retries: u32,
    pub(crate) retry_backoff_ms: u64,
}

impl ArweaveClientConfig {
    /// Create a new configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the gateway URL
    #[must_use]
    pub fn with_gateway_url(mut self, url: impl Into<String>) -> Self {
        self.gateway_url = url.into();
        self
    }

    /// Set the GraphQL endpoint URL
    #[must_use]
    pub fn with_graphql_url(mut self, url: impl Into<String>) -> Self {
        self.graphql_url = url.into();
        self
    }

    /// Set the request timeout in seconds
    #[must_use]
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set retry configuration
    #[must_use]
    pub fn with_retries(mut self, max: u32, backoff_ms: u64) -> Self {
        self.max_retries = max;
        self.retry_backoff_ms = backoff_ms;
        self
    }

    /// Create configuration from environment variables
    ///
    /// Reads the following environment variables:
    /// - `ARWEAVE_GATEWAY_URL`: Gateway URL (default: https://arweave.net)
    /// - `ARWEAVE_GRAPHQL_URL`: GraphQL URL (default: https://arweave.net/graphql)
    /// - `ARWEAVE_TIMEOUT_SECS`: Request timeout in seconds (default: 30)
    /// - `ARWEAVE_MAX_RETRIES`: Maximum retry count (default: 3)
    /// - `ARWEAVE_RETRY_BACKOFF_MS`: Retry backoff in milliseconds (default: 1000)
    pub fn from_env() -> Result<Self, AdapterError> {
        let gateway_url =
            env::var("ARWEAVE_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.to_string());
        let graphql_url =
            env::var("ARWEAVE_GRAPHQL_URL").unwrap_or_else(|_| DEFAULT_GRAPHQL_URL.to_string());

        let timeout_secs = env::var("ARWEAVE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let max_retries = env::var("ARWEAVE_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let retry_backoff_ms = env::var("ARWEAVE_RETRY_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_RETRY_BACKOFF_MS);

        Ok(Self {
            gateway_url,
            graphql_url,
            timeout_secs,
            max_retries,
            retry_backoff_ms,
        })
    }

    /// Get the gateway URL
    #[must_use]
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    /// Get the GraphQL URL
    #[must_use]
    pub fn graphql_url(&self) -> &str {
        &self.graphql_url
    }

    /// Get the timeout in seconds
    #[must_use]
    pub const fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    /// Get the maximum retry count
    #[must_use]
    pub const fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the retry backoff in milliseconds
    #[must_use]
    pub const fn retry_backoff_ms(&self) -> u64 {
        self.retry_backoff_ms
    }
}

impl Default for ArweaveClientConfig {
    fn default() -> Self {
        Self {
            gateway_url: DEFAULT_GATEWAY_URL.to_string(),
            graphql_url: DEFAULT_GRAPHQL_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff_ms: DEFAULT_RETRY_BACKOFF_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ArweaveClientConfig::default();
        assert_eq!(config.gateway_url(), DEFAULT_GATEWAY_URL);
        assert_eq!(config.graphql_url(), DEFAULT_GRAPHQL_URL);
        assert_eq!(config.timeout_secs(), DEFAULT_TIMEOUT_SECS);
        assert_eq!(config.max_retries(), DEFAULT_MAX_RETRIES);
        assert_eq!(config.retry_backoff_ms(), DEFAULT_RETRY_BACKOFF_MS);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ArweaveClientConfig::new()
            .with_gateway_url("https://custom.gateway.io")
            .with_graphql_url("https://custom.graphql.io")
            .with_timeout(60)
            .with_retries(5, 2000);

        assert_eq!(config.gateway_url(), "https://custom.gateway.io");
        assert_eq!(config.graphql_url(), "https://custom.graphql.io");
        assert_eq!(config.timeout_secs(), 60);
        assert_eq!(config.max_retries(), 5);
        assert_eq!(config.retry_backoff_ms(), 2000);
    }

    #[test]
    fn test_from_env_defaults() {
        let config = ArweaveClientConfig::from_env().unwrap();
        assert!(!config.gateway_url().is_empty());
        assert!(!config.graphql_url().is_empty());
    }
}
