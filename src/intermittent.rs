use crate::db::connect;
use crate::events::{ACTUATOR_INTERMITTENT_CHANGE_EVENT, ACTUATOR_STATE_CHANGE_EVENT};
use crate::helper::broadcast_event;
use crate::schema::actuators;
use crate::coap_client;
use diesel::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use log::info;

struct IntermittentHandle {
    handle_id: u64,
    stop: Arc<AtomicBool>,
}

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

static INTERMITTENT_THREADS: LazyLock<Mutex<HashMap<i32, IntermittentHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn emit_intermittent_change(actuator_id: i32, intermittent: bool, on_ms: Option<i32>, off_ms: Option<i32>) {
    let mut payload = json!({
        "actuator_id": actuator_id,
        "intermittent": intermittent,
        "updated_at": chrono::Local::now().naive_local(),
    });

    // Only include timings on start — clients keep their last values on stop
    if let (Some(on), Some(off)) = (on_ms, off_ms) {
        payload["intermittent_on_ms"] = json!(on);
        payload["intermittent_off_ms"] = json!(off);
    }

    broadcast_event(ACTUATOR_INTERMITTENT_CHANGE_EVENT, payload);
}

pub fn start_intermittent(actuator_id: i32, on_ms: i32, off_ms: i32) {
    stop_intermittent(actuator_id);

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let handle_id = NEXT_HANDLE_ID.fetch_add(1, Ordering::Relaxed);

    thread::spawn(move || {
        run_intermittent_loop(actuator_id, on_ms, off_ms, handle_id, stop_clone);
    });

    let mut threads = INTERMITTENT_THREADS.lock().unwrap();
    threads.insert(
        actuator_id,
        IntermittentHandle {
            handle_id,
            stop,
        },
    );

    // Update DB
    let conn = &mut match connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = diesel::update(actuators::table.find(actuator_id))
        .set((
            actuators::intermittent.eq(true),
            actuators::intermittent_on_ms.eq(on_ms),
            actuators::intermittent_off_ms.eq(off_ms),
            actuators::updated_at.eq(chrono::Local::now().naive_local()),
        ))
        .execute(conn);

    emit_intermittent_change(actuator_id, true, Some(on_ms), Some(off_ms));

    info!(
        "Intermittent started for actuator {} (on={}ms, off={}ms)",
        actuator_id, on_ms, off_ms
    );
}

pub fn stop_intermittent(actuator_id: i32) {
    let mut threads = INTERMITTENT_THREADS.lock().unwrap();

    let had_thread = if let Some(handle) = threads.remove(&actuator_id) {
        handle.stop.store(true, Ordering::Relaxed);
        true
    } else {
        false
    };
    drop(threads);

    // Always clear the DB flag — also fixes stale `intermittent=true` after a restart
    let conn = &mut match connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = diesel::update(actuators::table.find(actuator_id))
        .set((
            actuators::intermittent.eq(false),
            actuators::updated_at.eq(chrono::Local::now().naive_local()),
        ))
        .execute(conn);

    // Notify clients regardless of whether a loop thread existed.
    // Omits on/off ms so clients keep their last known timing values.
    emit_intermittent_change(actuator_id, false, None, None);

    if had_thread {
        info!("Intermittent stopped for actuator {}", actuator_id);
    }
}

pub fn is_intermittent_active(actuator_id: i32) -> bool {
    let threads = INTERMITTENT_THREADS.lock().unwrap();
    threads.contains_key(&actuator_id)
}

fn run_intermittent_loop(
    actuator_id: i32,
    on_ms: i32,
    off_ms: i32,
    handle_id: u64,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        // Turn ON
        send_and_update(actuator_id, true);

        if sleep_interruptible(on_ms as u64, &stop) {
            break;
        }

        // Turn OFF
        send_and_update(actuator_id, false);

        if sleep_interruptible(off_ms as u64, &stop) {
            break;
        }
    }

    // Ensure OFF when stopping — unless a newer intermittent cycle replaced us.
    // (The replacement cycle controls the actuator from here on.)
    let replaced = INTERMITTENT_THREADS
        .lock()
        .map(|threads| {
            matches!(threads.get(&actuator_id), Some(h) if h.handle_id != handle_id)
        })
        .unwrap_or(false);

    if !replaced {
        send_and_update(actuator_id, false);
    }
}

fn sleep_interruptible(ms: u64, stop: &AtomicBool) -> bool {
    let step = 50;
    let mut remaining = ms;
    while remaining > 0 && !stop.load(Ordering::Relaxed) {
        let sleep_time = remaining.min(step);
        thread::sleep(Duration::from_millis(sleep_time));
        remaining = remaining.saturating_sub(sleep_time);
    }
    stop.load(Ordering::Relaxed)
}

fn send_and_update(actuator_id: i32, state: bool) {
    let conn = &mut match connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    let actuator: crate::models::Actuator = match actuators::table.find(actuator_id).first(conn) {
        Ok(a) => a,
        Err(_) => return,
    };

    let command: &[u8] = if state { b"ON" } else { b"OFF" };

    if let Ok(payload) = coap_client::send_actuator_command(
        actuator.get_ip_address(),
        actuator.get_port(),
        command,
    ) {
        let new_state = payload == "ON";

        let now = chrono::Local::now().naive_local();
        let _ = diesel::update(actuators::table.find(actuator_id))
            .set((
                actuators::state.eq(new_state),
                actuators::updated_at.eq(now),
            ))
            .execute(conn);

        broadcast_event(
            ACTUATOR_STATE_CHANGE_EVENT,
            json!({
                "actuator_id": actuator_id,
                "actuator_state": new_state,
                "updated_at": now,
            }),
        );
    }
}
