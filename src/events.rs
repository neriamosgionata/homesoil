use crate::actuator_methods::{change_actuator_name, unregister_actuator};
use crate::db::connect;
use crate::flow_methods::{delete_flow, save_new_flow, toggle_flow, update_flow};
use crate::intermittent::{start_intermittent, stop_intermittent};
use crate::helper::{broadcast_event, broadcast_message_to_dashboard, send_message_to_dashboard, DashboardMessageType};
use crate::models::{Actuator, GetSensorReadings, ToggleFlow, UpdateActuatorState};
use crate::schema::actuators;
use crate::schema::actuators::{id, state, updated_at};
use crate::script_methods::{delete_script, get_scripts, save_new_script, update_script};
use crate::script_parser::Script;
use crate::sensor_methods::{change_sensor_name, get_sensor_readings, unregister_sensor};
use crate::coap_client;
use diesel::prelude::*;
use diesel::{update, ExpressionMethods};
use log::{error, info, warn};
use serde_json::json;
use socketioxide::extract::{Data, SocketRef};
use std::collections::HashMap as StdHashMap;
use std::sync::{LazyLock, Mutex as StdMutex};
use std::time::Instant;

/// Actuator command rate limiter: prevents rapid-fire toggle/pulse commands.
static ACTUATOR_RATE_LIMITER: LazyLock<StdMutex<StdHashMap<i32, Instant>>> =
    LazyLock::new(|| StdMutex::new(StdHashMap::new()));

/// Check if an actuator command is rate-limited. Returns true if allowed.
fn check_actuator_rate_limit(actuator_id: i32, min_interval_ms: u64) -> bool {
    let mut limiter = ACTUATOR_RATE_LIMITER.lock().unwrap();
    let now = Instant::now();

    if let Some(last) = limiter.get(&actuator_id) {
        if now.duration_since(*last).as_millis() < min_interval_ms as u128 {
            return false;
        }
    }

    limiter.insert(actuator_id, now);
    true
}

//GENERIC
pub const MESSAGE_SENT_EVENT: &str = "message-sent";

//SENSORS
pub const ALL_SENSORS_EVENT: &str = "all-sensors";
pub const ALL_LAST_SENSOR_READINGS_EVENT: &str = "all-last-sensors-reads";
pub const GET_SENSOR_READINGS_EVENT: &str = "get-sensor-readings";
pub const ALL_SENSOR_READINGS_EVENT: &str = "all-sensor-reads";

pub const SENSOR_REGISTER_EVENT: &str = "sensor-register";
pub const SENSOR_UNREGISTER_EVENT: &str = "sensor-unregister";
pub const SENSOR_READ_EVENT: &str = "sensor-read";
pub const SENSOR_NAME_CHANGE_EVENT: &str = "sensor-name-change";

pub const SENSOR_CHANGE_ONLINE_EVENT: &str = "sensor-change-online";

pub const RENAME_SENSOR_EVENT: &str = "rename-sensor";

pub const REMOVE_SENSOR_EVENT: &str = "remove-sensor";

//ACTUATORS

pub const ALL_ACTUATORS_EVENT: &str = "all-actuators";

pub const ACTUATOR_REGISTER_EVENT: &str = "actuator-register";
pub const ACTUATOR_UNREGISTER_EVENT: &str = "actuator-unregister";

pub const TOGGLE_ACTUATOR_EVENT: &str = "toggle-actuator";
pub const PULSE_ACTUATOR_EVENT: &str = "pulse-actuator";

pub const ACTUATOR_NAME_CHANGE_EVENT: &str = "actuator-name-change";
pub const ACTUATOR_STATE_CHANGE_EVENT: &str = "actuator-state-change";

pub const INTERMITTENT_ACTUATOR_EVENT: &str = "intermittent-actuator";
pub const STOP_INTERMITTENT_ACTUATOR_EVENT: &str = "stop-intermittent-actuator";

