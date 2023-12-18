use std::collections::HashMap;
use serde_json::{json, Value};
use socketioxide::extract::{Data, SocketRef};
use crate::sensor_models::{get_all_registered_sensors, get_sensor_readings};

const GET_SENSORS_EVENT: &str = "get-sensors";
const ALL_SENSORS_EVENT: &str = "all-sensors";

const GET_SENSOR_READS_EVENT: &str = "get-sensor-reads";
const SENSOR_READS_EVENT: &str = "sensor-reads";

pub const SENSOR_REGISTER_EVENT: &str = "sensor-register";
pub const SENSOR_UNREGISTER_EVENT: &str = "sensor-unregister";
pub const SENSOR_READ_EVENT: &str = "sensor-read";
pub const SENSOR_NAME_CHANGE_EVENT: &str = "sensor-name-change";

pub fn register_all_callbacks(socket: &SocketRef) {
    let mut event_map: HashMap<String, fn(SocketRef, Data<_>)> = HashMap::new();

    event_map.insert(GET_SENSORS_EVENT.parse().unwrap(), all_sensors_callback);
    event_map.insert(GET_SENSOR_READS_EVENT.parse().unwrap(), sensor_data_callback);

    for (event, callback) in event_map.into_iter() {
        socket.on(event, move |socket, data: Data<Value>| {
            callback(socket, data);
        });
    }
}

fn all_sensors_callback(socket: SocketRef, _data: Data<Value>) {
    match get_all_registered_sensors() {
        Ok(sensors) => {
            match socket.emit(
                ALL_SENSORS_EVENT,
                json!({
                    "sensors": sensors,
                }),
            ) {
                Ok(_) => {
                    println!("All sensors event emitted");
                }
                Err(e) => {
                    println!("Error emitting all sensors event: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("Error getting all sensors: {:?}", e);
        }
    }
}

fn sensor_data_callback(socket: SocketRef, data: Data<Value>) {
    let sensor_id = match data.0.as_i64() {
        Some(id) => id,
        None => {
            println!("Error getting sensor id from data");
            return;
        }
    };

    match get_sensor_readings(sensor_id as i32) {
        Ok(sensor_reads) => {
            match socket.emit(
                SENSOR_READS_EVENT,
                json!({
                    "sensor_reads": sensor_reads,
                }),
            ) {
                Ok(_) => {
                    println!("Sensor reads event emitted");
                }
                Err(e) => {
                    println!("Error emitting sensor reads event: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("Error getting sensor reads: {:?}", e);
        }
    }
}