use crate::db::connect;
use crate::schema::session_tokens;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use log::{info, warn};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::{LazyLock, Mutex};

const SESSION_TOKEN_MAX_AGE_DAYS: i64 = 30;
// Global PIN attempt limiting: a fresh socket id is minted per connection, so
// per-connection counters are bypassable. A global window still throttles
// brute-force attempts across reconnects.
const MAX_PIN_ATTEMPTS_PER_MINUTE: u32 = 5;
const PIN_EXPIRY_SECONDS: i64 = 600; // 10 minutes

static FAILED_ATTEMPTS: LazyLock<Mutex<(u32, chrono::DateTime<Utc>)>> =
    LazyLock::new(|| Mutex::new((0, Utc::now())));

static PIN_CREATED_AT: Mutex<Option<chrono::DateTime<Utc>>> = Mutex::new(None);

static CURRENT_PIN: Mutex<Option<String>> = Mutex::new(None);

/// True if PIN pairing attempts are allowed right now.
/// Denies all pairing when more than MAX_PIN_ATTEMPTS_PER_MINUTE have failed
/// within the current 60-second window.
pub fn check_rate_limit() -> bool {
    let mut guard = FAILED_ATTEMPTS.lock().unwrap();
    let (count, window_start) = *guard;

    if Utc::now() - window_start > Duration::seconds(60) {
        *guard = (0, Utc::now());
        return true;
    }

    if count >= MAX_PIN_ATTEMPTS_PER_MINUTE {
        warn!(
            "Rate limit active: {} failed PIN attempts this minute",
            count
        );
        return false;
    }

    true
}

pub fn record_failed_attempt() {
    let mut guard = FAILED_ATTEMPTS.lock().unwrap();
    if Utc::now() - guard.1 > Duration::seconds(60) {
        *guard = (0, Utc::now());
    }
    guard.0 += 1;
}

pub fn clear_failed_attempts() {
    let mut guard = FAILED_ATTEMPTS.lock().unwrap();
    *guard = (0, Utc::now());
}

/// Record when the PIN was generated so we can enforce expiry.
pub fn set_pin_created_at() {
    let mut guard = PIN_CREATED_AT.lock().unwrap();
    *guard = Some(Utc::now());
}

/// Check if the PIN has expired (10 minutes after generation).
pub fn is_pin_expired() -> bool {
    let guard = PIN_CREATED_AT.lock().unwrap();
    match *guard {
        Some(created) => {
            let elapsed = Utc::now() - created;
            elapsed > Duration::seconds(PIN_EXPIRY_SECONDS)
        }
        None => true, // No PIN created — treat as expired
    }
}

/// Current pairing PIN. None until first generation.
pub fn get_current_pin() -> Option<String> {
    CURRENT_PIN.lock().unwrap().clone()
}

/// Generate a new PIN, record its creation time, and publish it for use by the socket server.
/// Returns the new PIN so callers can log it.
pub fn rotate_pin() -> String {
    let pin = generate_pin();
    *CURRENT_PIN.lock().unwrap() = Some(pin.clone());
    set_pin_created_at();
    info!("New pairing PIN generated: {}", pin);
    pin
}

pub fn generate_pin() -> String {
    // 8-digit PIN (10_000_000 to 99_999_999)
    let pin: u32 = rand::thread_rng().gen_range(10_000_000..99_999_999);
    pin.to_string()
}

pub fn generate_device_secret() -> String {
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect()
}

pub fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    (0..48)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect()
}

/// Hash a token with SHA-256 for storage.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Save a session token (hashed) to the database.
pub fn save_session_token(token: &str) -> bool {
    let hashed = hash_token(token);

    let mut conn = match connect() {
        Ok(c) => c,
        Err(_) => return false,
    };

    diesel::insert_into(session_tokens::table)
        .values(session_tokens::token.eq(&hashed))
        .execute(&mut conn)
        .is_ok()
}

/// Validate a session token by hashing and looking up in the database.
pub fn is_valid_session_token(token: &str) -> bool {
    let hashed = hash_token(token);

    let mut conn = match connect() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let result = session_tokens::table
        .filter(session_tokens::token.eq(&hashed))
        .select(session_tokens::created_at)
        .first::<chrono::NaiveDateTime>(&mut conn);

    match result {
        Ok(created) => {
            let age = Utc::now().naive_utc() - created;
            if age > Duration::days(SESSION_TOKEN_MAX_AGE_DAYS) {
                // Expired — clean it up
                let _ = diesel::delete(
                    session_tokens::table.filter(session_tokens::token.eq(&hashed)),
                )
                .execute(&mut conn);
                info!("Expired session token cleaned up");
                false
            } else {
                true
            }
        }
        Err(_) => false,
    }
}

/// Clean up all expired session tokens from the database.
pub fn cleanup_expired_tokens() {
    let mut conn = match connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    let cutoff = (Utc::now() - Duration::days(SESSION_TOKEN_MAX_AGE_DAYS)).naive_utc();

    match diesel::delete(session_tokens::table.filter(session_tokens::created_at.lt(cutoff)))
        .execute(&mut conn)
    {
        Ok(count) if count > 0 => {
            info!("Cleaned up {} expired session tokens", count);
        }
        _ => {}
    }
}
