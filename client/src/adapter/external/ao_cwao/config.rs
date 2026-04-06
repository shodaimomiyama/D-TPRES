#[cfg(not(target_arch = "wasm32"))]
use std::env;

use crate::adapter::errors::AOCommunicationError;

const DEFAULT_MU_URL: &str = "https://mu.ao-testnet.xyz";
const DEFAULT_CU_URL: &str = "https://cu.ao-testnet.xyz";
const DEFAULT_GATEWAY_URL: &str = "https://arweave.net";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// AO Network connection configuration
#[derive(Debug, Clone)]
pub struct AOConfig {
    mu_url: String,
    cu_url: String,
    gateway_url: String,
    timeout_ms: u64,
}

impl AOConfig {
    pub fn new(
        mu_url: &str,
        cu_url: &str,
        gateway_url: &str,
        timeout_ms: u64,
    ) -> Result<Self, AOCommunicationError> {
        if mu_url.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "MU URL must not be empty".to_string(),
            });
        }
        if cu_url.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "CU URL must not be empty".to_string(),
            });
        }

        Ok(Self {
            mu_url: mu_url.to_string(),
            cu_url: cu_url.to_string(),
            gateway_url: gateway_url.to_string(),
            timeout_ms,
        })
    }

    pub fn mu_url(&self) -> &str {
        &self.mu_url
    }

    pub fn cu_url(&self) -> &str {
        &self.cu_url
    }

    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_env() -> Result<Self, AOCommunicationError> {
        let mu_url = env::var("AO_MU_URL").unwrap_or_else(|_| DEFAULT_MU_URL.to_string());
        let cu_url = env::var("AO_CU_URL").unwrap_or_else(|_| DEFAULT_CU_URL.to_string());
        let gateway_url =
            env::var("AO_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.to_string());
        let timeout_ms = env::var("AO_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_MS);

        Self::new(&mu_url, &cu_url, &gateway_url, timeout_ms)
    }
}

impl Default for AOConfig {
    fn default() -> Self {
        Self {
            mu_url: DEFAULT_MU_URL.to_string(),
            cu_url: DEFAULT_CU_URL.to_string(),
            gateway_url: DEFAULT_GATEWAY_URL.to_string(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}
