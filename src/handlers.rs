use std::collections::HashMap;
use std::net::SocketAddr;
use coap_lite::CoapRequest;
use futures_util::future::BoxFuture;
use rust_socketio::asynchronous::Client;
use crate::sensor_handlers::{sensor_update_handler, sensor_read_handler, sensor_register_handler};

pub async fn path_handler<'a>(
    socket: &'a Client,
    request: &'a CoapRequest<SocketAddr>,
) -> Option<String> {
    let mut handlers = get_handlers(socket, request).await;

    let path = format!("{}{}", "/", request.get_path());

    println!("Path called: {}", path);

    match handlers.get_mut(&*path) {
        Some(future) => {
            let res = future.await;
            Some(res)
        }
        None => {
            None
        }
    }
}

pub async fn get_handlers<'a>(socket: &'a Client, request: &'a CoapRequest<SocketAddr>) -> HashMap<String, BoxFuture<'a, String>> {
    let mut handlers = HashMap::new();

    handlers.insert("/sensor/register".to_string(), sensor_register_handler(socket, request));
    handlers.insert("/sensor/name".to_string(), sensor_update_handler(socket, request));
    handlers.insert("/sensor".to_string(), sensor_read_handler(socket, request));

    return handlers;
}