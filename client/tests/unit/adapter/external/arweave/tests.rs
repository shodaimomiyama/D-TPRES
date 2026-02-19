use formix::adapter::errors::AdapterError;
use formix::adapter::external::arweave::{ArweaveClientConfig, ArweaveClientImpl, ArweaveWallet};
use formix::adapter::repository_impl::{ArweaveClient, Tag};
use serde_json::json;

mod config_tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = ArweaveClientConfig::default();
        assert_eq!(config.gateway_url(), "https://arweave.net");
        assert_eq!(config.graphql_url(), "https://arweave.net/graphql");
        assert_eq!(config.timeout_secs(), 30);
        assert_eq!(config.max_retries(), 3);
        assert_eq!(config.retry_backoff_ms(), 1000);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ArweaveClientConfig::new()
            .with_gateway_url("https://custom-gateway.io")
            .with_graphql_url("https://custom-graphql.io")
            .with_timeout(60)
            .with_retries(5, 2000);

        assert_eq!(config.gateway_url(), "https://custom-gateway.io");
        assert_eq!(config.graphql_url(), "https://custom-graphql.io");
        assert_eq!(config.timeout_secs(), 60);
        assert_eq!(config.max_retries(), 5);
        assert_eq!(config.retry_backoff_ms(), 2000);
    }

    #[test]
    fn test_builder_with_string() {
        let gateway = String::from("https://gateway.example.com");
        let config = ArweaveClientConfig::new().with_gateway_url(gateway);
        assert_eq!(config.gateway_url(), "https://gateway.example.com");
    }

    #[test]
    fn test_config_clone() {
        let config1 = ArweaveClientConfig::new()
            .with_gateway_url("https://test.io")
            .with_timeout(45);

        let config2 = config1.clone();

        assert_eq!(config1.gateway_url(), config2.gateway_url());
        assert_eq!(config1.timeout_secs(), config2.timeout_secs());
    }
}

mod client_impl_tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_read_only_client() {
        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::read_only(config);
        let client = client.unwrap();
        assert!(!client.has_wallet());
    }

    #[test]
    fn test_client_with_custom_config() {
        let config = ArweaveClientConfig::new()
            .with_gateway_url("https://custom.gateway.io")
            .with_timeout(120);

        let client = ArweaveClientImpl::new(config);
        assert!(client.is_ok());
    }
}

mod wallet_tests {
    use super::*;

