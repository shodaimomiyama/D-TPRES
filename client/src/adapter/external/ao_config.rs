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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao_config_fields() {
        let config = AOConfig::new(
            "https://mu.test",
            "https://cu.test",
            "https://gw.test",
            5000,
        )
        .unwrap();
        assert_eq!(config.mu_url(), "https://mu.test");
        assert_eq!(config.cu_url(), "https://cu.test");
        assert_eq!(config.gateway_url(), "https://gw.test");
        assert_eq!(config.timeout_ms(), 5000);
    }

    #[test]
    fn test_ao_config_default_endpoints() {
        let config = AOConfig::default();
        assert_eq!(config.mu_url(), "https://mu.ao-testnet.xyz");
        assert_eq!(config.cu_url(), "https://cu.ao-testnet.xyz");
        assert_eq!(config.gateway_url(), "https://arweave.net");
        assert_eq!(config.timeout_ms(), 30_000);
    }

    #[test]
    fn test_ao_config_invalid_mu_url() {
        let result = AOConfig::new("", "https://cu.test", "https://gw.test", 5000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AOCommunicationError::ValidationError { .. }));
    }

    #[test]
    fn test_ao_config_invalid_cu_url() {
        let result = AOConfig::new("https://mu.test", "", "https://gw.test", 5000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AOCommunicationError::ValidationError { .. }));
    }
}
