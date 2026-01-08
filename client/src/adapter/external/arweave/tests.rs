//! Unit tests for Arweave client components

#[cfg(test)]
mod config_tests {
    use crate::adapter::external::arweave::ArweaveClientConfig;

    #[test]
    fn test_default_config_values() {
        println!("\n=== Test: test_default_config_values ===");
        let config = ArweaveClientConfig::default();

        println!("gateway_url: {}", config.gateway_url());
        println!("graphql_url: {}", config.graphql_url());
        println!("timeout_secs: {}", config.timeout_secs());
        println!("max_retries: {}", config.max_retries());
        println!("retry_backoff_ms: {}", config.retry_backoff_ms());

        assert_eq!(config.gateway_url(), "https://arweave.net");
        assert_eq!(config.graphql_url(), "https://arweave.net/graphql");
        assert_eq!(config.timeout_secs(), 30);
        assert_eq!(config.max_retries(), 3);
        assert_eq!(config.retry_backoff_ms(), 1000);
    }

    #[test]
    fn test_builder_pattern() {
        println!("\n=== Test: test_builder_pattern ===");
        let config = ArweaveClientConfig::new()
            .with_gateway_url("https://custom-gateway.io")
            .with_graphql_url("https://custom-graphql.io")
            .with_timeout(60)
            .with_retries(5, 2000);

        println!("Custom config:");
        println!("  gateway_url: {}", config.gateway_url());
        println!("  graphql_url: {}", config.graphql_url());
        println!("  timeout_secs: {}", config.timeout_secs());
        println!("  max_retries: {}", config.max_retries());
        println!("  retry_backoff_ms: {}", config.retry_backoff_ms());

        assert_eq!(config.gateway_url(), "https://custom-gateway.io");
        assert_eq!(config.graphql_url(), "https://custom-graphql.io");
        assert_eq!(config.timeout_secs(), 60);
        assert_eq!(config.max_retries(), 5);
        assert_eq!(config.retry_backoff_ms(), 2000);
    }

    #[test]
    fn test_builder_with_string() {
        println!("\n=== Test: test_builder_with_string ===");
        let gateway = String::from("https://gateway.example.com");
        println!("Input gateway (String): {}", gateway);

        let config = ArweaveClientConfig::new().with_gateway_url(gateway);
        println!("Result gateway_url: {}", config.gateway_url());

        assert_eq!(config.gateway_url(), "https://gateway.example.com");
    }

    #[test]
    fn test_config_clone() {
        println!("\n=== Test: test_config_clone ===");
        let config1 = ArweaveClientConfig::new()
            .with_gateway_url("https://test.io")
            .with_timeout(45);

        let config2 = config1.clone();

        println!(
            "Original: gateway={}, timeout={}",
            config1.gateway_url(),
            config1.timeout_secs()
        );
        println!(
            "Cloned:   gateway={}, timeout={}",
            config2.gateway_url(),
            config2.timeout_secs()
        );

        assert_eq!(config1.gateway_url(), config2.gateway_url());
        assert_eq!(config1.timeout_secs(), config2.timeout_secs());
    }
}

#[cfg(test)]
mod graphql_response_tests {
    use crate::adapter::external::arweave::client::GraphQLResponse;

    #[test]
    fn test_deserialize_successful_response() {
        println!("\n=== Test: test_deserialize_successful_response ===");
        let json = r#"{
            "data": {
                "transactions": {
                    "edges": [
                        {
                            "node": { "id": "tx1" },
                            "cursor": "cursor1"
                        },
                        {
                            "node": { "id": "tx2" },
                            "cursor": "cursor2"
                        }
                    ],
                    "pageInfo": { "hasNextPage": false }
                }
            }
        }"#;
        println!("Input JSON:\n{}", json);

        let response: GraphQLResponse = serde_json::from_str(json).unwrap();
        println!("Parsed response.data: {:?}", response.data);
        println!("Parsed response.errors: {:?}", response.errors);

        assert!(response.data.is_some());
        assert!(response.errors.is_none());

