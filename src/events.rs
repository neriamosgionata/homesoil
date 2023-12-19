use diesel::{ExpressionMethods, update};
use serde_json::json;
use socketioxide::extract::{Data, SocketRef};
use crate::db::connect;
use crate::models::{Actuator, UpdateActuatorState};
use crate::schema::actuators;
use crate::schema::actuators::{id, state, updated_at};
use crate::sensor_methods::get_sensor_readings;
use diesel::prelude::*;
use crate::CoAPClient;

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


//ACTUATORS

pub const ALL_ACTUATORS_EVENT: &str = "all-actuators";

pub const ACTUATOR_REGISTER_EVENT: &str = "actuator-register";
pub const ACTUATOR_UNREGISTER_EVENT: &str = "actuator-unregister";

pub const TOGGLE_ACTUATOR_EVENT: &str = "toggle-actuator";
pub const PULSE_ACTUATOR_EVENT: &str = "pulse-actuator";

pub const ACTUATOR_NAME_CHANGE_EVENT: &str = "actuator-name-change";
pub const ACTUATOR_STATE_CHANGE_EVENT: &str = "actuator-state-change";

pub const ACTUATOR_CHANGE_ONLINE_EVENT: &str = "actuator-change-online";


pub fn register_all_callbacks(socket: &SocketRef) {
    socket.on(
        GET_SENSOR_READINGS_EVENT,
        |s: SocketRef, data: Data<i32>| {
            let sensor_id = data.0;

            println!("Get sensor readings: {:?}", sensor_id);

            match get_sensor_readings(sensor_id) {
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

            println!("Pulsing actuator {:?}", actuator_id);

            let actuator = actuators::table
                .filter(id.eq(actuator_id))
                .get_result::<Actuator>(conn)
                .expect("Error loading actuator");

            let address = "coap://".to_owned() + actuator.get_ip_address() + ":" + actuator.get_port().to_string().as_str();

            println!("Actuator address: {:?}", address);

            let response_actuator = match CoAPClient::post(&address, b"ON-PULSE".to_vec()) {
                Ok(response) => response,
                Err(_) => {
                    println!("Error changing actuator state");
                    return;
                }
            };

            let payload = String::from_utf8(response_actuator.message.payload.clone()).unwrap();

            println!("Actuator response: {:?}", payload);

            if payload == "ON-PULSE" {
                let mut uas = UpdateActuatorState::new(actuator_id, true);

                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                println!("Actuator new state: {:?}", uas.get_state());

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

                println!("Waiting 2 seconds");

                std::thread::sleep(std::time::Duration::from_millis(2000));

                uas.set_state(false);
                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                println!("Actuator new state: {:?}", uas.get_state());

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

            println!("Changing actuator {:?} state", actuator_id);

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

            println!("Actuator current state: {:?}", payload);

            let b = if payload == "ON" || payload == "ON-PULSE" {
                b"OFF".to_vec()
            } else {
                b"ON".to_vec()
            };

            println!("Actuator address: {:?}", address);

            let response_actuator = match CoAPClient::post(&address, b) {
                Ok(response) => response,
                Err(_) => {
                    println!("Error changing actuator state");
                    return;
                }
            };

            let payload = String::from_utf8(response_actuator.message.payload.clone()).unwrap();

            println!("Actuator response: {:?}", payload);

            if payload == "ON" || payload == "OFF" || payload == "ON-PULSE" {
                let mut uas = UpdateActuatorState::new(actuator_id, if payload.contains("ON") { true } else { false });

                uas.set_updated_at(chrono::Local::now().naive_local());

                update(actuators::table.find(actuator_id))
                    .set((updated_at.eq(uas.get_updated_at()), state.eq(uas.get_state())))
                    .execute(conn)
                    .expect("Error updating actuator");

                println!("Actuator new state: {:?}", uas.get_state());

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
}