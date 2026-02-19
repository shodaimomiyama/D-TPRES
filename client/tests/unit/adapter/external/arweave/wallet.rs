use formix::adapter::external::arweave::ArweaveWallet;
use serde_json::json;

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
fn test_wallet_from_jwk_valid() {
    let jwk = create_test_jwk();
    let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
    let address = wallet.address();
    assert!(!address.is_empty());
}

#[test]
fn test_wallet_from_jwk_missing_kty() {
    let jwk = json!({
        "n": "test",
        "e": "AQAB"
    });
    let result = ArweaveWallet::from_jwk(jwk);
    assert!(result.is_err());
}

#[test]
fn test_wallet_from_jwk_invalid_kty() {
    let jwk = json!({
        "kty": "EC",
        "n": "test",
        "e": "AQAB"
    });
    let result = ArweaveWallet::from_jwk(jwk);
    assert!(result.is_err());
}