    fn create_test_jwk() -> serde_json::Value {
        json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB",
            "d": "X4cTteJY_gn4FYPsXB8rdXix5vwsg1FLN5E3EaG6RJoVH-HLLKD9M7dx5oo7GURknchnrRweUkC7hT5fJLM0WbFAKNLWY2vv7B6NqXSzUvxT0_YSfqijwp3RTzlBaCxWp4doFk5N2o8Gy_nHNKroADIkJ46pRUohsXywbReAdYaMwFs9tv8d_cPVY3i07a3t8MN6TNwm0dSawm9v47UiCl3Sk5ZiG7xojPLu4sbg1U2jx4IBTNBznbJSzFHK66jT8bgkuqsk0GjskDJk19Z4qwjwbsnn4j2WBii3RL-Us2lGVkY8fkFzme1z0HbIkfz0Y6mqnOYtqc0X4jfcKoAC8Q",
            "p": "83i-7IvMGXoMXCskv73TKr8637FiO7Z27zv8oj6pbWUQyLPQBQxtPVnwD20R-60eTDmD2ujnMt5PoqMrm8RfmNhVWDtjjMmCMjOpSXicFHj7XOuVIYQyqVWlWEh6dN36GVZYk93N8Bc9vY41xy8B9RzzOGVQzXvNEvn7O0nVbfs",
            "q": "3dfOR9cuYq-0S-mkFLzgItgMEfFzB2q3hWehMuG0oCuqnb3vobLyumqjb37qSxPODCQt1yY0EHTy6EaJ2sG3-xLLlRqfvPyM7AqZAVzu9NMs0F-4V3OBJzuVxhqkzNjgQCc7Nh9rEGlZyQOXLFHGXUsnJHJCDYUzz7LxPD7pzKM"
        })
    }

    #[test]
    fn test_wallet_from_valid_jwk() {
        let jwk = create_test_jwk();
        let wallet = ArweaveWallet::from_jwk(jwk);
        assert!(wallet.is_ok());
    }

    #[test]
    fn test_wallet_address_derivation() {
        let jwk = create_test_jwk();
        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let address = wallet.address();
        assert_eq!(address.len(), 43);
    }

    #[test]
    fn test_wallet_owner() {
        let jwk = create_test_jwk();
        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let owner = wallet.owner();
        assert!(!owner.is_empty());
    }

    #[test]
    fn test_wallet_missing_kty() {
        let jwk = json!({
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx",
            "e": "AQAB"
        });
        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_invalid_kty() {
        let jwk = json!({
            "kty": "EC",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx",
            "e": "AQAB"
        });
        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_missing_modulus() {
        let jwk = json!({
            "kty": "RSA",
            "e": "AQAB"
        });
        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_missing_private_key() {
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });
        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn test_network_error_display() {
        let err = AdapterError::network_error("get", "Connection timeout", 3);
        let msg = format!("{err}");
        assert!(msg.contains("Network error"));
        assert!(msg.contains("get"));
        assert!(msg.contains("3 retries"));
    }

    #[test]
    fn test_configuration_error_display() {
        let err = AdapterError::configuration_error("wallet", "Missing private key");
        let msg = format!("{err}");
        assert!(msg.contains("Configuration error"));
        assert!(msg.contains("wallet"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = AdapterError::validation_error("post", "Data too large");
        let msg = format!("{err}");
        assert!(msg.contains("Validation error"));
        assert!(msg.contains("post"));
    }

    #[test]
    fn test_query_error_display() {
        let err = AdapterError::query_error("query", "Invalid GraphQL syntax");
        let msg = format!("{err}");
        assert!(msg.contains("Query error"));
        assert!(msg.contains("query"));
    }
}

mod integration_tests {
    use super::*;

    fn is_integration_test_enabled() -> bool {
        let _ = dotenvy::dotenv();
        std::env::var("ARWEAVE_INTEGRATION_TESTS")
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    fn try_load_wallet_from_env() -> Option<ArweaveWallet> {
        match ArweaveWallet::from_env() {
            Ok(wallet) => Some(wallet),
            Err(_) => None,
        }
    }

    #[tokio::test]
    async fn test_get_transaction_from_arweave() {
        if !is_integration_test_enabled() {
            return;
        }

        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::new(config).unwrap();
        let tx_id = "BNttzDav3jHVnNiV7nYbQv-GY0HQ-4XXsdkE5K9ylHQ";
        let _result = client.get(tx_id).await;
    }

    #[tokio::test]
    async fn test_query_transactions_by_tags() {
        if !is_integration_test_enabled() {
            return;
        }

        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::new(config).unwrap();
        let tags = vec![Tag::new("App-Name", "ArDrive")];
        let _result = client.query(tags).await;
    }

    #[tokio::test]
    async fn test_post_transaction_to_arweave() {
        if !is_integration_test_enabled() {
            return;
        }

        let wallet = match try_load_wallet_from_env() {
            Some(w) => w,
            None => return,
        };

        let config = ArweaveClientConfig::default();
        let client = ArweaveClientImpl::new(config).unwrap().with_wallet(wallet);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let payload = format!("FORMIX integration test @ {timestamp}");
        let payload_bytes = payload.as_bytes();

        let tags = vec![
            Tag::new("App-Name", "FORMIX-Test"),
            Tag::new("Content-Type", "text/plain"),
            Tag::new("Test-Timestamp", timestamp.to_string()),
        ];

        let result = client.post(payload_bytes, tags).await;
        assert!(result.is_ok(), "POST should succeed with valid wallet");
        let tx_id = result.unwrap();
        assert!(!tx_id.is_empty());
        assert_eq!(tx_id.len(), 43);
    }
}