        let data = response.data.unwrap();
        println!("Transaction edges count: {}", data.transactions.edges.len());
        println!(
            "Edge 0: id={}, cursor={}",
            data.transactions.edges[0].node.id, data.transactions.edges[0].cursor
        );
        println!(
            "Edge 1: id={}, cursor={}",
            data.transactions.edges[1].node.id, data.transactions.edges[1].cursor
        );
        println!("hasNextPage: {}", data.transactions.page_info.has_next_page);

        assert_eq!(data.transactions.edges.len(), 2);
        assert_eq!(data.transactions.edges[0].node.id, "tx1");
        assert_eq!(data.transactions.edges[1].node.id, "tx2");
        assert!(!data.transactions.page_info.has_next_page);
    }

    #[test]
    fn test_deserialize_error_response() {
        println!("\n=== Test: test_deserialize_error_response ===");
        let json = r#"{
            "errors": [
                { "message": "Query error occurred" }
            ]
        }"#;
        println!("Input JSON:\n{}", json);

        let response: GraphQLResponse = serde_json::from_str(json).unwrap();
        println!("Parsed response.data: {:?}", response.data);
        println!("Parsed response.errors: {:?}", response.errors);

        assert!(response.data.is_none());
        assert!(response.errors.is_some());

        let errors = response.errors.unwrap();
        println!("Error count: {}", errors.len());
        println!("Error message: {}", errors[0].message);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Query error occurred");
    }

    #[test]
    fn test_deserialize_with_pagination() {
        println!("\n=== Test: test_deserialize_with_pagination ===");
        let json = r#"{
            "data": {
                "transactions": {
                    "edges": [
                        {
                            "node": { "id": "tx1" },
                            "cursor": "abc123"
                        }
                    ],
                    "pageInfo": { "hasNextPage": true }
                }
            }
        }"#;
        println!("Input JSON:\n{}", json);

        let response: GraphQLResponse = serde_json::from_str(json).unwrap();
        let data = response.data.unwrap();

        println!("hasNextPage: {}", data.transactions.page_info.has_next_page);
        println!("cursor: {}", data.transactions.edges[0].cursor);

        assert!(data.transactions.page_info.has_next_page);
        assert_eq!(data.transactions.edges[0].cursor, "abc123");
    }
}

#[cfg(test)]
mod base64url_tests {
    use crate::adapter::external::arweave::client::{base64url_decode, base64url_encode};

