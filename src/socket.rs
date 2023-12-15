use serde_json::json;
use socketioxide::extract::SocketRef;
use crate::sensor_models::get_all_registered_sensors;


const GET_SENSORS_EVENT: &str = "get-sensors";
const ALL_SENSORS_EVENT: &str = "all-sensors";

pub fn register_all_callbacks(socket: &SocketRef) {
    let event_map = vec![
        (GET_SENSORS_EVENT, all_sensors_callback),
    ];

    for (event, callback) in event_map.into_iter() {
        socket.on(event, move |socket| {
            callback(socket);
        });
    }
}

fn all_sensors_callback(socket: SocketRef) {
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
