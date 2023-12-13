use std::collections::HashMap;
use std::net::SocketAddr;
use coap_lite::CoapRequest;
use crate::sensors::{sensor_read_handler, sensor_register_handler};

pub fn path_handler(
    request: &CoapRequest<SocketAddr>,
    handler: HashMap<String, fn(&CoapRequest<SocketAddr>) -> String>,
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

pub fn get_handlers() -> HashMap<String, fn(&CoapRequest<SocketAddr>) -> String> {
    let mut handlers = HashMap::new();
    handlers.insert("/sensor/register".to_string(), sensor_register_handler());
    handlers.insert("/sensor".to_string(), sensor_read_handler());
    return handlers;
}