    #[test]
    fn test_encode_decode_roundtrip() {
        println!("\n=== Test: test_encode_decode_roundtrip ===");
        let original = b"Hello, Arweave!";
        println!("Original bytes: {:?}", original);
        println!("Original string: {}", String::from_utf8_lossy(original));

        let encoded = base64url_encode(original);
        println!("Encoded (Base64URL): {}", encoded);

        let decoded = base64url_decode(&encoded).unwrap();
        println!("Decoded bytes: {:?}", decoded);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_encode_empty() {
        println!("\n=== Test: test_encode_empty ===");
        let encoded = base64url_encode(b"");
        println!("Empty input encoded: '{}'", encoded);

        assert_eq!(encoded, "");
    }

    #[test]
    fn test_encode_binary_data() {
        println!("\n=== Test: test_encode_binary_data ===");
        let binary = vec![0u8, 255, 128, 64, 32];
        println!("Binary input: {:?}", binary);

        let encoded = base64url_encode(&binary);
        println!("Encoded: {}", encoded);

        let decoded = base64url_decode(&encoded).unwrap();
        println!("Decoded: {:?}", decoded);

        assert_eq!(decoded, binary);
    }

    #[test]
    fn test_decode_invalid_base64() {
        println!("\n=== Test: test_decode_invalid_base64 ===");
        let invalid = "not-valid-base64!!!";
        println!("Invalid input: {}", invalid);

        let result = base64url_decode(invalid);
        println!("Decode result: {:?}", result);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod client_impl_tests {
    use crate::adapter::external::arweave::{ArweaveClientConfig, ArweaveClientImpl};

    #[test]
    fn test_create_client() {
        println!("\n=== Test: test_create_client ===");
        let config = ArweaveClientConfig::default();
        println!("Config: gateway={}", config.gateway_url());

        let client = ArweaveClientImpl::new(config);
        println!("Client creation result: {:?}", client.is_ok());

        assert!(client.is_ok());
    }

    #[test]
    fn test_create_read_only_client() {
        println!("\n=== Test: test_create_read_only_client ===");
        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::read_only(config);

        println!("Read-only client created: {:?}", client.is_ok());
        let client = client.unwrap();
        println!("has_wallet: {}", client.has_wallet());

        assert!(!client.has_wallet());
    }

    #[test]
    fn test_client_with_custom_config() {
        println!("\n=== Test: test_client_with_custom_config ===");
        let config = ArweaveClientConfig::new()
            .with_gateway_url("https://custom.gateway.io")
            .with_timeout(120);

        println!(
            "Custom config: gateway={}, timeout={}",
            config.gateway_url(),
            config.timeout_secs()
        );

        let client = ArweaveClientImpl::new(config);
        println!("Client creation result: {:?}", client.is_ok());

        assert!(client.is_ok());
    }
}

#[cfg(test)]
mod wallet_tests {
    use crate::adapter::external::arweave::ArweaveWallet;
    use serde_json::json;

    #[test]
    fn test_wallet_from_valid_jwk() {
        println!("\n=== Test: test_wallet_from_valid_jwk ===");
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });
        println!("JWK kty: {}", jwk["kty"]);
        println!(
            "JWK n (first 50 chars): {}...",
            &jwk["n"].as_str().unwrap()[..50]
        );

        let wallet = ArweaveWallet::from_jwk(jwk);
        println!("Wallet creation result: {:?}", wallet.is_ok());

        assert!(wallet.is_ok());
    }

    #[test]
    fn test_wallet_address_derivation() {
        println!("\n=== Test: test_wallet_address_derivation ===");
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });

        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let address = wallet.address();

        println!("Wallet address: {}", address);
        println!("Address length: {} (expected 43)", address.len());

