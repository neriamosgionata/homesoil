use crate::CoAPClient;
use coap_lite::CoapResponse;
use log::{debug, error};
use std::io;

/// Send a CoAP GET request to a device.
///
/// This is the single point of entry for all outbound CoAP GET requests.
pub fn coap_get(ip: &str, port: i16) -> io::Result<CoapResponse> {
    let address = format!("coap://{}:{}", ip, port);
    debug!("CoAP GET -> {}", address);

    CoAPClient::get(&address).map_err(|e| {
        error!("CoAP GET to {} failed: {:?}", address, e);
        e
    })
}

/// Send a CoAP GET request with a specific path.
pub fn coap_get_path(ip: &str, port: i16, path: &str) -> io::Result<CoapResponse> {
    let address = format!("coap://{}:{}/{}", ip, port, path.trim_start_matches('/'));
    debug!("CoAP GET -> {}", address);

    CoAPClient::get(&address).map_err(|e| {
        error!("CoAP GET to {} failed: {:?}", address, e);
        e
    })
}

/// Send a CoAP POST request to a device with a payload.
///
/// This is the single point of entry for all outbound CoAP POST requests.
pub fn coap_post(ip: &str, port: i16, payload: Vec<u8>) -> io::Result<CoapResponse> {
    let address = format!("coap://{}:{}", ip, port);
    debug!("CoAP POST -> {} ({} bytes)", address, payload.len());

    CoAPClient::post(&address, payload).map_err(|e| {
        error!("CoAP POST to {} failed: {:?}", address, e);
        e
    })
}

/// Send a CoAP POST to a specific path on a device.
pub fn coap_post_path(
    ip: &str,
    port: i16,
    path: &str,
    payload: Vec<u8>,
) -> io::Result<CoapResponse> {
    let address = format!("coap://{}:{}/{}", ip, port, path.trim_start_matches('/'));
    debug!("CoAP POST -> {} ({} bytes)", address, payload.len());

    CoAPClient::post(&address, payload).map_err(|e| {
        error!("CoAP POST to {} failed: {:?}", address, e);
        e
    })
}

/// Send a command to an actuator (ON, OFF, ON-PULSE) and return the response payload string.
pub fn send_actuator_command(ip: &str, port: i16, command: &[u8]) -> io::Result<String> {
    let response = coap_post(ip, port, command.to_vec())?;
    String::from_utf8(response.message.payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// Ping a device (GET) and return the response payload string.
pub fn ping_device(ip: &str, port: i16) -> io::Result<String> {
    let response = coap_get(ip, port)?;
    String::from_utf8(response.message.payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
