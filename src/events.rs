use diesel::{ExpressionMethods, update};
use serde_json::json;
use socketioxide::extract::{Data, SocketRef};
use crate::db::connect;
use crate::models::{Actuator, GetSensorReadings, UpdateActuatorState};
use crate::schema::actuators;
use crate::schema::actuators::{id, state, updated_at};
use crate::sensor_methods::{change_sensor_name, get_sensor_readings, unregister_sensor};
use diesel::prelude::*;
use crate::actuator_methods::{change_actuator_name, unregister_actuator};
use crate::CoAPClient;
use crate::helper::send_message_to_dashboard;
use crate::script_parser::{CommandFunctionResult, Script};

//GENERIC
pub const MESSAGE_SENT_EVENT: &str = "message-sent";
pub const SCRIPT_EVENT: &str = "script";

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

pub const ACTUATOR_CHANGE_ONLINE_EVENT: &str = "actuator-change-online";

pub const RENAME_ACTUATOR_EVENT: &str = "rename-actuator";

pub const REMOVE_ACTUATOR_EVENT: &str = "remove-actuator";


pub fn register_all_callbacks(socket: &SocketRef) {
    socket.on(
        GET_SENSOR_READINGS_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            let gsr = serde_json::from_str::<GetSensorReadings>(&payload).unwrap();

            match get_sensor_readings(gsr.get_id(), gsr.get_from_date(), gsr.get_to_date()) {
                Ok(sensor_reads) => {
                    match s.emit(
                        ALL_SENSOR_READINGS_EVENT,
                        json!({
                            "sensor_reads": sensor_reads,
                        }),
                    ) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        },
    );

    socket.on(
        PULSE_ACTUATOR_EVENT,
        |s: SocketRef, data: Data<i32>| {
            let actuator_id = data.0;

            let conn = &mut connect().unwrap();

            let actuator = actuators::table
                .filter(id.eq(actuator_id))
                .get_result::<Actuator>(conn)
                .expect("Error loading actuator");

            let address = "coap://".to_owned() + actuator.get_ip_address() + ":" + actuator.get_port().to_string().as_str();

            let response_actuator = match CoAPClient::post(&address, b"ON-PULSE".to_vec()) {
                Ok(response) => response,
                Err(_) => {
                    println!("Error changing actuator state");
                    return;
                }
            };

            let payload = String::from_utf8(response_actuator.message.payload.clone()).unwrap();

            if payload == "ON-PULSE" {
                let mut uas = UpdateActuatorState::new(actuator_id, true);

                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                match s.emit(
                    ACTUATOR_STATE_CHANGE_EVENT,
                    json!({
                        "actuator_id": actuator.get_id(),
                        "actuator_state": uas.get_state(),
                        "updated_at": actuator.get_updated_at()
                    }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }

                std::thread::sleep(std::time::Duration::from_millis(2000));

                uas.set_state(false);
                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                match s.emit(
                    ACTUATOR_STATE_CHANGE_EVENT,
                    json!({
                            "actuator_id": actuator.get_id(),
                            "actuator_state": uas.get_state(),
                            "updated_at": actuator.get_updated_at()
                        }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            } else {
                println!("Error changing actuator state");
            }
        },
    );

    socket.on(
        TOGGLE_ACTUATOR_EVENT,
        |s: SocketRef, data: Data<i32>| {
            let actuator_id = data.0;

            let conn = &mut connect().unwrap();

            let actuator = actuators::table
                .filter(id.eq(actuator_id))
                .get_result::<Actuator>(conn)
                .expect("Error loading actuator");

            let address = "coap://".to_owned() + actuator.get_ip_address() + ":" + actuator.get_port().to_string().as_str();

            let response_actuator = match CoAPClient::get(&address) {
                Ok(response) => response,
                Err(_) => {
                    println!("Error retrieving actuator state");
                    return;
                }
            };

            let payload = String::from_utf8(response_actuator.message.payload.clone()).unwrap();

            let b = if payload == "ON" || payload == "ON-PULSE" {
                b"OFF".to_vec()
            } else {
                b"ON".to_vec()
            };

            let response_actuator = match CoAPClient::post(&address, b) {
                Ok(response) => response,
                Err(_) => {
                    println!("Error changing actuator state");
                    return;
                }
            };

            let payload = String::from_utf8(response_actuator.message.payload.clone()).unwrap();

            if payload == "ON" || payload == "OFF" || payload == "ON-PULSE" {
                let mut uas = UpdateActuatorState::new(actuator_id, if payload.contains("ON") { true } else { false });

                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                match s.emit(
                    ACTUATOR_STATE_CHANGE_EVENT,
                    json!({
                        "actuator_id": actuator.get_id(),
                        "actuator_state": uas.get_state(),
                        "updated_at": actuator.get_updated_at()
                    }),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            } else {
                println!("Error changing actuator state");
            }
        },
    );

    socket.on(
        RENAME_SENSOR_EVENT,
        |s: SocketRef, data: Data<String>| {
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
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        },
    );

    socket.on(
        RENAME_ACTUATOR_EVENT,
        |s: SocketRef, data: Data<String>| {
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
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        },
    );

    socket.on(
        REMOVE_ACTUATOR_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            match unregister_actuator(payload) {
                Ok(actuator) => {
                    match s.broadcast().emit(
                        ACTUATOR_UNREGISTER_EVENT,
                        json!({
                                    "actuator_id": actuator.get_id(),
                             }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting actuator unregister event: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error unregistering actuator: {:?}", e);
                }
            }
        },
    );

    socket.on(
        REMOVE_SENSOR_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            match unregister_sensor(payload) {
                Ok(sensor) => {
                    match s.broadcast().emit(
                        SENSOR_UNREGISTER_EVENT,
                        json!({
                                    "sensor_id": sensor.get_id(),
                             }),
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error emitting sensor unregister event: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error unregistering sensor: {:?}", e);
                }
            }
        },
    );

    socket.on(
        SCRIPT_EVENT,
        |s: SocketRef, data: Data<String>| {
            let payload = data.0;

            let script = match Script::parse(payload) {
                Ok(script) => script,
                Err(e) => {
                    match send_message_to_dashboard(&s, format!("Error parsing script: {:?}", e).to_string()) {
                        Ok(_) => {}
                        Err(_) => {}
                    };

                    return;
                }
            };

            let res = script.run(&s);

            match res {
                Ok(res) => {
                    match res {
                        CommandFunctionResult::Error(e) => {
                            match send_message_to_dashboard(&s, format!("Error running script: {:?}", e).to_string()) {
                                Ok(_) => {}
                                Err(_) => {}
                            };
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    match send_message_to_dashboard(&s, format!("Error running script: {:?}", e).to_string()) {
                        Ok(_) => {}
                        Err(_) => {}
                    };
                }
            }
        },
    );
}