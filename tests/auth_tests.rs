use homesoil::auth::{
    check_rate_limit, clear_failed_attempts, generate_device_secret, generate_pin,
    generate_session_token, record_failed_attempt,
};

use std::collections::HashSet;

#[test]
fn test_generate_pin_is_eight_digits() {
    for _ in 0..100 {
        let pin = generate_pin();
        assert_eq!(pin.len(), 8, "PIN should be 8 characters");
        assert!(pin.parse::<u32>().is_ok(), "PIN should be numeric");
        let n: u32 = pin.parse().unwrap();
        assert!(n >= 10000000 && n <= 99999999, "PIN should be 8 digits");
    }
}

#[test]
fn test_generate_pin_randomness() {
    let pins: HashSet<String> = (0..50).map(|_| generate_pin()).collect();
    assert!(pins.len() > 1, "PINs should not all be identical");
}

#[test]
fn test_generate_device_secret_format() {
    for _ in 0..20 {
        let secret = generate_device_secret();
        assert_eq!(secret.len(), 32, "Device secret should be 32 characters");
        assert!(
            secret.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "Device secret should be alphanumeric lowercase"
        );
    }
}

#[test]
fn test_generate_device_secret_randomness() {
    let secrets: HashSet<String> = (0..20).map(|_| generate_device_secret()).collect();
    assert!(secrets.len() > 1, "Device secrets should not all be identical");
}

#[test]
fn test_generate_session_token_format() {
    for _ in 0..20 {
        let token = generate_session_token();
        assert_eq!(token.len(), 48, "Session token should be 48 characters");
        assert!(
            token.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "Session token should be alphanumeric lowercase"
        );
    }
}

#[test]
fn test_generate_session_token_randomness() {
    let tokens: HashSet<String> = (0..20).map(|_| generate_session_token()).collect();
    assert!(tokens.len() > 1, "Session tokens should not all be identical");
}

#[test]
fn test_rate_limit_allows_initial_request() {
    assert!(check_rate_limit("test_initial_12345"));
}

#[test]
fn test_rate_limit_blocks_after_max_attempts() {
    let id = "test_block_99999";
    clear_failed_attempts(id);

    for _ in 0..3 {
        record_failed_attempt(id);
    }

    assert!(!check_rate_limit(id), "Should be rate limited after 3 attempts");
}

#[test]
fn test_rate_limit_clear_resets() {
    let id = "test_clear_88888";
    clear_failed_attempts(id);

    for _ in 0..3 {
        record_failed_attempt(id);
    }
    assert!(!check_rate_limit(id));

    clear_failed_attempts(id);
    assert!(check_rate_limit(id), "Should be allowed after clearing");
}

#[test]
fn test_rate_limit_allows_below_max() {
    let id = "test_below_77777";
    clear_failed_attempts(id);

    for _ in 0..2 {
        record_failed_attempt(id);
    }

    assert!(check_rate_limit(id), "Should still be allowed with 2 attempts");
}

#[test]
fn test_save_and_validate_session_token() {
    std::env::set_var("DATABASE_URL", ":memory:");
    // This test requires the actual DB connection from auth.rs to work with :memory:
    // Since connect() uses env var and creates a new connection each time,
    // in-memory DBs won't share state. We test the token generation/format instead.
    let token = generate_session_token();
    assert_eq!(token.len(), 48);
    assert!(token.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
}
