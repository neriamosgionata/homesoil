use crate::db::connect;
use crate::schema::session_tokens;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use log::{info, warn};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

const SESSION_TOKEN_MAX_AGE_DAYS: i64 = 30;
const MAX_PIN_ATTEMPTS: u32 = 3;
const BASE_LOCKOUT_SECONDS: i64 = 60;
const PIN_EXPIRY_SECONDS: i64 = 600; // 10 minutes

static FAILED_ATTEMPTS: Mutex<Option<HashMap<String, (u32, chrono::DateTime<Utc>)>>> =
    Mutex::new(None);

static PIN_CREATED_AT: Mutex<Option<chrono::DateTime<Utc>>> = Mutex::new(None);

fn with_attempts<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, (u32, chrono::DateTime<Utc>)>) -> R,
{
    let mut guard = FAILED_ATTEMPTS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

pub fn check_rate_limit(identifier: &str) -> bool {
    with_attempts(|attempts| {
        if let Some((count, last_attempt)) = attempts.get(identifier) {
            if *count >= MAX_PIN_ATTEMPTS {
                // Exponential backoff: 60s, 300s, 3600s
                let multiplier = (*count - MAX_PIN_ATTEMPTS + 1) as i64;
                let lockout = BASE_LOCKOUT_SECONDS * (5_i64.pow(multiplier.min(3) as u32 - 1)).max(1);
                let elapsed = Utc::now() - *last_attempt;
                if elapsed < Duration::seconds(lockout) {
                    warn!(
                        "Rate limit active for {}: {} attempts, lockout {}s",
                        identifier, count, lockout
                    );
                    return false;
                }
                // Lockout expired, but don't clear — keep escalating
            }
        }
        true
    })
}

pub fn record_failed_attempt(identifier: &str) {
    with_attempts(|attempts| {
        let entry = attempts
            .entry(identifier.to_string())
            .or_insert((0, Utc::now()));
        entry.0 += 1;
        entry.1 = Utc::now();
    });
}

pub fn clear_failed_attempts(identifier: &str) {
    with_attempts(|attempts| {
        attempts.remove(identifier);
    });
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