        assert_eq!(address.len(), 43);
    }

    #[test]
    fn test_wallet_owner() {
        println!("\n=== Test: test_wallet_owner ===");
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });

        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let owner = wallet.owner();

        println!(
            "Owner (first 80 chars): {}...",
            &owner[..80.min(owner.len())]
        );
        println!("Owner length: {}", owner.len());

        assert!(!owner.is_empty());
    }

    #[test]
    fn test_wallet_missing_kty() {
        println!("\n=== Test: test_wallet_missing_kty ===");
        let jwk = json!({
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx",
            "e": "AQAB"
        });
        println!("JWK (missing kty): {:?}", jwk);

        let result = ArweaveWallet::from_jwk(jwk);
        println!("Result: {:?}", result);

        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_invalid_kty() {
        println!("\n=== Test: test_wallet_invalid_kty ===");
        let jwk = json!({
            "kty": "EC",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx",
            "e": "AQAB"
        });
        println!("JWK kty: {} (expected RSA)", jwk["kty"]);

        let result = ArweaveWallet::from_jwk(jwk);
        println!("Result: {:?}", result);

        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_missing_modulus() {
        println!("\n=== Test: test_wallet_missing_modulus ===");
        let jwk = json!({
            "kty": "RSA",
            "e": "AQAB"
        });
        println!("JWK (missing n): {:?}", jwk);

        let result = ArweaveWallet::from_jwk(jwk);
        println!("Result: {:?}", result);

        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_sign_without_private_key() {
        println!("\n=== Test: test_wallet_sign_without_private_key ===");
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });
        println!("JWK has 'd' (private key): {}", jwk.get("d").is_some());

        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let result = wallet.sign(b"test data");

        println!("Sign result (should fail): {:?}", result);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod error_tests {
    use crate::adapter::errors::AdapterError;

    #[test]
    fn test_network_error_display() {
        println!("\n=== Test: test_network_error_display ===");
        let err = AdapterError::network_error("get", "Connection timeout", 3);
        let msg = format!("{err}");

        println!("Error: {}", msg);

        assert!(msg.contains("Network error"));
        assert!(msg.contains("get"));
        assert!(msg.contains("3 retries"));
    }

    #[test]
    fn test_configuration_error_display() {
        println!("\n=== Test: test_configuration_error_display ===");
        let err = AdapterError::configuration_error("wallet", "Missing private key");
        let msg = format!("{err}");

        println!("Error: {}", msg);

        assert!(msg.contains("Configuration error"));
        assert!(msg.contains("wallet"));
    }

    #[test]
    fn test_validation_error_display() {
        println!("\n=== Test: test_validation_error_display ===");
        let err = AdapterError::validation_error("post", "Data too large");
        let msg = format!("{err}");

        println!("Error: {}", msg);

        assert!(msg.contains("Validation error"));
        assert!(msg.contains("post"));
    }

    #[test]
    fn test_query_error_display() {
        println!("\n=== Test: test_query_error_display ===");
        let err = AdapterError::query_error("query", "Invalid GraphQL syntax");
        let msg = format!("{err}");

        println!("Error: {}", msg);

        assert!(msg.contains("Query error"));
        assert!(msg.contains("query"));
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::adapter::external::arweave::{
        ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet,
    };
    use crate::adapter::repository_impl::{ArweaveClient, Tag};

    fn is_integration_test_enabled() -> bool {
        let _ = dotenvy::dotenv();
        std::env::var("ARWEAVE_INTEGRATION_TESTS")
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    fn try_load_wallet_from_env() -> Option<ArweaveWallet> {
        match ArweaveWallet::from_env() {
            Ok(wallet) => {
                println!("Wallet loaded from ARWEAVE_WALLET_JWK");
                println!("  Address: {}", wallet.address());
                Some(wallet)
            }
            Err(e) => {
                println!("Skipping (no wallet): {:?}", e);
                None
            }
        }
    }

    #[tokio::test]
    async fn test_get_transaction_from_arweave() {
        if !is_integration_test_enabled() {
            println!("Skipping: set ARWEAVE_INTEGRATION_TESTS=true in .env to run");
            return;
        }

        println!("\n=== Integration Test: get_transaction_from_arweave ===");

        let config = ArweaveClientConfig::default();
        println!("Gateway: {}", config.gateway_url());

        let client = ArweaveClientImpl::new(config).unwrap();

        let tx_id = "BNttzDav3jHVnNiV7nYbQv-GY0HQ-4XXsdkE5K9ylHQ";
        println!("Fetching tx_id: {}", tx_id);

        let result = client.get(tx_id).await;
        println!("Result: {:?}", result);

        match &result {
            Ok(Some(bytes)) => println!("Data size: {} bytes", bytes.len()),
            Ok(None) => println!("Transaction not found (404)"),
            Err(e) => println!("Error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_query_transactions_by_tags() {
        if !is_integration_test_enabled() {
            println!("Skipping: set ARWEAVE_INTEGRATION_TESTS=true in .env to run");
            return;
        }

        println!("\n=== Integration Test: query_transactions_by_tags ===");

        let config = ArweaveClientConfig::default();
        println!("GraphQL URL: {}", config.graphql_url());

        let client = ArweaveClientImpl::new(config).unwrap();

        let tags = vec![Tag::new("App-Name", "ArDrive")];
        println!("Query tags: {:?}", tags);

        let result = client.query(tags).await;

        match &result {
            Ok(tx_ids) => {
                println!("Found {} transactions", tx_ids.len());
                for (i, tx_id) in tx_ids.iter().take(10).enumerate() {
                    println!("  [{}] {}", i, tx_id);
                }
                if tx_ids.len() > 10 {
                    println!("  ... and {} more", tx_ids.len() - 10);
                }
            }
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
