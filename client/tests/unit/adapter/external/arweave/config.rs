use formix::adapter::external::arweave::ArweaveClientConfig;

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
