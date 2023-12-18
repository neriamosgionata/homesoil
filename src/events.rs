use serde_json::json;
use socketioxide::extract::{Data, SocketRef};
use crate::sensor_methods::get_sensor_readings;

pub const ALL_SENSORS_EVENT: &str = "all-sensors";
pub const ALL_LAST_SENSOR_READINGS_EVENT: &str = "all-last-sensors-reads";

pub const GET_SENSOR_READINGS_EVENT: &str = "get-sensor-readings";
pub const ALL_SENSOR_READINGS_EVENT: &str = "all-sensor-reads";

pub const SENSOR_REGISTER_EVENT: &str = "sensor-register";
pub const SENSOR_UNREGISTER_EVENT: &str = "sensor-unregister";
pub const SENSOR_READ_EVENT: &str = "sensor-read";
pub const SENSOR_NAME_CHANGE_EVENT: &str = "sensor-name-change";

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
}