use std::collections::HashMap;
use std::net::SocketAddr;
use coap_lite::{CoapRequest, RequestType};
use crate::sensors::register_sensor;

pub fn path_handler(
    request: &CoapRequest<SocketAddr>,
    handler: HashMap<String, impl Fn(&CoapRequest<SocketAddr>) -> String>,
) -> Option<String> {
    let path = request.get_path();
    match handler.get(&*path) {
        Some(callback) => {
            Some(callback(request))
        }
        None => {
            println!("No handler for path {}", path);
            None
        }
    }
}

pub fn get_handlers() -> HashMap<String, impl Fn(&CoapRequest<SocketAddr>) -> String> {
    let mut handlers = HashMap::new();
    handlers.insert("/sensor".to_string(), sensor_handlers());
    return handlers;
}

fn sensor_handlers() -> impl Fn(&CoapRequest<SocketAddr>) -> String {
    return |request: &CoapRequest<SocketAddr>| {
        let payload = String::from_utf8(request.message.payload.clone()).unwrap();

        match request.get_method() {
            RequestType::Get => {
                println!("GET request");
                "".to_string()
            }
            RequestType::Post => {
                println!("POST request");

                match register_sensor(payload) {
                    Ok(sensor) => {
                        println!("Registered sensor: {:?}", sensor);

                        format!("Registered sensor: {:?}", sensor)
                    }
                    Err(e) => {
                        println!("Error registering sensor: {:?}", e);
                        "".to_string()
                    }
                }
            }
            RequestType::Put => {
                println!("PUT request");
                "".to_string()
            }
            RequestType::Delete => {
                println!("DELETE request");
                "".to_string()
            }
            _ => {
                println!("Unknown request");
                "".to_string()
            }
        }
    };
}