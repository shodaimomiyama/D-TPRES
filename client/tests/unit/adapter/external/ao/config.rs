use formix::adapter::errors::AOCommunicationError;
use formix::adapter::external::ao::AOConfig;

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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_ao_config_from_env_defaults() {
    let config = AOConfig::from_env().unwrap();
    assert!(!config.mu_url().is_empty());
    assert!(!config.cu_url().is_empty());
}

#[test]
fn test_ao_config_invalid_cu_url() {
    let result = AOConfig::new("https://mu.test", "", "https://gw.test", 5000);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AOCommunicationError::ValidationError { .. }));
}