pub const ACTUATOR_CHANGE_ONLINE_EVENT: &str = "actuator-change-online";
pub const ACTUATOR_INTERMITTENT_CHANGE_EVENT: &str = "actuator-intermittent-change";

pub const RENAME_ACTUATOR_EVENT: &str = "rename-actuator";

pub const REMOVE_ACTUATOR_EVENT: &str = "remove-actuator";

//SCRIPTS
pub const GET_ALL_SCRIPTS_EVENT: &str = "get-all-scripts";
pub const RUN_SCRIPT_EVENT: &str = "run-script";
pub const ADD_SCRIPT_EVENT: &str = "add-script";
pub const REMOVE_SCRIPT_EVENT: &str = "remove-script";
pub const MODIFY_SCRIPT_EVENT: &str = "modify-script";
pub const ADD_SCRIPT_SCHEDULE_EVENT: &str = "add-script-schedule";
pub const REMOVE_SCRIPT_SCHEDULE_EVENT: &str = "remove-script-schedule";

pub const ALL_SCRIPTS_EVENT: &str = "all-scripts";
pub const SCRIPT_SAVED_EVENT: &str = "script-saved";
pub const SCRIPT_DELETED_EVENT: &str = "script-deleted";
pub const SCRIPT_MODIFIED_EVENT: &str = "script-modified";
pub const SCRIPT_STATUS_CHANGE_EVENT: &str = "script-status-change";
pub const SCRIPT_SCHEDULE_ADDED_EVENT: &str = "script-schedule-added";
pub const SCRIPT_SCHEDULE_REMOVED_EVENT: &str = "script-schedule-removed";

//FLOWS
pub const ALL_FLOWS_EVENT: &str = "all-flows";
pub const ADD_FLOW_EVENT: &str = "add-flow";
pub const MODIFY_FLOW_EVENT: &str = "modify-flow";
pub const REMOVE_FLOW_EVENT: &str = "remove-flow";
pub const TOGGLE_FLOW_EVENT: &str = "toggle-flow";
pub const FLOW_SAVED_EVENT: &str = "flow-saved";
pub const FLOW_MODIFIED_EVENT: &str = "flow-modified";
pub const FLOW_DELETED_EVENT: &str = "flow-deleted";
pub const FLOW_TOGGLED_EVENT: &str = "flow-toggled";

