use crate::db::connect;
use crate::schema::session_tokens;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Mutex;

const SESSION_TOKEN_MAX_AGE_DAYS: i64 = 30;
const MAX_PIN_ATTEMPTS: u32 = 5;
const LOCKOUT_SECONDS: i64 = 60;

static FAILED_ATTEMPTS: Mutex<Option<HashMap<String, (u32, chrono::DateTime<Utc>)>>> =
    Mutex::new(None);

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
                let elapsed = Utc::now() - *last_attempt;
                if elapsed < Duration::seconds(LOCKOUT_SECONDS) {
                    return false;
                }
                attempts.remove(identifier);
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

pub fn generate_pin() -> String {
    let pin: u32 = rand::thread_rng().gen_range(100000..999999);
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

pub fn save_session_token(token: &str) -> bool {
    let mut conn = match connect() {
        Ok(c) => c,
        Err(_) => return false,
    };

    diesel::insert_into(session_tokens::table)
        .values(session_tokens::token.eq(token))
        .execute(&mut conn)
        .is_ok()
}

pub fn is_valid_session_token(token: &str) -> bool {
    let mut conn = match connect() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let result = session_tokens::table
        .filter(session_tokens::token.eq(token))
        .select(session_tokens::created_at)
        .first::<chrono::NaiveDateTime>(&mut conn);

    match result {
        Ok(created) => {
            let age = Utc::now().naive_utc() - created;
            if age > Duration::days(SESSION_TOKEN_MAX_AGE_DAYS) {
                // Expired — clean it up
                let _ = diesel::delete(
                    session_tokens::table.filter(session_tokens::token.eq(token)),
                )
                .execute(&mut conn);
                false
            } else {
                true
            }
        }
        Err(_) => false,
    }
}
