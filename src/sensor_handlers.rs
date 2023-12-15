use std::net::SocketAddr;
use coap_lite::{CoapRequest, RequestType};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use serde_json::json;
use socketioxide::extract::SocketRef;
use crate::sensor_models::{change_sensor_name, read_sensor, register_sensor};

pub fn sensor_register_handler<'a>(socket: &'a SocketRef, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match register_sensor(payload) {
            Ok(sensor) => {
                println!("Registered sensor: {:?}", sensor);

                match socket.emit(
                    "sensor_register",
                    json!({
                             "sensor_id": sensor.get_id(),
                             "sensor_name": sensor.get_name(),
                             "sensor_ip_address": sensor.get_ip_address(),
                          }),
                ) {
                    Ok(_) => {
                        println!("Sensor name changed event emitted");
                    }
                    Err(e) => {
                        println!("Error emitting sensor name changed event: {:?}", e);
                    }
                }

                sensor.get_id().to_string()
            }
            Err(e) => {
                println!("Error registering sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}

pub fn sensor_read_handler<'a>(socket: &'a SocketRef, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        if request.get_method() != &RequestType::Post {
            println!("Not a POST request");
            return "KO".to_string();
        }

        println!("POST request");

        match read_sensor(payload) {
            Ok(sensor_read) => {
                println!("Sensor read received: {:?}", sensor_read);

                match socket.emit("sensor_read", json!({
                    "sensor_id": sensor_read.get_id(),
                    "sensor_value": sensor_read.get_sensor_value(),
                })) {
                    Ok(_) => {
                        println!("Sensor name changed event emitted");
                    }
                    Err(e) => {
                        println!("Error emitting sensor name changed event: {:?}", e);
                    }
                }

                "OK".to_string()
            }
            Err(e) => {
                println!("Error reading sensor: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}

pub fn sensor_update_handler<'a>(socket: &'a SocketRef, request: &'a CoapRequest<SocketAddr>) -> BoxFuture<'a, String> {
    async move {
        if request.get_method() != &RequestType::Put {
            println!("Not a PUT request");
            return "KO".to_string();
        }

        println!("PUT request");

        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match change_sensor_name(payload) {
            Ok(sensor) => {
                println!("Sensor name changed: {:?}", sensor);

                match socket.emit("sensor_name_changed", json!({
                    "sensor_id": sensor.get_id(),
                    "sensor_name": sensor.get_name(),
                })) {
                    Ok(_) => {
                        println!("Sensor name changed event emitted");
                    }
                    Err(e) => {
                        println!("Error emitting sensor name changed event: {:?}", e);
                    }
                }

                "OK".to_string()
            }
            Err(e) => {
                println!("Error changing sensor name: {:?}", e);
                "KO".to_string()
            }
        }
    }
        .boxed()
}