pub fn register_all_callbacks(socket: &SocketRef) {
    socket.on(
        GET_SENSOR_READINGS_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            let gsr = match serde_json::from_str::<GetSensorReadings>(&payload) {
                Ok(gsr) => gsr,
                Err(e) => {
                    error!("Error parsing sensor readings request: {:?}", e);
                    return;
                }
            };

            match get_sensor_readings(gsr.get_id(), gsr.get_from_date(), gsr.get_to_date()) {
                Ok(sensor_reads) => {
                    let _: Result<(), _> = s.emit(
                        ALL_SENSOR_READINGS_EVENT,
                        json!({
                            "sensor_reads": sensor_reads,
                        }),
                    );
                }
                Err(e) => {
                    match send_message_to_dashboard(
                        &s,
                        format!("Error getting sensor readings: {:?}", e).to_string(),
                        DashboardMessageType::Error,
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error sending message to dashboard: {:?}", e);
                        }
                    };
                }
            }
        },
    );

    socket.on(PULSE_ACTUATOR_EVENT, |s: SocketRef, data: Data<i32>| {
        let actuator_id = data.0;

        // Rate limit: max 1 pulse per 3 seconds
        if !check_actuator_rate_limit(actuator_id, 3000) {
            warn!("Pulse rate limited for actuator {}", actuator_id);
            let _ = send_message_to_dashboard(
                &s,
                format!("Actuator {} command rate limited", actuator_id),
                DashboardMessageType::Warning,
            );
            return;
        }

        // Device round-trip (up to 3s) + 2s self-reset must not block the async runtime
        tokio::task::spawn_blocking(move || {
            let conn = &mut match connect() {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Error connecting to database: {:?}", e);
                    return;
                }
            };

            let actuator = match actuators::table
                .filter(id.eq(actuator_id))
                .get_result::<Actuator>(conn)
            {
                Ok(actuator) => actuator,
                Err(_) => {
                    error!("Error loading actuator");
                    return;
                }
            };

            let payload = match coap_client::send_actuator_command(
                actuator.get_ip_address(),
                actuator.get_port(),
                b"ON-PULSE",
            ) {
                Ok(p) => p,
                Err(_) => {
                    error!("Error changing actuator state");
                    return;
                }
            };

            if payload != "ON-PULSE" {
                error!("Error changing actuator state");
                return;
            }

            let mut uas = UpdateActuatorState::new(actuator_id, true);

            uas.set_updated_at(chrono::Local::now().naive_local());

            let res = update(actuators::table.find(actuator_id))
                .set((
                    updated_at.eq(uas.get_updated_at()),
                    state.eq(uas.get_state()),
                ))
                .execute(conn);

            if res.is_err() {
                error!("Error updating actuator");
                return;
            }

            // Emit the ON phase immediately
            let now = uas.get_updated_at();
            broadcast_event(
                ACTUATOR_STATE_CHANGE_EVENT,
                json!({
                    "actuator_id": actuator_id,
                    "actuator_state": true,
                    "updated_at": now,
                }),
            );

            // Set the actuator back to OFF after the device pulse completes (2s).
            // Runs on a dedicated thread — never sleep on the async runtime.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2000));

                let conn = &mut match connect() {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Error connecting to database: {:?}", e);
                        return;
                    }
                };

                let now = chrono::Local::now().naive_local();
                let res = update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(now), state.eq(false)))
                    .execute(conn);

                if res.is_err() {
                    error!("Error updating actuator after pulse");
                    return;
                }

                broadcast_event(
                    ACTUATOR_STATE_CHANGE_EVENT,
                    json!({
                        "actuator_id": actuator_id,
                        "actuator_state": false,
                        "updated_at": now,
                    }),
                );
            });
        });
    });

    socket.on(TOGGLE_ACTUATOR_EVENT, |s: SocketRef, data: Data<i32>| {
        let actuator_id = data.0;

        // Rate limit: max 1 toggle per second
        if !check_actuator_rate_limit(actuator_id, 1000) {
            warn!("Toggle rate limited for actuator {}", actuator_id);
            let _ = send_message_to_dashboard(
                &s,
                format!("Actuator {} command rate limited", actuator_id),
                DashboardMessageType::Warning,
            );
            return;
        }

        // Stop any active intermittent cycling when manually toggling
        stop_intermittent(actuator_id);

        // Device round-trips (up to 3s each) must not block the async runtime
        tokio::task::spawn_blocking(move || {
            let conn = &mut match connect() {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Error connecting to database: {:?}", e);
                    return;
                }
            };

            let actuator = match actuators::table
                .filter(id.eq(actuator_id))
                .get_result::<Actuator>(conn)
            {
                Ok(actuator) => actuator,
                Err(_) => {
                    error!("Error loading actuator");
                    return;
                }
            };

            let current_state = match coap_client::ping_device(
                actuator.get_ip_address(),
                actuator.get_port(),
            ) {
                Ok(p) => p,
                Err(_) => {
                    error!("Error retrieving actuator state");
                    return;
                }
            };

        let command = if current_state == "ON" || current_state == "ON-PULSE" {
            b"OFF".as_slice()
        } else {
            b"ON".as_slice()
        };

        let payload = match coap_client::send_actuator_command(
            actuator.get_ip_address(),
            actuator.get_port(),
            command,
        ) {
            Ok(p) => p,
            Err(_) => {
                error!("Error changing actuator state");
                return;
            }
        };

        if payload == "ON" || payload == "OFF" || payload == "ON-PULSE" {
            let mut uas = UpdateActuatorState::new(actuator_id, payload.contains("ON"));

            uas.set_updated_at(chrono::Local::now().naive_local());

            let res = update(actuators::table.find(actuator_id))
                .set((
                    updated_at.eq(uas.get_updated_at()),
                    state.eq(uas.get_state()),
                ))
                .execute(conn);

            match res {
                Ok(_) => {}
                Err(_) => {
                    error!("Error updating actuator");
                    return;
                }
            }

            broadcast_event(
                ACTUATOR_STATE_CHANGE_EVENT,
                json!({
                    "actuator_id": actuator.get_id(),
                    "actuator_state": uas.get_state(),
                    "updated_at": uas.get_updated_at()
                }),
            );
        } else {
            error!("Error changing actuator state");
        }
        });
    });

    socket.on(
        INTERMITTENT_ACTUATOR_EVENT,
        |_s: SocketRef, data: Data<String>| {
            let payload = data.0;

            #[derive(serde::Deserialize)]
            struct IntermittentData {
                actuator_id: i32,
                on_ms: i32,
                off_ms: i32,
            }

            let params: IntermittentData = match serde_json::from_str(&payload) {
                Ok(p) => p,
                Err(e) => {
                    error!("Error parsing intermittent data: {:?}", e);
                    return;
                }
            };

            // start_intermittent emits the ACTUATOR_INTERMITTENT_CHANGE event
            start_intermittent(params.actuator_id, params.on_ms, params.off_ms);
        },
    );

    socket.on(
        STOP_INTERMITTENT_ACTUATOR_EVENT,
        |_s: SocketRef, data: Data<i32>| {
            let actuator_id = data.0;

            // stop_intermittent emits the ACTUATOR_INTERMITTENT_CHANGE event
            stop_intermittent(actuator_id);
        },
    );

    socket.on(RENAME_SENSOR_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        let sensor = change_sensor_name(payload);

        match sensor {
            Ok(sensor) => {
                match s.emit(
                    SENSOR_NAME_CHANGE_EVENT,
                    json!({
                            "sensor_id": sensor.get_id(),
                            "sensor_name": sensor.get_name(),
                            "updated_at": sensor.get_updated_at(),
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting sensor name change event: {:?}", e);
                    }
                }

                match s.broadcast().emit(
                    SENSOR_NAME_CHANGE_EVENT,
                    json!({
                            "sensor_id": sensor.get_id(),
                            "sensor_name": sensor.get_name(),
                            "updated_at": sensor.get_updated_at(),
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting sensor name change event broadcast: {:?}", e);
                    }
                }
            }
            Err(e) => {
                match send_message_to_dashboard(
                    &s,
                    format!("Error renaming sensor: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error sending message to dashboard: {:?}", e);
                    }
                };
            }
        }
    });

    socket.on(RENAME_ACTUATOR_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        let actuator = change_actuator_name(payload);

        match actuator {
            Ok(actuator) => {
                match s.emit(
                    ACTUATOR_NAME_CHANGE_EVENT,
                    json!({
                            "actuator_id": actuator.get_id(),
                            "actuator_name": actuator.get_name(),
                            "updated_at": actuator.get_updated_at(),
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting actuator name change event: {:?}", e);
                    }
                }

                match s.broadcast().emit(
                    ACTUATOR_NAME_CHANGE_EVENT,
                    json!({
                            "actuator_id": actuator.get_id(),
                            "actuator_name": actuator.get_name(),
                            "updated_at": actuator.get_updated_at(),
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        info!(
                            "Error emitting actuator name change event broadcast: {:?}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                match send_message_to_dashboard(
                    &s,
                    format!("Error renaming actuator: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error sending message to dashboard: {:?}", e);
                    }
                };
            }
        }
    });

    socket.on(REMOVE_ACTUATOR_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match unregister_actuator(payload) {
            Ok(actuator) => {
                let _ = s.emit(
                    ACTUATOR_UNREGISTER_EVENT,
                    json!({ "actuator_id": actuator.get_id() }),
                );
                let _ = s.broadcast().emit(
                    ACTUATOR_UNREGISTER_EVENT,
                    json!({ "actuator_id": actuator.get_id() }),
                );
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error unregistering actuator: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });

    socket.on(REMOVE_SENSOR_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match unregister_sensor(payload) {
            Ok(sensor) => {
                let _ = s.emit(
                    SENSOR_UNREGISTER_EVENT,
                    json!({ "sensor_id": sensor.get_id() }),
                );
                let _ = s.broadcast().emit(
                    SENSOR_UNREGISTER_EVENT,
                    json!({ "sensor_id": sensor.get_id() }),
                );
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error unregistering sensor: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });

    socket.on(GET_ALL_SCRIPTS_EVENT, |s: SocketRef| {
        let all_scripts = get_scripts();

        if let Ok(scripts) = all_scripts {
            let _: Result<(), _> = s.emit(
                ALL_SCRIPTS_EVENT,
                json!({
                       "scripts_array": scripts,
                }),
            );
        }
    });

    socket.on(RUN_SCRIPT_EVENT, |s: SocketRef, data: Data<i32>| {
        let payload = data.0;

        let script = match Script::parse(payload) {
            Ok(script) => script,
            Err(e) => {
                let _: Result<(), _> = send_message_to_dashboard(
                    &s,
                    format!("Error parsing script: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                );
                return;
            }
        };

        let _: Result<(), _> = s.emit(
            SCRIPT_STATUS_CHANGE_EVENT,
            json!({
                   "script_id": script.get_id(),
                   "status": 1,
            }),
        );

        let _: Result<(), _> = s.broadcast().emit(
            SCRIPT_STATUS_CHANGE_EVENT,
            json!({
                   "script_id": script.get_id(),
                   "status": 1,
            }),
        );

        // Scripts can run for up to 5 minutes — never block the async runtime
        let script_id = script.get_id();
        tokio::task::spawn_blocking(move || {
            let res = script.run();

            let status = if res.is_ok() { 0 } else { -2 };

            broadcast_event(
                SCRIPT_STATUS_CHANGE_EVENT,
                json!({
                       "script_id": script_id,
                       "status": status,
                }),
            );

            if let Err(e) = res {
                broadcast_message_to_dashboard(
                    format!("Error running script: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                );
            }
        });
    });

    socket.on(ADD_SCRIPT_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match save_new_script(payload) {
            Ok(script) => {
                match s.emit(
                    SCRIPT_SAVED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting add script event: {:?}", e);
                    }
                }

                match s.broadcast().emit(
                    SCRIPT_SAVED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting add script event broadcast: {:?}", e);
                    }
                }
            }
            Err(e) => {
                match send_message_to_dashboard(
                    &s,
                    format!("Error adding script: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error sending message to dashboard: {:?}", e);
                    }
                };
            }
        }
    });

    socket.on(REMOVE_SCRIPT_EVENT, |s: SocketRef, data: Data<i32>| {
        let payload = data.0;

        match delete_script(payload) {
            Ok(script) => {
                match s.emit(
                    SCRIPT_DELETED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting delete script event: {:?}", e);
                    }
                }

                match s.broadcast().emit(
                    SCRIPT_DELETED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting delete script event broadcast: {:?}", e);
                    }
                }
            }
            Err(e) => {
                match send_message_to_dashboard(
                    &s,
                    format!("Error deleting script: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error sending message to dashboard: {:?}", e);
                    }
                };
            }
        }
    });

    socket.on(MODIFY_SCRIPT_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match update_script(payload) {
            Ok(script) => {
                match s.emit(
                    SCRIPT_MODIFIED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting update script event: {:?}", e);
                    }
                }

                match s.broadcast().emit(
                    SCRIPT_MODIFIED_EVENT,
                    json!({
                        "script": script,
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error emitting update script event broadcast: {:?}", e);
                    }
                }
            }
            Err(e) => {
                match send_message_to_dashboard(
                    &s,
                    format!("Error updating script: {:?}", e).to_string(),
                    DashboardMessageType::Error,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error sending message to dashboard: {:?}", e);
                    }
                };
            }
        }
    });

    socket.on(
        ADD_SCRIPT_SCHEDULE_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            match update_script(payload) {
                Ok(script) => {
                    match s.emit(
                        SCRIPT_SCHEDULE_ADDED_EVENT,
                        json!({
                            "script": script,
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error emitting add script schedule event: {:?}", e);
                        }
                    }

                    match s.broadcast().emit(
                        SCRIPT_SCHEDULE_ADDED_EVENT,
                        json!({
                            "script": script,
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            info!(
                                "Error emitting add script schedule event broadcast: {:?}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    match send_message_to_dashboard(
                        &s,
                        format!("Error updating script: {:?}", e).to_string(),
                        DashboardMessageType::Error,
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error sending message to dashboard: {:?}", e);
                        }
                    };
                }
            }
        },
    );

    socket.on(
        REMOVE_SCRIPT_SCHEDULE_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            match update_script(payload) {
                Ok(script) => {
                    match s.emit(
                        SCRIPT_SCHEDULE_REMOVED_EVENT,
                        json!({
                            "script": script,
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error emitting remove script schedule event: {:?}", e);
                        }
                    }

                    match s.broadcast().emit(
                        SCRIPT_SCHEDULE_REMOVED_EVENT,
                        json!({
                            "script": script,
                        }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            info!(
                                "Error emitting remove script schedule event broadcast: {:?}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    match send_message_to_dashboard(
                        &s,
                        format!("Error updating script: {:?}", e).to_string(),
                        DashboardMessageType::Error,
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error sending message to dashboard: {:?}", e);
                        }
                    };
                }
            }
        },
    );

    // FLOW EVENTS

    socket.on(ADD_FLOW_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match save_new_flow(payload) {
            Ok(flow) => {
                let _ = s.emit(FLOW_SAVED_EVENT, json!({ "flow": flow }));
                let _ = s.broadcast().emit(FLOW_SAVED_EVENT, json!({ "flow": flow }));
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error adding flow: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });

    socket.on(MODIFY_FLOW_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        match update_flow(payload) {
            Ok(flow) => {
                let _ = s.emit(FLOW_MODIFIED_EVENT, json!({ "flow": flow }));
                let _ = s.broadcast().emit(FLOW_MODIFIED_EVENT, json!({ "flow": flow }));
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error updating flow: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });

    socket.on(REMOVE_FLOW_EVENT, |s: SocketRef, data: Data<i32>| {
        let flow_id = data.0;

        match delete_flow(flow_id) {
            Ok(_) => {
                let _ = s.emit(FLOW_DELETED_EVENT, json!({ "flow_id": flow_id }));
                let _ = s.broadcast().emit(FLOW_DELETED_EVENT, json!({ "flow_id": flow_id }));
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error deleting flow: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });

    socket.on(TOGGLE_FLOW_EVENT, |s: SocketRef, data: Data<String>| {
        let payload = data.0;

        let toggle_data: ToggleFlow = match serde_json::from_str(&payload) {
            Ok(t) => t,
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error parsing toggle flow: {:?}", e),
                    DashboardMessageType::Error,
                );
                return;
            }
        };

        match toggle_flow(toggle_data.get_id(), toggle_data.get_enabled()) {
            Ok(flow) => {
                let _ = s.emit(FLOW_TOGGLED_EVENT, json!({ "flow": flow }));
                let _ = s.broadcast().emit(FLOW_TOGGLED_EVENT, json!({ "flow": flow }));
            }
            Err(e) => {
                let _ = send_message_to_dashboard(
                    &s,
                    format!("Error toggling flow: {:?}", e),
                    DashboardMessageType::Error,
                );
            }
        }
    });
}
