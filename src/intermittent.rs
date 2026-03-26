use crate::db::connect;
use crate::events::ACTUATOR_STATE_CHANGE_EVENT;
use crate::schema::actuators;
use crate::CoAPClient;
use diesel::prelude::*;
use serde_json::json;
use socketioxide::SocketIo;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

struct IntermittentHandle {
    stop: Arc<AtomicBool>,
}

static INTERMITTENT_THREADS: LazyLock<Mutex<HashMap<i32, IntermittentHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static SOCKET_IO: OnceLock<SocketIo> = OnceLock::new();

pub fn set_socket_io(io: SocketIo) {
    let _ = SOCKET_IO.set(io);
}

fn get_socket_io() -> Option<&'static SocketIo> {
    SOCKET_IO.get()
}

pub fn start_intermittent(actuator_id: i32, on_ms: i32, off_ms: i32) {
    stop_intermittent(actuator_id);

    let socket = match get_socket_io() {
        Some(s) => s.clone(),
        None => {
            println!("Intermittent: SocketIo not initialized");
            return;
        }
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    thread::spawn(move || {
        run_intermittent_loop(actuator_id, on_ms, off_ms, stop_clone, &socket);
    });

    let mut threads = INTERMITTENT_THREADS.lock().unwrap();
    threads.insert(actuator_id, IntermittentHandle { stop });

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

    println!(
        "Intermittent started for actuator {} (on={}ms, off={}ms)",
        actuator_id, on_ms, off_ms
    );
}

pub fn stop_intermittent(actuator_id: i32) {
    let mut threads = INTERMITTENT_THREADS.lock().unwrap();
    if let Some(handle) = threads.remove(&actuator_id) {
        handle.stop.store(true, Ordering::Relaxed);

        // Update DB
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

        println!("Intermittent stopped for actuator {}", actuator_id);
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
    stop: Arc<AtomicBool>,
    socket: &SocketIo,
) {
    while !stop.load(Ordering::Relaxed) {
        // Turn ON
        send_and_update(actuator_id, true, socket);

        if sleep_interruptible(on_ms as u64, &stop) {
            break;
        }

        // Turn OFF
        send_and_update(actuator_id, false, socket);

        if sleep_interruptible(off_ms as u64, &stop) {
            break;
        }
    }

    // Ensure OFF when stopping
    send_and_update(actuator_id, false, socket);
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

fn send_and_update(actuator_id: i32, state: bool, socket: &SocketIo) {
    let conn = &mut match connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    let actuator: crate::models::Actuator = match actuators::table.find(actuator_id).first(conn) {
        Ok(a) => a,
        Err(_) => return,
    };

    let address = format!(
        "coap://{}:{}",
        actuator.get_ip_address(),
        actuator.get_port()
    );

    let command = if state { b"ON".to_vec() } else { b"OFF".to_vec() };

    if let Ok(response) = CoAPClient::post(&address, command) {
        let payload = String::from_utf8(response.message.payload).unwrap_or_default();
        let new_state = payload == "ON";

        let now = chrono::Local::now().naive_local();
        let _ = diesel::update(actuators::table.find(actuator_id))
            .set((
                actuators::state.eq(new_state),
                actuators::updated_at.eq(now),
            ))
            .execute(conn);

        if let Some(ns) = socket.of("/") {
            let _ = ns.emit(
                ACTUATOR_STATE_CHANGE_EVENT,
                json!({
                    "actuator_id": actuator_id,
                    "actuator_state": new_state,
                    "updated_at": now,
                }),
            );
        }
    }
}
